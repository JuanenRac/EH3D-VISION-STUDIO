//! Waiting for the picture to be new. After the light, the exposure or the position changes, the frames that were already made (and the ones a driver keeps in a queue) show the old state. The
//! wait here is by frames and not by seconds: the frames up to the counter at the moment of the change are thrown away, then `capture_settle_frames` more, and the next one is the one that
//! is used. `measure_latency` finds out how many frames pass between a change and the picture that shows it.

use super::{Camera, CapError, Code, Frame};

/// The keys of the configuration that the errors of this module name.
pub const KEYS_NAMED: &[&str] = &["capture_settle_frames", "capture_grab_tries"];

/// How long to wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettleSettings {
    /// Frames after the change that are thrown away besides the old ones.
    pub frames: usize,
    /// The most frames asked for while waiting.
    pub tries: usize,
}

impl Default for SettleSettings {
    fn default() -> Self {
        SettleSettings {
            frames: 3,
            tries: 30,
        }
    }
}

/// Whether the values can be used.
pub fn check(s: &SettleSettings) -> Result<(), CapError> {
    if s.frames > 100 {
        return Err(CapError::with_key(
            Code::NoNewFrames,
            "capture_settle_frames",
            "it must be from 0 to 100 frames",
        ));
    }
    if !(1..=1000).contains(&s.tries) {
        return Err(CapError::with_key(
            Code::NoNewFrames,
            "capture_grab_tries",
            "it must be from 1 to 1000 frames",
        ));
    }
    if s.tries <= s.frames {
        return Err(CapError::with_key(
            Code::NoNewFrames,
            "capture_grab_tries",
            "it must be more than capture_settle_frames, or no frame can be used",
        ));
    }
    Ok(())
}

/// A frame that came after the change.
#[derive(Debug, Clone, PartialEq)]
pub struct Settled {
    pub frame: Frame,
    /// Frames asked for, the damaged ones included.
    pub asked: usize,
}

/// Waits for the first frame that is more than `frames` after the counter of now, which is where the change is taken to have happened. A frame that comes damaged is skipped; a camera that
/// is gone ends the wait with its own error; when too many frames pass the answer is `no_new_frames`.
pub fn settle(cam: &mut dyn Camera, s: &SettleSettings) -> Result<Settled, CapError> {
    check(s)?;
    let mark = cam.frame_count();
    let wanted = mark + s.frames as u64;
    let mut last = None;
    for asked in 1..=s.tries {
        match cam.grab() {
            Ok(frame) if frame.seq > wanted => return Ok(Settled { frame, asked }),
            Ok(_) => {}
            Err(e) if e.code == Code::CameraDisconnected => return Err(e),
            Err(e) => last = Some(e),
        }
    }
    Err(CapError::with_key(
        Code::NoNewFrames,
        "capture_grab_tries",
        match last {
            Some(e) => format!("{} frames asked, the last error was {e}", s.tries),
            None => format!("{} frames asked", s.tries),
        },
    ))
}

/// The frames that pass between a change and the picture that shows it: `trigger` makes the change, then frames are asked for until `shows` says the change is in one; the answer is the
/// number of frames before that one (0 when the first frame already shows it). More than `max` frames without it is `no_new_frames`.
pub fn measure_latency(
    cam: &mut dyn Camera,
    trigger: &mut dyn FnMut(),
    shows: &dyn Fn(&Frame) -> bool,
    max: usize,
) -> Result<usize, CapError> {
    trigger();
    let mut passed = 0;
    for _ in 0..=max {
        match cam.grab() {
            Ok(f) if shows(&f) => return Ok(passed),
            Ok(_) => passed += 1,
            Err(e) if e.code == Code::CameraDisconnected => return Err(e),
            Err(_) => passed += 1,
        }
    }
    Err(CapError::with_key(
        Code::NoNewFrames,
        "capture_grab_tries",
        format!("the change was not seen in {max} frames"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::align::Transform;
    use crate::capture::sim::{Fault, SimBackend, Spec, Window, World};
    use crate::capture::{CameraId, CaptureBackend, FormatDesc, PixelFormat};

    fn home() -> Transform {
        crate::capture::sim::home()
    }

    fn setup(latency: usize) -> (SimBackend, CameraId, crate::capture::sim::Shared) {
        let world = World::board(home()).shared();
        let b = SimBackend::new(5, world.clone());
        let mut spec = Spec::standard(1, "1-1");
        spec.latency_frames = latency;
        let id = spec.id.clone();
        b.add(spec);
        (b, id, world)
    }

    fn open(b: &mut SimBackend, id: &CameraId) -> Box<dyn Camera> {
        b.open(
            id,
            FormatDesc {
                width: 480,
                height: 320,
                fps: 30,
                pixel: PixelFormat::Gray8,
            },
        )
        .unwrap()
    }

    fn column(f: &Frame) -> usize {
        f.luma.data.iter().position(|&v| v < 100).unwrap_or(0) % f.luma.w
    }

    #[test]
    fn the_wait_throws_away_the_old_frames_and_gives_the_first_one_past_the_mark() {
        let (mut b, id, _) = setup(0);
        let mut cam = open(&mut b, &id);
        for _ in 0..4 {
            cam.grab().unwrap();
        }
        let s = settle(
            cam.as_mut(),
            &SettleSettings {
                frames: 3,
                tries: 10,
            },
        )
        .unwrap();
        assert_eq!(s.frame.seq, 8);
        assert_eq!(s.asked, 4);
        let none = settle(
            cam.as_mut(),
            &SettleSettings {
                frames: 0,
                tries: 5,
            },
        )
        .unwrap();
        assert_eq!(none.asked, 1);
    }

    #[test]
    fn with_a_delay_in_the_driver_the_frames_to_wait_are_the_delay() {
        let (mut b, id, world) = setup(2);
        let mut cam = open(&mut b, &id);
        let before = column(&cam.grab().unwrap());
        world.lock().unwrap().pose.dx = 20.0;
        let early = settle(
            cam.as_mut(),
            &SettleSettings {
                frames: 0,
                tries: 5,
            },
        )
        .unwrap();
        assert_eq!(column(&early.frame), before, "one frame is not enough");
        world.lock().unwrap().pose.dx = 40.0;
        let late = settle(
            cam.as_mut(),
            &SettleSettings {
                frames: 2,
                tries: 5,
            },
        )
        .unwrap();
        assert_ne!(column(&late.frame), before);
    }

    #[test]
    fn the_latency_in_frames_is_measured() {
        for latency in [0usize, 1, 3] {
            let (mut b, id, world) = setup(latency);
            let mut cam = open(&mut b, &id);
            let base = column(&cam.grab().unwrap());
            let w = world.clone();
            let mut trigger = move || w.lock().unwrap().pose.dx = 30.0;
            let got =
                measure_latency(cam.as_mut(), &mut trigger, &|f| column(f) != base, 10).unwrap();
            assert_eq!(got, latency);
        }
    }

    #[test]
    fn a_change_that_never_shows_is_no_new_frames_and_names_the_key() {
        let (mut b, id, _) = setup(0);
        let mut cam = open(&mut b, &id);
        let e = measure_latency(cam.as_mut(), &mut || {}, &|_| false, 4).unwrap_err();
        assert_eq!(e.code, Code::NoNewFrames);
        assert_eq!(e.key, Some("capture_grab_tries"));
    }

    #[test]
    fn damaged_frames_are_skipped_and_a_camera_that_is_gone_ends_the_wait() {
        let (mut b, id, _) = setup(0);
        b.inject(
            &id.key(),
            Window {
                from: 1,
                to: 2,
                fault: Fault::Truncated,
            },
        );
        let mut cam = open(&mut b, &id);
        let s = settle(
            cam.as_mut(),
            &SettleSettings {
                frames: 0,
                tries: 5,
            },
        )
        .unwrap();
        assert_eq!((s.frame.seq, s.asked), (3, 3));
        b.unplug(&id.key());
        assert_eq!(
            settle(cam.as_mut(), &SettleSettings::default())
                .unwrap_err()
                .code,
            Code::CameraDisconnected
        );
    }

    #[test]
    fn a_stream_that_is_always_damaged_ends_with_the_last_error() {
        let (mut b, id, _) = setup(0);
        b.inject(
            &id.key(),
            Window {
                from: 1,
                to: 100,
                fault: Fault::BadSize,
            },
        );
        let mut cam = open(&mut b, &id);
        let e = settle(
            cam.as_mut(),
            &SettleSettings {
                frames: 0,
                tries: 4,
            },
        )
        .unwrap_err();
        assert_eq!(e.code, Code::NoNewFrames);
        assert!(e.detail.contains("frame_bad_size"), "{e}");
    }

    #[test]
    fn settings_that_leave_no_frame_to_use_are_refused() {
        for (s, key) in [
            (
                SettleSettings {
                    frames: 101,
                    tries: 500,
                },
                "capture_settle_frames",
            ),
            (
                SettleSettings {
                    frames: 1,
                    tries: 0,
                },
                "capture_grab_tries",
            ),
            (
                SettleSettings {
                    frames: 5,
                    tries: 5,
                },
                "capture_grab_tries",
            ),
            (
                SettleSettings {
                    frames: 1,
                    tries: 1001,
                },
                "capture_grab_tries",
            ),
        ] {
            assert_eq!(check(&s).unwrap_err().key, Some(key));
        }
        assert!(check(&SettleSettings::default()).is_ok());
    }
}
