//! Random input with a fixed seed for the live camera, the readers of frame formats, the calibration file and the alignment of a board. Whatever is put in must never panic, and what comes out must
//! make sense: a reader that says yes gives a picture of the size it says, an identifier that is read is written back the same, a calibration file that is read passes its checks, a camera that
//! fails in a random way gives errors with known codes and a counter that never goes back, and an alignment never moves further than its limits whatever the stage does. The generator reads no
//! clock, so a failure comes back the same way and says its seed and its round.

use eh3d_vision_studio::boardaim::{align_board, allowance, AimSettings, Env, Mode, SimStage};
use eh3d_vision_studio::capture::choice::{parse_format, parse_profile};
use eh3d_vision_studio::capture::health::{HealthSettings, Monitor};
use eh3d_vision_studio::capture::log::{Level, Log};
use eh3d_vision_studio::capture::pixels::{
    gray8_to_luma, jpeg_info, mjpeg_to_luma, rgb24_to_luma, stored_jpeg, yuyv_to_luma, yuyv_to_rgb,
};
use eh3d_vision_studio::capture::sim::{home, Fault, SimBackend, Spec, Window, World};
use eh3d_vision_studio::capture::{CameraId, CaptureBackend, Code, FormatDesc, PixelFormat};
use eh3d_vision_studio::charuco::detect::detect;
use eh3d_vision_studio::charuco::solve::Settings as CalSettings;
use eh3d_vision_studio::charuco::{BoardSpec, Dictionary};
use eh3d_vision_studio::image::Gray;
use eh3d_vision_studio::sim::SimRng;

const SEED: u64 = 0x5EED_0008;

fn bytes(rng: &mut SimRng, max: u64) -> Vec<u8> {
    let n = rng.below(max) as usize;
    (0..n).map(|_| rng.below(256) as u8).collect()
}

/// The valid frame bytes of a format, to be damaged.
fn valid(rng: &mut SimRng, w: usize, h: usize, kind: usize) -> Vec<u8> {
    let g = Gray::new(w, h, (0..w * h).map(|_| rng.below(256) as u8).collect()).unwrap();
    match kind {
        0 => g.data.iter().flat_map(|&y| [y, 128]).collect(),
        1 => g.data.iter().flat_map(|&y| [y, y, y]).collect(),
        2 => g.data,
        _ => stored_jpeg(&g),
    }
}

fn damage(rng: &mut SimRng, mut b: Vec<u8>) -> Vec<u8> {
    for _ in 0..1 + rng.below(4) {
        if b.is_empty() {
            break;
        }
        match rng.below(4) {
            0 => {
                let at = rng.below(b.len() as u64) as usize;
                b[at] = rng.below(256) as u8;
            }
            1 => b.truncate(rng.below(b.len() as u64) as usize),
            2 => {
                let at = rng.below(b.len() as u64 + 1) as usize;
                b.insert(at, rng.below(256) as u8);
            }
            _ => {
                let at = rng.below(b.len() as u64) as usize;
                b.remove(at);
            }
        }
    }
    b
}

#[test]
fn the_readers_of_raw_frames_accept_exactly_the_right_length_and_never_panic() {
    let mut rng = SimRng::new(SEED);
    let (mut ok, mut refused) = (0, 0);
    for round in 0..1500 {
        let (w, h) = (rng.below(40) as usize, rng.below(40) as usize);
        let b = bytes(&mut rng, 4000);
        let wants = [
            (w * h * 2, yuyv_to_luma(&b, w, h).map(|g| (g.w, g.h))),
            (w * h * 3, rgb24_to_luma(&b, w, h).map(|g| (g.w, g.h))),
            (w * h, gray8_to_luma(&b, w, h).map(|g| (g.w, g.h))),
        ];
        for (k, (len, got)) in wants.into_iter().enumerate() {
            let should = b.len() == len && w > 0 && h > 0 && (k != 0 || w % 2 == 0);
            match got {
                Ok(size) => {
                    assert!(
                        should,
                        "seed {SEED:#x} round {round} reader {k}: {w}x{h} accepted {} bytes",
                        b.len()
                    );
                    assert_eq!(size, (w, h));
                    ok += 1;
                }
                Err(e) => {
                    assert!(
                        !should,
                        "seed {SEED:#x} round {round} reader {k}: {w}x{h} refused {} bytes: {e}",
                        b.len()
                    );
                    assert!(matches!(e.code, Code::FrameTruncated | Code::FrameBadSize));
                    refused += 1;
                }
            }
        }
        let _ = yuyv_to_rgb(&b, w, h);
    }
    assert!(refused > 3000, "most random lengths are wrong: {refused}");
    let _ = ok;
}

#[test]
fn damaged_frames_of_every_format_are_read_or_refused_and_a_picture_that_is_read_has_its_size() {
    let mut rng = SimRng::new(SEED + 1);
    let (mut good, mut bad) = (0, 0);
    for round in 0..800 {
        let (w, h) = (2 * (1 + rng.below(12) as usize), 1 + rng.below(20) as usize);
        let kind = rng.below(4) as usize;
        let mut frame = valid(&mut rng, w, h, kind);
        if rng.below(5) != 0 {
            frame = damage(&mut rng, frame);
        }
        let got = match kind {
            0 => yuyv_to_luma(&frame, w, h),
            1 => rgb24_to_luma(&frame, w, h),
            2 => gray8_to_luma(&frame, w, h),
            _ => mjpeg_to_luma(&frame),
        };
        match got {
            Ok(g) => {
                assert_eq!(
                    g.data.len(),
                    g.w * g.h,
                    "seed {:#x} round {round}",
                    SEED + 1
                );
                if kind < 3 {
                    assert_eq!((g.w, g.h), (w, h));
                }
                good += 1;
            }
            Err(e) => {
                assert!(
                    matches!(
                        e.code,
                        Code::FrameTruncated | Code::FrameBadSize | Code::FrameNotDecoded
                    ),
                    "round {round}: {e}"
                );
                bad += 1;
            }
        }
        let _ = jpeg_info(&frame);
    }
    assert!(good > 100 && bad > 100, "{good} read, {bad} refused");
}

#[test]
fn random_bytes_are_never_a_motion_jpeg_picture_of_a_size_that_is_not_there() {
    let mut rng = SimRng::new(SEED + 2);
    for round in 0..2000 {
        let mut b = bytes(&mut rng, 300);
        if rng.below(2) == 0 && b.len() > 4 {
            b[0] = 0xFF;
            b[1] = 0xD8;
        }
        if let Ok(g) = mjpeg_to_luma(&b) {
            assert_eq!(g.data.len(), g.w * g.h, "round {round}");
        }
        if let Ok(info) = jpeg_info(&b) {
            assert!(
                info.width > 0 && info.height > 0 && info.complete,
                "round {round}"
            );
        }
    }
}

#[test]
fn identifiers_formats_and_profiles_made_of_garbage_are_refused_or_written_back_the_same() {
    let mut rng = SimRng::new(SEED + 3);
    let alphabet = b"0123456789abcdefxX@:=. -_ABCDEFauto";
    let mut parsed = 0;
    for round in 0..3000 {
        let n = rng.below(40) as usize;
        let text: String = (0..n)
            .map(|_| alphabet[rng.below(alphabet.len() as u64) as usize] as char)
            .collect();
        if let Ok(id) = CameraId::parse(&text) {
            assert_eq!(
                CameraId::parse(&id.key()).unwrap(),
                id,
                "round {round}: {text:?}"
            );
            parsed += 1;
        }
        let _ = parse_format(&text);
        let _ = parse_profile(&text);
    }
    // identifiers that are valid are made on purpose, too
    for _ in 0..200 {
        let id = CameraId {
            vendor: rng.below(65_536) as u16,
            product: rng.below(65_536) as u16,
            serial: (0..rng.below(8))
                .map(|_| (b'A' + rng.below(26) as u8) as char)
                .collect(),
            port: format!("{}-{}.{}", rng.below(9), rng.below(9), rng.below(9)),
        };
        assert_eq!(CameraId::parse(&id.key()).unwrap(), id);
        parsed += 1;
    }
    assert!(parsed >= 200);
}

#[test]
fn a_calibration_file_with_random_damage_is_never_accepted_unless_it_passes_its_checks() {
    let mut rng = SimRng::new(SEED + 4);
    let spec = BoardSpec::default();
    let mut corners = Vec::new();
    for j in 1..spec.rows {
        for i in 1..spec.cols {
            let mm = spec.corner_mm(i, j);
            corners.push(eh3d_vision_studio::charuco::detect::Corner {
                id: (i, j),
                px: (50.0 + 8.0 * mm.0, 60.0 + 8.0 * mm.1),
                mm,
                quality: 1.0,
            });
        }
    }
    let det = eh3d_vision_studio::charuco::detect::Detection {
        corners,
        markers: vec![],
        marker_rms_px: 0.0,
        square_px: 40.0,
    };
    let settings = CalSettings {
        check_mm: 30.0,
        ..CalSettings::default()
    };
    let cal = eh3d_vision_studio::charuco::solve::calibrate(&det, &spec, &settings).unwrap();
    let text = cal.to_text().into_bytes();
    let mut accepted = 0;
    for round in 0..1500 {
        let bad = damage(&mut rng, text.clone());
        let Ok(s) = String::from_utf8(bad) else {
            continue;
        };
        match eh3d_vision_studio::charuco::solve::Calibration::parse(&s, &settings) {
            Ok(c) => {
                accepted += 1;
                assert!(
                    c.rms_px <= settings.max_residual_px && c.gap <= settings.max_method_gap,
                    "round {round}"
                );
                assert!(c.mm_per_px > 0.0 && c.homography.inverse().is_some());
            }
            Err(e) => assert!(
                matches!(
                    e.code,
                    Code::CalibFileBad
                        | Code::CalibFewCorners
                        | Code::CalibResidual
                        | Code::CalibMethodsDisagree
                        | Code::CalibCheckFailed
                        | Code::CalibUnverified
                ),
                "round {round}: {e}"
            ),
        }
    }
    assert!(
        accepted < 400,
        "most damage is noticed: {accepted} accepted"
    );
}

#[test]
fn the_search_for_the_board_in_random_pictures_never_panics() {
    let mut rng = SimRng::new(SEED + 5);
    let spec = BoardSpec::default();
    let dict = Dictionary::for_board(&spec).unwrap();
    for round in 0..40 {
        let (w, h) = (1 + rng.below(90) as usize, 1 + rng.below(90) as usize);
        let kind = rng.below(3);
        let data: Vec<u8> = (0..w * h)
            .map(|i| match kind {
                0 => rng.below(256) as u8,
                1 => {
                    if (i % w / 7 + i / w / 7) % 2 == 0 {
                        20
                    } else {
                        230
                    }
                }
                _ => 100,
            })
            .collect();
        let img = Gray::new(w, h, data).unwrap();
        let _ = detect(&img, &spec, &dict);
        let _ = round;
    }
}

#[test]
fn a_camera_that_fails_in_random_ways_gives_known_errors_and_a_counter_that_never_goes_back() {
    let mut rng = SimRng::new(SEED + 6);
    for round in 0..10 {
        let world = World::board(home()).shared();
        let mut backend = SimBackend::new(rng.below(1000), world);
        let spec = Spec::standard(1, "1-1");
        let id = spec.id.clone();
        backend.add(spec);
        for _ in 0..rng.below(6) {
            let from = 1 + rng.below(20);
            let fault = match rng.below(6) {
                0 => Fault::Disconnect,
                1 => Fault::Black,
                2 => Fault::Frozen,
                3 => Fault::Truncated,
                4 => Fault::BadSize,
                _ => Fault::Skip(1 + rng.below(5) as u32),
            };
            backend.inject(
                &id.key(),
                Window {
                    from,
                    to: from + rng.below(5),
                    fault,
                },
            );
        }
        let pixel = PixelFormat::ALL[rng.below(4) as usize];
        let mut cam = backend
            .open(
                &id,
                FormatDesc {
                    width: 480,
                    height: 320,
                    fps: 30,
                    pixel,
                },
            )
            .unwrap();
        let mut monitor = Monitor::new(HealthSettings::default()).unwrap();
        let mut last = 0;
        for _ in 0..20 {
            match cam.grab() {
                Ok(f) => {
                    assert!(f.seq > last, "round {round}: {} after {last}", f.seq);
                    last = f.seq;
                    assert_eq!(f.luma.data.len(), 480 * 320);
                    let _ = monitor.observe(&f);
                }
                Err(e) => {
                    assert!(
                        matches!(
                            e.code,
                            Code::FrameTruncated | Code::FrameBadSize | Code::CameraDisconnected
                        ),
                        "round {round}: {e}"
                    );
                    let _ = monitor.observe_error(&e);
                }
            }
            assert!(cam.frame_count() >= last);
        }
    }
}

#[test]
fn an_alignment_never_moves_further_than_its_limits_whatever_the_stage_and_the_board_do() {
    let mut rng = SimRng::new(SEED + 7);
    let (mut aligned, mut stopped) = (0, 0);
    for round in 0..25 {
        let unit = |rng: &mut SimRng| rng.below(2001) as f64 / 1000.0 - 1.0;
        let pose = eh3d_vision_studio::align::Transform {
            dx: 6.0 * unit(&mut rng),
            dy: 6.0 * unit(&mut rng),
            theta_deg: 5.0 * unit(&mut rng),
            scale: 1.0,
            rms_mm: 0.0,
        };
        let world = World::board(pose).shared();
        let mut backend = SimBackend::new(round, world.clone());
        let spec = Spec::standard(1, "1-1");
        let id = spec.id.clone();
        backend.add(spec);
        if rng.below(3) == 0 {
            let from = 2 + rng.below(6);
            backend.inject(
                &id.key(),
                Window {
                    from,
                    to: from + 50,
                    fault: if rng.below(2) == 0 {
                        Fault::Black
                    } else {
                        Fault::Frozen
                    },
                },
            );
        }
        let mut cam = backend
            .open(
                &id,
                FormatDesc {
                    width: 480,
                    height: 320,
                    fps: 30,
                    pixel: PixelFormat::Gray8,
                },
            )
            .unwrap();
        let mut stage = SimStage::new(world);
        stage.accuracy = if rng.below(5) == 0 {
            -0.7
        } else {
            0.6 + 0.4 * (unit(&mut rng) + 1.0) / 2.0
        };
        if rng.below(4) == 0 {
            stage.fail_on = Some(1 + rng.below(3) as usize);
        }
        let aim = AimSettings {
            max_step_mm: 0.5 + 3.0 * (unit(&mut rng) + 1.0) / 2.0,
            max_step_deg: 0.5 + 2.0 * (unit(&mut rng) + 1.0) / 2.0,
            max_total_mm: 3.0 + 10.0 * (unit(&mut rng) + 1.0) / 2.0,
            max_steps: 1 + rng.below(8) as usize,
            ..AimSettings::default()
        };
        let mut env = Env::new(
            vec![(-40.0, -25.0), (40.0, -25.0), (40.0, 25.0), (-40.0, 25.0)],
            0.25,
        );
        env.settle.frames = rng.below(3) as usize;
        let out = align_board(
            cam.as_mut(),
            &mut stage,
            &env,
            &aim,
            &Log::new(Level::Debug),
        );
        let total: f64 = stage.asked.iter().map(|m| m.0.hypot(m.1)).sum();
        assert!(
            total <= aim.max_total_mm + 1e-9,
            "round {round}: {total} of {}",
            aim.max_total_mm
        );
        assert!(stage.asked.len() <= aim.max_steps, "round {round}");
        for (dx, dy, dt) in &stage.asked {
            assert!(
                dx.hypot(*dy) <= aim.max_step_mm + 1e-9 && dt.abs() <= aim.max_step_deg + 1e-9,
                "round {round}"
            );
        }
        // the modes only go up, and a step that was made in a mode that does not move has no move
        let mut mode = Mode::Normal;
        for s in &out.steps {
            assert!(s.mode >= mode, "round {round}: the mode went down");
            mode = s.mode;
            if allowance(s.mode, &aim) == (0.0, 0.0) {
                assert!(s.command.is_none(), "round {round}: a move in {:?}", s.mode);
            }
        }
        match &out.result {
            Ok(()) => {
                aligned += 1;
                let last = out.steps.last().unwrap();
                assert!(
                    last.error_mm <= aim.tolerance_mm && last.error_deg <= aim.tolerance_deg,
                    "round {round}"
                );
            }
            Err(e) => {
                stopped += 1;
                assert!(
                    matches!(
                        e.code,
                        Code::AlignNotConverged
                            | Code::AlignDiverging
                            | Code::AlignTotalLimit
                            | Code::AlignStageError
                            | Code::AlignCameraUnsafe
                            | Code::AlignNoMarks
                    ),
                    "round {round}: {e}"
                );
            }
        }
    }
    assert!(
        aligned >= 3 && stopped >= 3,
        "{aligned} aligned, {stopped} stopped"
    );
}
