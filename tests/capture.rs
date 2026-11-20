//! The live camera chain from end to end on the simulated camera: the cameras are listed and opened from the configuration, a profile is applied, the calibration board is shown to the camera and
//! a calibration is made from what it sees (with the ruler check that refuses a board that was printed at the wrong size), and a board that is off its place is put right by looking, moving and
//! looking again. Everything here is `evidence = simulator`: nothing in it has seen a real camera, lens or board.

use eh3d_vision_studio::align::Transform;
use eh3d_vision_studio::boardaim::{align_board, Env, SimStage};
use eh3d_vision_studio::capture::budget::{check, BudgetSettings};
use eh3d_vision_studio::capture::camid::{locate, Change};
use eh3d_vision_studio::capture::choice::{
    apply_profile, choose_format, parse_format, parse_profile,
};
use eh3d_vision_studio::capture::health::{Cause, Finding, Monitor};
use eh3d_vision_studio::capture::log::{Level, Log};
use eh3d_vision_studio::capture::sim::{home, Fault, Scene, SimBackend, Spec, Window, World};
use eh3d_vision_studio::capture::{
    CameraId, CaptureBackend, Code, FormatDesc, PixelFormat, Property,
};
use eh3d_vision_studio::charuco::detect::detect;
use eh3d_vision_studio::charuco::solve::{calibrate, CameraOffset, Settings};
use eh3d_vision_studio::charuco::store::{load, save};
use eh3d_vision_studio::charuco::{render, view_flat, view_tilted, BoardSpec, Dictionary};
use eh3d_vision_studio::config::Config;
use eh3d_vision_studio::lang::Lang;

fn board_scene(
    px_per_mm: f64,
    deg: f64,
) -> (
    BoardSpec,
    Dictionary,
    Scene,
    eh3d_vision_studio::perspective::Homography,
) {
    let spec = BoardSpec::default();
    let dict = Dictionary::for_board(&spec).unwrap();
    let view = view_flat(&spec, px_per_mm, deg, 320.0, 240.0);
    let img = render(&spec, &dict, &view, 640, 480, 2, 21).unwrap();
    (spec, dict, Scene::Picture(img), view)
}

fn camera_on(
    scene: Scene,
    pixel: PixelFormat,
) -> (
    SimBackend,
    Box<dyn eh3d_vision_studio::capture::Camera>,
    CameraId,
) {
    let world = World::new(scene).shared();
    let mut backend = SimBackend::new(3, world);
    let spec = Spec::standard(1, "1-1");
    let id = spec.id.clone();
    backend.add(spec);
    let cam = backend
        .open(
            &id,
            FormatDesc {
                width: 640,
                height: 480,
                fps: 30,
                pixel,
            },
        )
        .unwrap();
    (backend, cam, id)
}

#[test]
fn the_configuration_chooses_the_camera_the_format_and_the_settings_of_the_camera() {
    let cfg = Config::parse(
        "capture_backend = sim\ncapture_camera = 1234:5678:SIM0001@1-1\ncapture_format = 640x480@30:mjpeg\ncapture_profile = exposure=150 gain=10",
    )
    .unwrap();
    let world = World::board(home()).shared();
    let mut backend = SimBackend::new(1, world);
    backend.add(Spec::standard(2, "1-3"));
    backend.add(Spec::standard(1, "1-1"));
    let listed = backend.enumerate().unwrap();
    let wanted = CameraId::parse(&cfg.capture_camera).unwrap();
    let (at, change) = locate(&wanted, &listed).unwrap();
    assert_eq!(change, Change::Same);
    assert_eq!(listed[at].name, "Simulated camera 1");
    let format = choose_format(&listed[at], parse_format(&cfg.capture_format).unwrap()).unwrap();
    assert_eq!(format.pixel, PixelFormat::Mjpeg);
    let mut cam = backend.open(&listed[at].id, format).unwrap();
    let profile = parse_profile(&cfg.capture_profile).unwrap();
    assert_eq!(apply_profile(cam.as_mut(), &profile).unwrap(), 2);
    assert_eq!(cam.get(Property::Exposure).unwrap().value, 150.0);
    let f = cam.grab().unwrap();
    assert_eq!(f.raw_format, PixelFormat::Mjpeg);
    assert!(
        f.raw.len() > f.luma.data.len(),
        "the original bytes are kept beside the picture"
    );
}

#[test]
fn a_camera_that_was_moved_to_another_port_is_found_and_the_move_is_said() {
    let world = World::board(home()).shared();
    let backend = SimBackend::new(1, world);
    let spec = Spec::standard(1, "1-1");
    let wanted = spec.id.clone();
    backend.add(spec);
    backend.unplug(&wanted.key());
    backend.plug(&wanted.key(), Some("2-4"));
    let mut backend = backend;
    let listed = backend.enumerate().unwrap();
    let (_, change) = locate(&wanted, &listed).unwrap();
    assert_eq!(
        change,
        Change::MovedPort {
            from: "1-1".into(),
            to: "2-4".into()
        }
    );
}

#[test]
fn two_cameras_that_need_more_than_a_bus_can_carry_are_told_so_before_one_goes_black() {
    let id = |port: &str| CameraId {
        vendor: 1,
        product: 2,
        serial: port.into(),
        port: port.into(),
    };
    let yuyv = FormatDesc {
        width: 1920,
        height: 1080,
        fps: 30,
        pixel: PixelFormat::Yuyv,
    };
    let mjpeg = FormatDesc {
        pixel: PixelFormat::Mjpeg,
        ..yuyv
    };
    let s = BudgetSettings::default();
    let e = check(&[(id("1-1"), yuyv), (id("1-2"), yuyv)], &s).unwrap_err();
    assert_eq!(e.code, Code::UsbBandwidth);
    assert!(e.text(Lang::Pt).contains("capture_usb_mbps"));
    assert!(check(&[(id("1-1"), mjpeg), (id("1-2"), mjpeg)], &s).is_ok());
    // the same stream that goes black is blamed on the bus when the bus is shared
    let finding = Finding::Black { run: 4 };
    assert_eq!(
        eh3d_vision_studio::capture::health::probable_cause(&finding, true),
        Cause::UsbBandwidth
    );
}

#[test]
fn a_black_stream_a_frozen_stream_and_lost_frames_are_found_in_the_frames_of_the_simulated_camera()
{
    let world = World::board(home()).shared();
    let mut backend = SimBackend::new(5, world);
    let spec = Spec::standard(1, "1-1");
    let id = spec.id.clone();
    backend.add(spec);
    backend.inject(
        &id.key(),
        Window {
            from: 4,
            to: 5,
            fault: Fault::Black,
        },
    );
    backend.inject(
        &id.key(),
        Window {
            from: 8,
            to: 14,
            fault: Fault::Frozen,
        },
    );
    backend.inject(
        &id.key(),
        Window {
            from: 17,
            to: 17,
            fault: Fault::Skip(3),
        },
    );
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
    let mut m =
        Monitor::new(eh3d_vision_studio::capture::health::HealthSettings::default()).unwrap();
    let mut findings = Vec::new();
    for _ in 0..20 {
        findings.extend(m.observe(&cam.grab().unwrap()));
    }
    assert!(
        findings
            .iter()
            .any(|f| matches!(f, Finding::Black { run: 2 })),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|f| matches!(f, Finding::Frozen { repeats: 5 })),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|f| matches!(f, Finding::Lost { count: 3 })),
        "{findings:?}"
    );
    assert_eq!(m.lost, 3);
}

#[test]
fn the_camera_shows_the_board_and_the_calibration_is_made_from_what_it_sees() {
    for pixel in [PixelFormat::Gray8, PixelFormat::Yuyv, PixelFormat::Mjpeg] {
        let (spec, dict, scene, _) = board_scene(8.0, 13.0);
        let (_b, mut cam, _) = camera_on(scene, pixel);
        let frame = cam.grab().unwrap();
        let det = detect(&frame.luma, &spec, &dict).unwrap();
        let settings = Settings {
            check_mm: 30.0,
            ..Settings::default()
        };
        let cal = calibrate(&det, &spec, &settings).unwrap_or_else(|e| panic!("{pixel:?}: {e}"));
        assert!(
            (cal.mm_per_px - 0.125).abs() < 0.125 * 0.005,
            "{pixel:?}: {}",
            cal.mm_per_px
        );
        assert!(
            cal.rms_px < 0.3 && cal.gap < 0.02,
            "{} {}",
            cal.rms_px,
            cal.gap
        );
        assert!(cal.verified && cal.corners >= 30);
        let cal = cal.with_evidence(cam.evidence()).unwrap();
        assert_eq!(cal.evidence, "simulator");
        assert!(
            cal.check_evidence("camera").is_err(),
            "frames of the simulator are not a real lens"
        );
        assert!((cal.aspect - 1.0).abs() < 0.01);
    }
}

#[test]
fn a_board_printed_two_percent_too_big_is_caught_by_the_ruler_and_not_used() {
    let (spec, dict, scene, _) = board_scene(8.0, 0.0);
    let (_b, mut cam, _) = camera_on(scene, PixelFormat::Gray8);
    let det = detect(&cam.grab().unwrap().luma, &spec, &dict).unwrap();
    // the design says 30 mm between the outer corners of a row; the ruler says 30.6 mm
    let e = calibrate(
        &det,
        &spec,
        &Settings {
            check_mm: 30.6,
            ..Settings::default()
        },
    )
    .unwrap_err();
    assert_eq!(e.code, Code::CalibCheckFailed);
    assert_eq!(e.key, Some("calib_check_tol_mm"));
    // with the true board size written in the configuration the same picture passes
    let big = BoardSpec {
        square_mm: 5.1,
        ..spec
    };
    let det_big = eh3d_vision_studio::charuco::detect::Detection {
        corners: det
            .corners
            .iter()
            .map(|c| eh3d_vision_studio::charuco::detect::Corner {
                mm: big.corner_mm(c.id.0, c.id.1),
                ..*c
            })
            .collect(),
        ..det.clone()
    };
    let cal = calibrate(
        &det_big,
        &big,
        &Settings {
            check_mm: 30.6,
            ..Settings::default()
        },
    )
    .unwrap();
    assert!(
        (cal.mm_per_px - 0.1275).abs() < 0.1275 * 0.005,
        "{}",
        cal.mm_per_px
    );
    // and with no ruler at all the calibration is not used unless the settings allow it
    let e = calibrate(&det, &spec, &Settings::default()).unwrap_err();
    assert_eq!(e.code, Code::CalibUnverified);
}

#[test]
fn a_view_that_is_tilted_too_far_is_refused_by_the_gap_between_the_methods() {
    let spec = BoardSpec::default();
    let dict = Dictionary::for_board(&spec).unwrap();
    let steep = view_tilted(&spec, 8.0, 0.0, 320.0, 240.0, 0.25).unwrap();
    let img = render(&spec, &dict, &steep, 640, 480, 0, 22).unwrap();
    let (_b, mut cam, _) = camera_on(Scene::Picture(img), PixelFormat::Gray8);
    let det = detect(&cam.grab().unwrap().luma, &spec, &dict).unwrap();
    let e = calibrate(
        &det,
        &spec,
        &Settings {
            allow_unchecked: true,
            ..Settings::default()
        },
    )
    .unwrap_err();
    assert_eq!(e.code, Code::CalibMethodsDisagree, "{e}");
    assert_eq!(e.key, Some("calib_max_method_gap"));
}

#[test]
fn a_calibration_is_saved_read_back_and_used_with_the_offset_of_the_configuration_only() {
    let (spec, dict, scene, view) = board_scene(8.0, 5.0);
    let (_b, mut cam, _) = camera_on(scene, PixelFormat::Gray8);
    let det = detect(&cam.grab().unwrap().luma, &spec, &dict).unwrap();
    let settings = Settings {
        check_mm: 30.0,
        ..Settings::default()
    };
    let cal = calibrate(&det, &spec, &settings).unwrap();
    let dir = std::env::temp_dir().join(format!("eh3d_vision_e2e_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("camera.cal");
    save(&path, &cal, false).unwrap();
    let back = load(&path, &settings).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    // a point of the board seen in the picture comes back as the point of the board it is
    let probe_mm = (17.0, 11.0);
    let px = view.apply(probe_mm).unwrap();
    let got = back.to_board(px).unwrap();
    assert!(
        (got.0 - probe_mm.0).abs() < 0.05 && (got.1 - probe_mm.1).abs() < 0.05,
        "{got:?}"
    );
    let machine = back
        .to_machine(
            px,
            &CameraOffset {
                x_mm: 2.0,
                y_mm: -1.0,
                deg: 0.0,
            },
        )
        .unwrap();
    assert!((machine.0 - got.0 - 2.0).abs() < 1e-9 && (machine.1 - got.1 + 1.0).abs() < 1e-9);
}

#[test]
fn a_board_off_its_place_is_put_right_by_looking_moving_and_looking_again() {
    let pose = Transform {
        dx: 1.8,
        dy: -1.1,
        theta_deg: 1.4,
        scale: 1.0,
        rms_mm: 0.0,
    };
    let world = World::board(pose).shared();
    let mut backend = SimBackend::new(8, world.clone());
    let spec = Spec::standard(1, "1-1");
    let id = spec.id.clone();
    backend.add(spec);
    backend.set_latency(&id.key(), 1);
    let mut cam = backend
        .open(
            &id,
            FormatDesc {
                width: 480,
                height: 320,
                fps: 30,
                pixel: PixelFormat::Yuyv,
            },
        )
        .unwrap();
    let mut stage = SimStage::new(world.clone());
    stage.accuracy = 0.8;
    let cfg = Config::parse("align_max_step_mm = 1\nalign_max_steps = 8").unwrap();
    let env = Env::new(
        vec![(-40.0, -25.0), (40.0, -25.0), (40.0, 25.0), (-40.0, 25.0)],
        0.25,
    );
    let log = Log::new(Level::Info);
    let out = align_board(cam.as_mut(), &mut stage, &env, &cfg.aim_settings(), &log);
    assert!(out.result.is_ok(), "{:?}", out.result);
    let p = world.lock().unwrap().pose;
    assert!(p.dx.hypot(p.dy) < 0.06 && p.theta_deg.abs() < 0.12, "{p:?}");
    assert!(out
        .steps
        .iter()
        .all(|s| s.command.is_none_or(|c| c.0.hypot(c.1) <= 1.0 + 1e-9)));
    assert!(out.moved_mm <= cfg.align_max_total_mm);
}

#[test]
fn every_error_code_of_the_chain_is_said_in_the_eight_languages_and_the_texts_differ() {
    for code in Code::ALL {
        let texts: Vec<&str> = Lang::ALL.iter().map(|&l| code.text(l)).collect();
        for (a, ta) in texts.iter().enumerate() {
            assert!(!ta.is_empty());
            for tb in &texts[a + 1..] {
                assert_ne!(ta, tb, "{code:?}");
            }
        }
    }
}

// the command `charuco` of the program

fn cli(args: &[&str]) -> (bool, String, String) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_eh3d-vision"))
        .args(args)
        .env_remove("EH3D_VISION_CONFIG")
        .output()
        .expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d_vision_cli_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_card_command_draws_the_board_in_the_language_asked_and_never_over_a_file() {
    let (ok, svg, _) = cli(&["charuco", "card", "--lang", "es"]);
    assert!(
        ok && svg.starts_with("<svg ") && svg.contains("Imprima"),
        "{svg}"
    );
    let (ok, en, _) = cli(&["charuco", "card", "--lang", "en"]);
    assert!(ok && en.contains("Print at 100 %"));
    let dir = temp_dir("card");
    let file = dir.join("card.svg");
    let file_s = file.to_str().unwrap();
    assert!(cli(&["charuco", "card", "--output", file_s]).0);
    let (again, _, err) = cli(&["charuco", "card", "--output", file_s]);
    assert!(!again && err.contains("file_exists"), "{err}");
    assert!(cli(&["charuco", "card", "--output", file_s, "--replace"]).0);
    let (ok, _, err) = cli(&["charuco", "card", "--lang", "xx"]);
    assert!(!ok && err.contains("--lang"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_calibrate_command_makes_the_file_from_a_picture_and_refuses_what_the_ruler_does_not_support()
{
    let (spec, dict, _, _) = board_scene(8.0, 9.0);
    let view = view_flat(&spec, 8.0, 9.0, 320.0, 240.0);
    let img = render(&spec, &dict, &view, 640, 480, 1, 31).unwrap();
    let dir = temp_dir("cal");
    let pic = dir.join("board.pgm");
    std::fs::write(&pic, eh3d_vision_studio::sim::to_pgm(&img)).unwrap();
    let pic_s = pic.to_str().unwrap();
    let out = dir.join("camera.cal");
    let out_s = out.to_str().unwrap();

    let (ok, text, note) = cli(&[
        "charuco",
        "calibrate",
        pic_s,
        "--check-mm",
        "30",
        "--output",
        out_s,
    ]);
    assert!(ok && text.is_empty(), "{note}");
    assert!(
        note.contains("corners") && note.contains("evidence picture") && note.contains("ruler"),
        "{note}"
    );
    let saved = std::fs::read_to_string(&out).unwrap();
    assert!(
        saved.contains("format = eh3d-vision-calibration/1") && saved.contains("accepted = yes")
    );
    let back = load(
        &out,
        &Settings {
            check_mm: 30.0,
            ..Settings::default()
        },
    )
    .unwrap();
    assert!(
        (back.mm_per_px - 0.125).abs() < 0.0007,
        "{}",
        back.mm_per_px
    );

    // without the file named the calibration goes to the screen
    let (ok, text, _) = cli(&["charuco", "calibrate", pic_s, "--check-mm", "30"]);
    assert!(ok && text.contains("homography ="));
    // the ruler says another distance: refused, and the key to change is named
    let (ok, _, err) = cli(&["charuco", "calibrate", pic_s, "--check-mm", "30.6"]);
    assert!(!ok && err.contains("calib_check_tol_mm"), "{err}");
    // no ruler at all: refused unless it is allowed
    let (ok, _, err) = cli(&["charuco", "calibrate", pic_s]);
    assert!(
        !ok && err.contains("calib_unverified") && err.contains("calib_check_mm"),
        "{err}"
    );
    let (ok, _, err) = cli(&["charuco", "calibrate", pic_s, "--allow-unchecked"]);
    assert!(ok, "{err}");
    // never over a file that is there
    let (ok, _, err) = cli(&[
        "charuco",
        "calibrate",
        pic_s,
        "--check-mm",
        "30",
        "--output",
        out_s,
    ]);
    assert!(!ok && err.contains("file_exists"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_charuco_command_refuses_what_it_does_not_know() {
    for args in [
        vec!["charuco"],
        vec!["charuco", "paint"],
        vec!["charuco", "card", "extra"],
        vec!["charuco", "card", "--colour"],
        vec!["charuco", "calibrate"],
        vec!["charuco", "calibrate", "a.pgm", "b.pgm"],
        vec!["charuco", "calibrate", "missing-file.pgm"],
    ] {
        let (ok, _, err) = cli(&args);
        assert!(!ok && !err.trim().is_empty(), "{args:?}: {err}");
    }
    // a picture without the board
    let dir = temp_dir("none");
    let blank = dir.join("blank.pgm");
    std::fs::write(
        &blank,
        eh3d_vision_studio::sim::to_pgm(
            &eh3d_vision_studio::image::Gray::blank(64, 64, 90).unwrap(),
        ),
    )
    .unwrap();
    let (ok, _, err) = cli(&[
        "charuco",
        "calibrate",
        blank.to_str().unwrap(),
        "--check-mm",
        "30",
    ]);
    assert!(!ok && err.contains("calib_few_corners"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}
