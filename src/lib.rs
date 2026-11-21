// EH3D-VISION-STUDIO - fiducials in a camera image.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
// unsafe code is forbidden, except in the thin backends of live cameras that are built only with their own feature
#![cfg_attr(
    not(any(feature = "v4l2", feature = "mfoundation", feature = "libcamera")),
    forbid(unsafe_code)
)]
#![cfg_attr(
    any(feature = "v4l2", feature = "mfoundation", feature = "libcamera"),
    deny(unsafe_code)
)]
//! Finds round fiducial marks in an image, turns pixels into millimetres and works out how far a board is from where its design puts it. Built with the `gui` feature it also has a window.

/// The reader of PGM and PNG pictures, kept in EH3D-COMMON because the other programs that work from a picture read them the same way.
pub use eh3d_image as image;
pub mod align;
pub mod batch;
pub mod board;
pub mod boardaim;
pub mod browse;
pub mod capture;
pub mod charuco;
pub mod checks;
pub mod config;
pub mod convention;
pub mod correlate;
pub mod cross;
pub mod diff;
pub mod discovery;
pub mod dotcard;
pub mod edge;
pub mod fiducial;
pub mod flatten;
pub mod footprint;
pub mod grid;
#[cfg(feature = "gui")]
pub mod gui;
pub mod histogram;
pub mod inspect;
pub mod job;
pub mod lang;
pub mod lens;
pub mod light;
pub mod limit;
pub mod measure;
pub mod mjpeg;
pub mod offset;
pub mod orientation;
pub mod pads;
pub mod panels;
pub mod perspective;
pub mod points;
pub mod quality;
pub mod readings;
pub mod reference;
pub mod report;
pub mod ring;
pub mod scale;
pub mod server;
pub mod sharpmap;
pub mod shot;
pub mod shots;
pub mod square;
pub mod station;
pub mod tape;
pub mod tools;
pub mod train;
pub mod turnpair;
pub mod viewport;
pub mod window;

/// The simulation of a camera over a board: a host twin that draws the fiducial marks of a design where a known turn and shift put them, and fails on purpose (noise, a frame
/// too dark or too bright, a dead sensor, a flat frame, a speck of dust that looks like a mark, a frame that never arrives, a picture file that is cut), with the glue that
/// finds the marks, pairs them with the design and fits the transform, and a retry. The truth is known, so the answer can be compared with it. Nothing here has looked at a
/// real board or a real lens.
pub mod sim {
    use crate::align::{accept, fit, pair_nearest, Transform};
    use crate::fiducial::{find_with, Settings};
    use crate::image::{from_pgm, Gray};
    use std::collections::VecDeque;

    /// A small deterministic generator (xorshift64*). It reads no clock.
    #[derive(Debug, Clone)]
    pub struct SimRng(u64);

    impl SimRng {
        /// A generator for the seed (zero is replaced, because xorshift cannot start from zero).
        pub fn new(seed: u64) -> SimRng {
            SimRng(if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            })
        }

        /// A number below `n` (`n` must be above zero).
        pub fn below(&mut self, n: u64) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D) % n
        }
    }

    /// What goes wrong with one frame.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Fault {
        /// Nothing: the frame is clean.
        Clean,
        /// Every pixel is off by up to this many levels, either way.
        Noise(u8),
        /// The exposure is too high: the levels are multiplied by three (and clip).
        Bright,
        /// The exposure is too low: the levels are divided by eight.
        Dark,
        /// The sensor is dead: every pixel is 0.
        Black,
        /// The frame is one flat grey.
        Flat,
        /// A speck of dust the size of a mark is on the lens.
        Dust,
        /// The frame never arrives.
        Dropped,
        /// The picture file is cut before its end.
        Truncated,
    }

    /// A camera over a board.
    #[derive(Debug, Clone)]
    pub struct SimCamera {
        rng: SimRng,
        script: VecDeque<Fault>,
        /// Where the board really is: the design moved by this turn and shift (and size).
        pub truth: Transform,
        /// The fiducial marks of the design, in millimetres.
        pub design: Vec<(f64, f64)>,
        /// Millimetres per pixel of the camera.
        pub mm_per_px: f64,
        /// Width of a frame in pixels.
        pub width: usize,
        /// Height of a frame in pixels.
        pub height: usize,
        /// Frames asked for so far.
        pub captured: usize,
    }

    /// The bytes of a binary PGM picture.
    pub fn to_pgm(img: &Gray) -> Vec<u8> {
        let mut out = format!("P5\n{} {}\n255\n", img.w, img.h).into_bytes();
        out.extend_from_slice(&img.data);
        out
    }

    impl SimCamera {
        /// A camera of 480 by 320 pixels at a quarter of a millimetre each, over a design with four marks near the corners of a board 80 by 50 millimetres.
        pub fn new(seed: u64, truth: Transform) -> SimCamera {
            SimCamera {
                rng: SimRng::new(seed),
                script: VecDeque::new(),
                truth,
                design: vec![(-40.0, -25.0), (40.0, -25.0), (40.0, 25.0), (-40.0, 25.0)],
                mm_per_px: 0.25,
                width: 480,
                height: 320,
                captured: 0,
            }
        }

        /// The faults of the next frames, in order; after them every frame is clean.
        pub fn with_script(mut self, faults: &[Fault]) -> SimCamera {
            self.script = faults.iter().copied().collect();
            self
        }

        fn draw(&self) -> Gray {
            let mut frame =
                Gray::blank(self.width, self.height, 220).expect("a frame of a fixed size");
            for &mark in &self.design {
                let (x, y) = self.truth.apply(mark);
                frame.disc(
                    self.width as f64 / 2.0 + x / self.mm_per_px,
                    self.height as f64 / 2.0 + y / self.mm_per_px,
                    10.0,
                    30,
                );
            }
            frame
        }

        /// The next frame, or the reason there is none.
        pub fn capture(&mut self) -> Result<Gray, String> {
            self.captured += 1;
            let fault = self.script.pop_front().unwrap_or(Fault::Clean);
            let mut frame = self.draw();
            match fault {
                Fault::Clean => {}
                Fault::Noise(amplitude) => {
                    let span = 2 * u64::from(amplitude) + 1;
                    for v in &mut frame.data {
                        let moved =
                            i32::from(*v) + self.rng.below(span) as i32 - i32::from(amplitude);
                        *v = moved.clamp(0, 255) as u8;
                    }
                }
                Fault::Bright => {
                    for v in &mut frame.data {
                        *v = (u16::from(*v) * 3).min(255) as u8;
                    }
                }
                Fault::Dark => {
                    for v in &mut frame.data {
                        *v /= 8;
                    }
                }
                Fault::Black => frame.data.iter_mut().for_each(|v| *v = 0),
                Fault::Flat => frame.data.iter_mut().for_each(|v| *v = 128),
                Fault::Dust => frame.disc(
                    self.width as f64 / 2.0 + 20.0,
                    self.height as f64 / 2.0 + 10.0,
                    10.0,
                    30,
                ),
                Fault::Dropped => return Err("the camera gave no frame".into()),
                Fault::Truncated => {
                    let bytes = to_pgm(&frame);
                    let cut = 1 + self.rng.below(bytes.len() as u64 - 1) as usize;
                    return from_pgm(&bytes[..cut]);
                }
            }
            Ok(frame)
        }
    }

    /// Where the board is, from one frame (`threshold` is the cut between dark and light, or `None` for the one of Otsu): the marks are found, counted against the design, paired with it and fitted, and the fit is accepted only when it is good.
    pub fn locate(
        frame: &Gray,
        design: &[(f64, f64)],
        mm_per_px: f64,
        settings: &Settings,
        threshold: Option<u8>,
        max_rms_mm: f64,
    ) -> Result<Transform, String> {
        let marks = find_with(frame, settings, threshold)?;
        if marks.len() != design.len() {
            return Err(format!(
                "{} marks found, {} expected",
                marks.len(),
                design.len()
            ));
        }
        let (cx, cy) = (frame.w as f64 / 2.0, frame.h as f64 / 2.0);
        let measured: Vec<(f64, f64)> = marks
            .iter()
            .map(|m| ((m.x - cx) * mm_per_px, (m.y - cy) * mm_per_px))
            .collect();
        let paired = pair_nearest(design, &measured)?;
        let transform = fit(design, &paired, false)?;
        accept(&transform, max_rms_mm, 0.01)?;
        Ok(transform)
    }

    /// Ask the camera up to `tries` times and return the first answer that locates the board, with how many frames it took; the last error when none does.
    pub fn locate_with_retries(
        camera: &mut SimCamera,
        tries: usize,
        settings: &Settings,
        threshold: Option<u8>,
        max_rms_mm: f64,
    ) -> (Result<Transform, String>, usize) {
        let mut last = Err("no frame was asked for".to_string());
        for attempt in 1..=tries {
            last = camera.capture().and_then(|frame| {
                locate(
                    &frame,
                    &camera.design,
                    camera.mm_per_px,
                    settings,
                    threshold,
                    max_rms_mm,
                )
            });
            if last.is_ok() {
                return (last, attempt);
            }
        }
        (last, tries)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const RMS: f64 = 0.2;

        fn truth() -> Transform {
            Transform {
                dx: 3.0,
                dy: -2.0,
                theta_deg: 1.5,
                scale: 1.0,
                rms_mm: 0.0,
            }
        }

        fn close(found: &Transform, truth: &Transform) {
            assert!((found.dx - truth.dx).abs() < 0.2, "{found:?}");
            assert!((found.dy - truth.dy).abs() < 0.2, "{found:?}");
            assert!((found.theta_deg - truth.theta_deg).abs() < 0.3, "{found:?}");
            assert!(found.rms_mm < RMS, "{found:?}");
        }

        fn locate_with(camera: &mut SimCamera, threshold: Option<u8>) -> Result<Transform, String> {
            let frame = camera.capture()?;
            locate(
                &frame,
                &camera.design,
                camera.mm_per_px,
                &Settings::default(),
                threshold,
                RMS,
            )
        }

        fn locate_one(camera: &mut SimCamera) -> Result<Transform, String> {
            locate_with(camera, None)
        }

        #[test]
        fn a_clean_frame_gives_back_the_turn_and_the_shift_that_drew_it() {
            let mut camera = SimCamera::new(1, truth());
            close(&locate_one(&mut camera).unwrap(), &truth());
            let moved = Transform {
                dx: -5.0,
                dy: 4.0,
                theta_deg: -2.5,
                ..truth()
            };
            close(&locate_one(&mut SimCamera::new(1, moved)).unwrap(), &moved);
        }

        #[test]
        fn noise_exposure_and_a_wrong_gain_still_locate_the_board_and_the_same_seed_repeats_exactly(
        ) {
            for fault in [Fault::Noise(10), Fault::Bright, Fault::Dark] {
                for seed in 1..=8 {
                    let run =
                        || locate_one(&mut SimCamera::new(seed, truth()).with_script(&[fault]));
                    let found = run().unwrap_or_else(|e| panic!("{fault:?} seed {seed}: {e}"));
                    close(&found, &truth());
                    assert_eq!(run(), Ok(found), "{fault:?} seed {seed}");
                }
            }
        }

        #[test]
        fn a_dead_flat_or_missing_frame_is_refused_and_a_retry_recovers_once_a_good_frame_comes() {
            for fault in [Fault::Black, Fault::Flat, Fault::Dropped, Fault::Truncated] {
                let mut camera = SimCamera::new(5, truth()).with_script(&[fault]);
                assert!(locate_one(&mut camera).is_err(), "{fault:?}");
                close(&locate_one(&mut camera).unwrap(), &truth());
            }
            let script = [Fault::Dropped, Fault::Truncated, Fault::Black, Fault::Flat];
            let (result, frames) = locate_with_retries(
                &mut SimCamera::new(5, truth()).with_script(&script),
                6,
                &Settings::default(),
                None,
                RMS,
            );
            close(&result.unwrap(), &truth());
            assert_eq!(frames, 5);
            let (gave_up, frames) = locate_with_retries(
                &mut SimCamera::new(5, truth()).with_script(&script),
                3,
                &Settings::default(),
                None,
                RMS,
            );
            assert!(gave_up.is_err());
            assert_eq!(frames, 3);
        }

        #[test]
        fn strong_noise_defeats_the_automatic_threshold_and_a_fixed_one_recovers() {
            for seed in 1..=6 {
                let strong = || SimCamera::new(seed, truth()).with_script(&[Fault::Noise(40)]);
                let error = locate_with(&mut strong(), None).unwrap_err();
                assert!(error.contains("marks found"), "seed {seed}: {error}");
                close(&locate_with(&mut strong(), Some(125)).unwrap(), &truth());
            }
        }

        #[test]
        fn a_speck_of_dust_or_a_mark_out_of_the_frame_is_refused_rather_than_guessed() {
            let mut dusty = SimCamera::new(2, truth()).with_script(&[Fault::Dust]);
            let error = locate_one(&mut dusty).unwrap_err();
            assert!(error.contains("5 marks found"), "{error}");
            let far = Transform {
                dx: 70.0,
                ..truth()
            };
            let error = locate_one(&mut SimCamera::new(2, far)).unwrap_err();
            assert!(error.contains("marks found"), "{error}");
        }

        #[test]
        fn a_picture_file_cut_at_any_place_is_refused_and_the_whole_one_reads_back() {
            let frame = SimCamera::new(1, truth()).capture().unwrap();
            let bytes = to_pgm(&frame);
            assert_eq!(from_pgm(&bytes).unwrap(), frame);
            for cut in (0..bytes.len()).step_by(997) {
                assert!(from_pgm(&bytes[..cut]).is_err(), "cut {cut}");
            }
        }

        #[test]
        fn the_generator_never_stalls_on_a_zero_seed() {
            let mut rng = SimRng::new(0);
            let drawn: Vec<u64> = (0..8).map(|_| rng.below(1_000_000)).collect();
            assert!(drawn.windows(2).any(|w| w[0] != w[1]));
        }
    }
}
