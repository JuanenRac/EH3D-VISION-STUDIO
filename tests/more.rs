//! More checks of the vision: the transforms undo each other, a board that is mirrored or a mark that is mixed up is not taken for a good fit, the marks are found in a noisy picture and
//! the command line says what is wrong with a file of points.

use eh3d_vision_studio::align::{accept, fit, Transform};
use eh3d_vision_studio::fiducial::{find, otsu, Polarity, Settings};
use eh3d_vision_studio::image::Gray;
use eh3d_vision_studio::scale::{fit_k1, undistort, Scale};
use std::process::Command;

fn near(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

/// Numbers that look random and are always the same, from -1 to 1.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
    }
}

fn square() -> Vec<(f64, f64)> {
    vec![(0.0, 0.0), (80.0, 0.0), (80.0, 50.0), (0.0, 50.0)]
}

fn tf(theta_deg: f64, scale: f64, dx: f64, dy: f64) -> Transform {
    Transform {
        dx,
        dy,
        theta_deg,
        scale,
        rms_mm: 0.0,
    }
}

#[test]
fn what_a_transform_does_its_inverse_undoes_for_any_turn() {
    let mut r = Lcg(5);
    for k in 0..200 {
        let t = tf(
            r.next() * 180.0,
            1.0 + r.next() * 0.2,
            r.next() * 500.0,
            r.next() * 500.0,
        );
        let p = (r.next() * 300.0, r.next() * 300.0);
        let back = t.inverse(t.apply(p));
        assert!(
            near(back.0, p.0, 1e-9) && near(back.1, p.1, 1e-9),
            "{k}: {t:?} {p:?} -> {back:?}"
        );
    }
}

#[test]
fn a_fit_recovers_the_turn_in_every_quadrant_including_the_back_of_the_circle() {
    for theta in [
        -179.0, -135.0, -90.0, -45.0, -0.5, 0.0, 0.5, 45.0, 90.0, 135.0, 179.0, 180.0,
    ] {
        let t = tf(theta, 1.0, 12.5, -7.25);
        let design = square();
        let measured: Vec<_> = design.iter().map(|p| t.apply(*p)).collect();
        let got = fit(&design, &measured, false).unwrap();
        // 180 and -180 are the same turn
        let diff = ((got.theta_deg - theta + 540.0).rem_euclid(360.0)) - 180.0;
        assert!(diff.abs() < 1e-9, "{theta}: found {}", got.theta_deg);
        assert!(
            near(got.dx, 12.5, 1e-9) && near(got.dy, -7.25, 1e-9),
            "{theta}: {got:?}"
        );
        assert!(got.rms_mm < 1e-9 && got.scale == 1.0);
    }
}

#[test]
fn a_pure_shift_has_no_turn_and_no_stretch() {
    let design = square();
    let measured: Vec<_> = design.iter().map(|p| (p.0 + 3.0, p.1 - 4.0)).collect();
    let t = fit(&design, &measured, true).unwrap();
    assert!(near(t.theta_deg, 0.0, 1e-9) && near(t.scale, 1.0, 1e-12));
    assert!(near(t.dx, 3.0, 1e-9) && near(t.dy, -4.0, 1e-9));
    assert!(accept(&t, 0.001, 0.0001).is_ok());
}

#[test]
fn a_mirrored_board_does_not_fit_and_is_not_accepted() {
    let design = square();
    let mirrored: Vec<_> = design.iter().map(|p| (-p.0, p.1)).collect();
    let t = fit(&design, &mirrored, false).unwrap();
    assert!(
        t.rms_mm > 10.0,
        "a turn cannot explain a mirror: {}",
        t.rms_mm
    );
    assert!(accept(&t, 0.1, 0.01).is_err());
}

#[test]
fn two_fiducials_swapped_are_caught_by_the_residual() {
    let design = square();
    let mut measured = design.clone();
    measured.swap(0, 2);
    let t = fit(&design, &measured, false).unwrap();
    let e = accept(&t, 0.1, 0.01).unwrap_err();
    assert!(e.contains("not the one that it should be"), "{e}");
}

#[test]
fn the_noise_of_the_camera_shows_up_as_the_residual_and_not_as_a_wrong_pose() {
    let mut r = Lcg(11);
    let design: Vec<_> = (0..12)
        .map(|i| ((i % 4) as f64 * 30.0, (i / 4) as f64 * 25.0))
        .collect();
    let truth = tf(2.0, 1.0, 40.0, 60.0);
    let noise = 0.02;
    let measured: Vec<_> = design
        .iter()
        .map(|p| {
            let q = truth.apply(*p);
            (q.0 + r.next() * noise, q.1 + r.next() * noise)
        })
        .collect();
    let t = fit(&design, &measured, false).unwrap();
    assert!(near(t.theta_deg, 2.0, 0.05), "{}", t.theta_deg);
    assert!(near(t.dx, 40.0, 0.05) && near(t.dy, 60.0, 0.05), "{t:?}");
    assert!(t.rms_mm > 0.0 && t.rms_mm < noise * 1.5, "{}", t.rms_mm);
    assert!(accept(&t, 0.1, 0.01).is_ok());
}

#[test]
fn without_permission_to_stretch_a_stretched_board_leaves_a_residual_and_the_size_stays_one() {
    let design = square();
    let measured: Vec<_> = design.iter().map(|p| (p.0 * 1.02, p.1 * 1.02)).collect();
    let fixed = fit(&design, &measured, false).unwrap();
    assert_eq!(fixed.scale, 1.0);
    assert!(fixed.rms_mm > 0.5);
    let free = fit(&design, &measured, true).unwrap();
    assert!(near(free.scale, 1.02, 1e-12) && free.rms_mm < 1e-9);
    // the stretch is judged by its size, either way
    assert!(accept(&free, 0.1, 0.0201).is_ok());
    assert!(accept(&free, 0.1, 0.0199).is_err());
    let shrunk = fit(
        &design,
        &design
            .iter()
            .map(|p| (p.0 * 0.98, p.1 * 0.98))
            .collect::<Vec<_>>(),
        true,
    )
    .unwrap();
    assert!(accept(&shrunk, 0.1, 0.0199).is_err());
}

#[test]
fn the_limits_of_the_residual_are_inclusive() {
    let t = Transform {
        rms_mm: 0.1,
        ..tf(0.0, 1.0, 0.0, 0.0)
    };
    assert!(accept(&t, 0.1, 0.01).is_ok());
    assert!(accept(&t, 0.0999, 0.01).is_err());
}

#[test]
fn a_fit_refuses_what_cannot_be_measured() {
    let p = [(0.0, 0.0), (1.0, 1.0)];
    assert!(fit(&p, &p[..1], false).is_err());
    assert!(fit(&p[..1], &p[..1], false).is_err());
    assert!(fit(&[], &[], false).is_err());
    let same = [(5.0, 5.0), (5.0, 5.0), (5.0, 5.0)];
    let e = fit(&same, &same, false).unwrap_err();
    assert!(e.contains("all the same point"), "{e}");
    assert!(fit(&p, &[(0.0, f64::NAN), (1.0, 1.0)], false).is_err());
    assert!(fit(&[(f64::INFINITY, 0.0), (1.0, 1.0)], &p, false).is_err());
    // points all on one line still give a turn
    assert!(fit(
        &[(0.0, 0.0), (10.0, 0.0)],
        &[(0.0, 0.0), (0.0, 10.0)],
        false
    )
    .is_ok());
    // a board that shrinks to a point gives no size
    let e = fit(&p, &[(2.0, 2.0), (2.0, 2.0)], true).unwrap_err();
    assert!(e.contains("size of the board"), "{e}");
}

#[test]
fn the_same_points_give_the_same_transform_in_any_order() {
    let design = square();
    let t = tf(7.0, 1.0, 1.0, 2.0);
    let measured: Vec<_> = design.iter().map(|p| t.apply(*p)).collect();
    let a = fit(&design, &measured, false).unwrap();
    let rev_d: Vec<_> = design.iter().rev().copied().collect();
    let rev_m: Vec<_> = measured.iter().rev().copied().collect();
    let b = fit(&rev_d, &rev_m, false).unwrap();
    assert!(
        near(a.theta_deg, b.theta_deg, 1e-9) && near(a.dx, b.dx, 1e-9) && near(a.dy, b.dy, 1e-9)
    );
}

#[test]
fn the_scale_turns_pixels_into_millimetres_with_y_going_up() {
    let s = Scale::from_known_diameter(50.0, 1.0).unwrap();
    assert!(near(s.mm_per_px, 0.02, 1e-15));
    let c = (320.0, 240.0);
    assert_eq!(s.point(320.0, 240.0, c), (0.0, 0.0));
    let (x, y) = s.point(420.0, 140.0, c);
    assert!(
        near(x, 2.0, 1e-12) && near(y, 2.0, 1e-12),
        "up in the picture is up on the machine"
    );
    let (x, y) = s.point(220.0, 340.0, c);
    assert!(near(x, -2.0, 1e-12) && near(y, -2.0, 1e-12));
    for (px, mm) in [
        (0.0, 1.0),
        (-1.0, 1.0),
        (1.0, 0.0),
        (1.0, -1.0),
        (f64::NAN, 1.0),
        (1.0, f64::INFINITY),
    ] {
        assert!(Scale::from_known_diameter(px, mm).is_err(), "{px} {mm}");
    }
}

#[test]
fn a_lens_with_no_distortion_changes_nothing_and_the_centre_never_moves() {
    let c = (320.0, 240.0);
    assert_eq!(undistort((10.0, 20.0), c, 0.0, 400.0), (10.0, 20.0));
    assert_eq!(undistort(c, c, 0.5, 400.0), c);
    // a point on the axis moves along the axis only
    let p = undistort((420.0, 240.0), c, -0.1, 400.0);
    assert_eq!(p.1, 240.0);
    assert!(p.0 < 420.0 && p.0 > 320.0);
}

#[test]
fn the_distortion_fit_refuses_what_makes_no_sense_and_finds_zero_for_a_straight_lens() {
    let c = (0.0, 0.0);
    let pts: Vec<_> = (-2..=2)
        .flat_map(|i| (-2..=2).map(move |j| (i as f64 * 40.0, j as f64 * 30.0)))
        .collect();
    assert!(fit_k1(&pts, &pts[..24], c, 100.0).is_err());
    assert!(fit_k1(&pts[..2], &pts[..2], c, 100.0).is_err());
    assert!(fit_k1(&pts, &pts, c, 0.0).is_err());
    assert!(fit_k1(&pts, &pts, c, f64::NAN).is_err());
    assert!(fit_k1(&[(0.0, 0.0); 4], &[(0.0, 0.0); 4], c, 100.0).is_err());
    let k = fit_k1(&pts, &pts, c, 100.0).unwrap();
    assert!(near(k, 0.0, 1e-12), "{k}");
}

#[test]
fn the_distortion_that_was_added_is_the_one_that_is_found_and_the_correction_undoes_it() {
    let c = (0.0, 0.0);
    let norm = 120.0;
    let ideal: Vec<_> = (-3..=3)
        .flat_map(|i| (-3..=3).map(move |j| (i as f64 * 20.0, j as f64 * 15.0)))
        .collect();
    for k_true in [-0.08, 0.03, 0.1] {
        // what the camera sees is the ideal point pushed the other way, to first order
        let measured: Vec<_> = ideal
            .iter()
            .map(|p| undistort(*p, c, -k_true, norm))
            .collect();
        let k = fit_k1(&measured, &ideal, c, norm).unwrap();
        assert!(near(k, k_true, 0.01), "{k_true}: found {k}");
        // with the k that was found the points get closer to their ideal than they were
        let err = |pts: &[(f64, f64)]| -> f64 {
            pts.iter()
                .zip(&ideal)
                .map(|(a, b)| (a.0 - b.0).hypot(a.1 - b.1))
                .sum::<f64>()
        };
        let fixed: Vec<_> = measured.iter().map(|p| undistort(*p, c, k, norm)).collect();
        assert!(err(&fixed) < err(&measured) * 0.3, "{k_true}");
    }
}

fn disc_image(w: usize, h: usize, bg: u8, marks: &[(f64, f64, f64, u8)]) -> Gray {
    let mut g = Gray::blank(w, h, bg).unwrap();
    for &(x, y, r, v) in marks {
        g.disc(x, y, r, v);
    }
    g
}

#[test]
fn the_threshold_of_a_two_level_picture_separates_them_whatever_the_levels_are() {
    for (a, b) in [(0u8, 255u8), (10, 240), (100, 110), (200, 201), (30, 220)] {
        let img = disc_image(100, 100, b, &[(50.0, 50.0, 20.0, a)]);
        let t = otsu(&img);
        assert!(t >= a && t < b, "levels {a} and {b}: threshold {t}");
    }
}

#[test]
fn the_centre_of_a_mark_is_found_to_a_fraction_of_a_pixel() {
    let mut r = Lcg(21);
    for _ in 0..25 {
        let (cx, cy) = (100.0 + r.next() * 40.0, 100.0 + r.next() * 40.0);
        let img = disc_image(200, 200, 220, &[(cx, cy, 14.0, 30)]);
        let f = find(&img, &Settings::default()).unwrap();
        assert_eq!(f.len(), 1);
        assert!(
            near(f[0].x, cx, 0.35) && near(f[0].y, cy, 0.35),
            "({cx}, {cy}) -> {:?}",
            f[0]
        );
        assert!(near(f[0].diameter, 28.0, 1.0), "{}", f[0].diameter);
        assert!(f[0].score > 0.9 && f[0].score <= 1.0);
    }
}

#[test]
fn a_noisy_picture_still_gives_its_marks_and_nothing_else() {
    let mut r = Lcg(33);
    let mut img = disc_image(
        300,
        200,
        200,
        &[
            (60.0, 50.0, 12.0, 40),
            (240.0, 50.0, 12.0, 40),
            (60.0, 150.0, 12.0, 40),
            (240.0, 150.0, 12.0, 40),
        ],
    );
    for v in img.data.iter_mut() {
        *v = (f64::from(*v) + r.next() * 12.0).clamp(0.0, 255.0) as u8;
    }
    let f = find(&img, &Settings::default()).unwrap();
    assert_eq!(f.len(), 4, "{f:?}");
    for (x, y) in [(60.0, 50.0), (240.0, 50.0), (60.0, 150.0), (240.0, 150.0)] {
        assert!(
            f.iter().any(|m| near(m.x, x, 0.6) && near(m.y, y, 0.6)),
            "{x},{y}"
        );
    }
}

#[test]
fn the_marks_come_best_first_and_the_one_that_is_not_round_is_behind() {
    let mut img = disc_image(300, 120, 220, &[(60.0, 60.0, 14.0, 30)]);
    // an ellipse wider than tall, inside the elongation that is allowed
    for y in 0..120usize {
        for x in 0..300usize {
            let (dx, dy) = ((x as f64 - 200.0) / 15.0, (y as f64 - 60.0) / 14.0);
            if dx * dx + dy * dy <= 1.0 {
                img.data[y * 300 + x] = 30;
            }
        }
    }
    let f = find(&img, &Settings::default()).unwrap();
    assert_eq!(f.len(), 2);
    assert!(f[0].score > f[1].score, "{f:?}");
    assert!(near(f[0].x, 60.0, 0.5), "the disc is first");
}

#[test]
fn the_size_limits_of_a_mark_are_inclusive_and_a_mark_outside_them_is_left_out() {
    let img = disc_image(200, 200, 220, &[(100.0, 100.0, 10.0, 30)]);
    let d = find(&img, &Settings::default()).unwrap()[0].diameter;
    let with = |lo: f64, hi: f64| {
        find(
            &img,
            &Settings {
                min_diameter: lo,
                max_diameter: hi,
                ..Settings::default()
            },
        )
        .unwrap()
        .len()
    };
    assert_eq!(with(d, d), 1);
    assert_eq!(with(d + 0.01, 200.0), 0);
    assert_eq!(with(8.0, d - 0.01), 0);
}

#[test]
fn a_mark_on_any_of_the_four_edges_is_not_trusted() {
    for (x, y) in [(10.0, 100.0), (189.0, 100.0), (100.0, 10.0), (100.0, 189.0)] {
        let img = disc_image(200, 200, 220, &[(x, y, 12.0, 30)]);
        assert!(
            find(&img, &Settings::default()).unwrap().is_empty(),
            "({x}, {y})"
        );
    }
    // one pixel further in, it is found
    let img = disc_image(200, 200, 220, &[(100.0, 14.0, 12.0, 30)]);
    assert_eq!(find(&img, &Settings::default()).unwrap().len(), 1);
}

#[test]
fn light_marks_are_not_dark_marks_and_the_same_picture_gives_opposite_answers() {
    let dark = disc_image(200, 200, 220, &[(100.0, 100.0, 12.0, 30)]);
    let light = Settings {
        polarity: Polarity::Light,
        ..Settings::default()
    };
    // the light parts of a dark-mark picture are the board around it: one big spot that touches the edge
    assert!(find(&dark, &light).unwrap().is_empty());
    let inverted = disc_image(200, 200, 30, &[(100.0, 100.0, 12.0, 220)]);
    assert!(find(&inverted, &Settings::default()).unwrap().is_empty());
    assert_eq!(find(&inverted, &light).unwrap().len(), 1);
}

#[test]
fn every_setting_that_makes_no_sense_is_refused() {
    let img = disc_image(50, 50, 220, &[(25.0, 25.0, 8.0, 30)]);
    let d = Settings::default();
    for s in [
        Settings {
            min_diameter: 1.0,
            ..d
        },
        Settings {
            min_diameter: f64::NAN,
            ..d
        },
        Settings {
            max_diameter: f64::INFINITY,
            ..d
        },
        Settings {
            max_diameter: 4.0,
            ..d
        },
        Settings { min_fill: 0.0, ..d },
        Settings {
            min_fill: 1.01,
            ..d
        },
        Settings {
            max_elongation: 0.99,
            ..d
        },
        Settings {
            max_elongation: f64::INFINITY,
            ..d
        },
    ] {
        assert!(find(&img, &s).is_err(), "{s:?}");
    }
}

#[test]
fn finding_twice_gives_the_same_marks_and_a_shifted_picture_gives_shifted_marks() {
    let a = disc_image(
        200,
        200,
        220,
        &[(70.0, 80.0, 11.0, 30), (130.0, 120.0, 9.0, 30)],
    );
    assert_eq!(
        find(&a, &Settings::default()),
        find(&a, &Settings::default())
    );
    let b = disc_image(
        200,
        200,
        220,
        &[(80.0, 90.0, 11.0, 30), (140.0, 130.0, 9.0, 30)],
    );
    let (fa, fb) = (
        find(&a, &Settings::default()).unwrap(),
        find(&b, &Settings::default()).unwrap(),
    );
    assert_eq!(fa.len(), 2);
    assert_eq!(fb.len(), 2);
    for m in &fa {
        assert!(
            fb.iter().any(|n| near(n.x, m.x + 10.0, 1e-9)
                && near(n.y, m.y + 10.0, 1e-9)
                && near(n.diameter, m.diameter, 1e-9)),
            "{m:?}"
        );
    }
}

// ---- the command line ----

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-vision"))
        .args(args)
        .output()
        .expect("the program starts");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn tmp(name: &str) -> String {
    format!("{}/{name}", env!("CARGO_TARGET_TMPDIR"))
}

fn write(name: &str, text: &str) -> String {
    let p = tmp(name);
    std::fs::write(&p, text).unwrap();
    p
}

#[test]
fn a_file_of_points_may_have_comments_blank_lines_windows_ends_and_any_script() {
    let d = write(
        "more-design.csv",
        "# fiduciales del diseño 日本語\r\n\r\n0,0\r\n  100 , 0 \r\n\r\n100,50\r\n",
    );
    let m = write("more-measured.csv", "10,20\n110,20\n110,70\n");
    let (code, out, e) = run(&["align", &d, &m]);
    assert_eq!(code, Some(0), "{e}");
    assert!(out.starts_with("shift (+10.0000, +20.0000) mm"), "{out}");
    assert!(out.contains("size 1.00000, residual 0.0000 mm"), "{out}");
}

#[test]
fn a_bad_line_names_the_file_and_the_line_and_a_bad_fit_still_prints_what_it_found() {
    let ok = write("more-ok.csv", "0,0\n10,0\n");
    for (text, expect) in [
        ("0,0\n10;0\n", "line 2: it must be x,y"),
        ("0,0\nx,5\n", "is not a number"),
        ("0,0\n1,2,3\n", "is not a number"),
        ("0,0\n1,nan\n", "is not a number"),
        ("0,0\n", "must be the same number"),
        ("", "must be the same number"),
    ] {
        let bad = write("more-bad.csv", text);
        let (code, _, e) = run(&["align", &ok, &bad]);
        assert_eq!(code, Some(1), "{text:?}");
        assert!(e.contains(expect), "{text:?}: {e}");
    }
    let (code, _, e) = run(&["align", &ok, &tmp("nothing-here.csv")]);
    assert_eq!(code, Some(1));
    assert!(e.contains("nothing-here.csv"), "{e}");
    // a board that is 5 mm off: the pose is printed, then the refusal
    let off = write("more-off.csv", "0,0\n15,0\n");
    let (code, out, e) = run(&["align", &ok, &off, "--scale"]);
    assert_eq!(code, Some(1));
    assert!(out.contains("size 1.50000"), "{out}");
    assert!(e.contains("bigger or smaller"), "{e}");
    // without --scale the same points leave a residual
    let (code, _, e) = run(&["align", &ok, &off, "--max-rms", "0.01"]);
    assert_eq!(code, Some(1));
    assert!(e.contains("not the one that it should be"), "{e}");
}

#[test]
fn the_options_of_the_command_line_are_strict() {
    let p = write("more-p.csv", "0,0\n10,0\n");
    let cases: &[(&[&str], &str)] = &[
        (&["align", &p], "needs the design points"),
        (&["align", &p, &p, &p], "unexpected argument"),
        (&["align", &p, &p, "--max-rms"], "--max-rms needs a value"),
        (&["align", &p, &p, "--max-rms", "x"], "is not a number"),
        (&["align", &p, &p, "--scal"], "unknown option"),
        (&["find"], "find needs an image"),
        (&["find", "a", "b"], "unexpected argument"),
        (&["find", &p, "--min-diam", "q"], "is not a number"),
        (&["frobnicate"], "unknown command"),
    ];
    for (args, expect) in cases {
        let (code, _, e) = run(args);
        assert_eq!(code, Some(1), "{args:?}");
        assert!(e.contains(expect), "{args:?}: {e}");
    }
    let (code, out, _) = run(&["--help"]);
    assert_eq!(code, Some(0));
    assert!(out.contains("eh3d-vision find"));
}

#[test]
fn the_find_command_reports_marks_with_their_millimetres_and_an_empty_picture_says_so() {
    let mut b = b"P5\n64 64\n255\n".to_vec();
    let g = disc_image(64, 64, 220, &[(40.0, 24.0, 8.0, 30)]);
    b.extend(&g.data);
    let path = tmp("more-find.pgm");
    std::fs::write(&path, &b).unwrap();
    let (code, out, e) = run(&["find", &path, "--known-mm", "1.6"]);
    assert_eq!(code, Some(0), "{e}");
    assert!(out.starts_with("1: (40.0, 24.0) px, diameter 1"), "{out}");
    // 8 px to the right and 8 px up from the centre (32, 32), at 0.1 mm per pixel
    let mm = out
        .split("->  (")
        .nth(1)
        .unwrap()
        .split(')')
        .next()
        .unwrap();
    let v: Vec<f64> = mm.split(", ").map(|n| n.parse().unwrap()).collect();
    assert!(near(v[0], 0.8, 0.02) && near(v[1], 0.8, 0.02), "{out}");
    let flat = {
        let mut b = b"P5\n8 8\n255\n".to_vec();
        b.extend([100u8; 64]);
        write_bytes("more-flat.pgm", &b)
    };
    let (code, out, _) = run(&["find", &flat]);
    assert_eq!(code, Some(0));
    assert_eq!(out.trim(), "no marks found");
    // a mark that is not that big is not a mark when the limits say so
    let (code, out, _) = run(&["find", &path, "--min-diam", "30"]);
    assert_eq!(code, Some(0));
    assert_eq!(out.trim(), "no marks found");
    let (code, _, e) = run(&["find", &path, "--min-diam", "1"]);
    assert_eq!(code, Some(1));
    assert!(e.contains("make no sense"), "{e}");
    let (code, _, e) = run(&["find", &path, "--known-mm", "0"]);
    assert_eq!(code, Some(1));
    assert!(e.contains("no scale"), "{e}");
}

fn write_bytes(name: &str, b: &[u8]) -> String {
    let p = tmp(name);
    std::fs::write(&p, b).unwrap();
    p
}
