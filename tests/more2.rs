//! More limits of the fit of the board and of the scale: two points, points in a line, a half turn, the messages that say what is wrong, and the lens.

use eh3d_vision_studio::align::{accept, fit, Transform};
use eh3d_vision_studio::scale::{fit_k1, undistort, Scale};

fn near(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

fn moved(design: &[(f64, f64)], theta_deg: f64, scale: f64, d: (f64, f64)) -> Vec<(f64, f64)> {
    let t = Transform {
        dx: d.0,
        dy: d.1,
        theta_deg,
        scale,
        rms_mm: 0.0,
    };
    design.iter().map(|p| t.apply(*p)).collect()
}

#[test]
fn two_fiducials_are_enough_to_find_the_turn_the_shift_and_the_size() {
    let design = [(0.0, 0.0), (100.0, 0.0)];
    let measured = moved(&design, 30.0, 1.002, (5.0, -7.0));
    let t = fit(&design, &measured, true).unwrap();
    assert!(near(t.theta_deg, 30.0, 1e-9) && near(t.scale, 1.002, 1e-9));
    assert!(near(t.dx, 5.0, 1e-9) && near(t.dy, -7.0, 1e-9));
    assert!(t.rms_mm < 1e-9);
}

#[test]
fn points_in_a_line_still_give_the_turn_because_the_line_has_a_direction() {
    let design = [(0.0, 0.0), (10.0, 0.0), (20.0, 0.0), (50.0, 0.0)];
    let measured = moved(&design, -12.5, 1.0, (1.0, 2.0));
    let t = fit(&design, &measured, false).unwrap();
    assert!(near(t.theta_deg, -12.5, 1e-9));
    assert_eq!(t.scale, 1.0);
}

#[test]
fn a_half_turn_is_a_hundred_and_eighty_degrees_in_either_sign_and_comes_back() {
    let design = [(0.0, 0.0), (30.0, 0.0), (30.0, 20.0)];
    let measured = moved(&design, 180.0, 1.0, (0.0, 0.0));
    let t = fit(&design, &measured, false).unwrap();
    assert!(near(t.theta_deg.abs(), 180.0, 1e-9), "{}", t.theta_deg);
    for (d, m) in design.iter().zip(&measured) {
        let back = t.inverse(*m);
        assert!(near(back.0, d.0, 1e-9) && near(back.1, d.1, 1e-9));
    }
}

#[test]
fn a_very_small_turn_is_found_and_does_not_become_noise() {
    let design = [(0.0, 0.0), (200.0, 0.0), (200.0, 150.0), (0.0, 150.0)];
    let measured = moved(&design, 0.01, 1.0, (0.0, 0.0));
    let t = fit(&design, &measured, false).unwrap();
    assert!(near(t.theta_deg, 0.01, 1e-9), "{}", t.theta_deg);
    assert!(t.rms_mm < 1e-9);
}

#[test]
fn the_number_of_points_must_match_and_the_message_says_both_numbers() {
    let e = fit(
        &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)],
        &[(0.0, 0.0), (1.0, 0.0)],
        false,
    )
    .unwrap_err();
    assert!(e.contains('3') && e.contains('2'), "{e}");
    assert!(fit(&[], &[], false).is_err());
    assert!(fit(&[(0.0, 0.0)], &[(0.0, 0.0)], false).is_err());
}

#[test]
fn a_point_that_is_not_a_number_in_the_design_or_in_the_measure_is_refused() {
    let ok = [(0.0, 0.0), (10.0, 0.0)];
    for bad in [f64::NAN, f64::INFINITY] {
        assert!(fit(&[(bad, 0.0), (10.0, 0.0)], &ok, false)
            .unwrap_err()
            .contains("not a number"));
        assert!(fit(&ok, &[(0.0, bad), (10.0, 0.0)], true)
            .unwrap_err()
            .contains("not a number"));
    }
}

#[test]
fn design_points_that_are_all_the_same_point_cannot_give_a_turn() {
    let e = fit(&[(5.0, 5.0), (5.0, 5.0)], &[(0.0, 0.0), (1.0, 1.0)], true).unwrap_err();
    assert!(e.contains("same point"), "{e}");
}

#[test]
fn measured_points_that_all_coincide_give_a_size_that_is_refused_when_the_size_is_free() {
    let e = fit(&[(0.0, 0.0), (10.0, 0.0)], &[(3.0, 3.0), (3.0, 3.0)], true).unwrap_err();
    assert!(e.contains("size"), "{e}");
}

#[test]
fn the_size_is_one_and_the_residual_tells_when_the_size_is_not_free() {
    let design = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
    let measured = moved(&design, 0.0, 1.01, (0.0, 0.0));
    let fixed = fit(&design, &measured, false).unwrap();
    assert_eq!(fixed.scale, 1.0);
    assert!(fixed.rms_mm > 0.1);
    let free = fit(&design, &measured, true).unwrap();
    assert!(near(free.scale, 1.01, 1e-9) && free.rms_mm < 1e-9);
}

#[test]
fn the_transform_is_not_accepted_for_a_residual_or_a_stretch_and_the_message_says_the_numbers() {
    let t = Transform {
        dx: 0.0,
        dy: 0.0,
        theta_deg: 0.0,
        scale: 1.02,
        rms_mm: 0.5,
    };
    let e = accept(&t, 0.2, 0.05).unwrap_err();
    assert!(e.contains("0.500") && e.contains("0.2"), "{e}");
    let e = accept(&t, 1.0, 0.01).unwrap_err();
    assert!(e.contains("2.00 %") && e.contains("1.00 %"), "{e}");
    assert!(accept(&t, 0.5, 0.02 + 1e-12).is_ok());
    // a board that is smaller is stretched in the same way
    let small = Transform {
        scale: 0.98,
        rms_mm: 0.0,
        ..t
    };
    assert!(accept(&small, 1.0, 0.01).is_err());
    assert!(accept(&small, 1.0, 0.03).is_ok());
}

#[test]
fn the_scale_needs_a_mark_of_a_size_and_a_real_size_that_are_more_than_zero_and_finite() {
    for (px, mm) in [
        (0.0, 1.0),
        (-1.0, 1.0),
        (1.0, 0.0),
        (1.0, -2.0),
        (f64::NAN, 1.0),
        (1.0, f64::INFINITY),
    ] {
        assert!(Scale::from_known_diameter(px, mm).is_err(), "{px} {mm}");
    }
    let s = Scale::from_known_diameter(40.0, 2.0).unwrap();
    assert_eq!(s.mm_per_px, 0.05);
    assert!(near(s.mm(100.0), 5.0, 1e-12));
    assert_eq!(s.mm(0.0), 0.0);
}

#[test]
fn a_pixel_below_the_centre_is_below_in_millimetres_too_and_the_centre_is_the_origin() {
    let s = Scale { mm_per_px: 0.1 };
    assert_eq!(s.point(320.0, 240.0, (320.0, 240.0)), (0.0, 0.0));
    let (x, y) = s.point(420.0, 340.0, (320.0, 240.0));
    assert!(near(x, 10.0, 1e-12) && near(y, -10.0, 1e-12));
    let (x, y) = s.point(220.0, 140.0, (320.0, 240.0));
    assert!(near(x, -10.0, 1e-12) && near(y, 10.0, 1e-12));
}

#[test]
fn the_correction_of_the_lens_grows_towards_the_edge_and_a_negative_k1_pulls_the_points_in() {
    let c = (100.0, 100.0);
    let near_centre = undistort((101.0, 100.0), c, 0.1, 100.0);
    let at_edge = undistort((200.0, 100.0), c, 0.1, 100.0);
    assert!(near(near_centre.0 - 101.0, 1.0 * 0.1 * 0.0001, 1e-9));
    assert!(near(at_edge.0, 100.0 + 100.0 * 1.1, 1e-9));
    let pulled = undistort((200.0, 100.0), c, -0.1, 100.0);
    assert!(pulled.0 < 200.0 && near(pulled.0, 100.0 + 100.0 * 0.9, 1e-9));
    // a k1 of zero changes nothing
    assert_eq!(undistort((37.5, 12.25), c, 0.0, 100.0), (37.5, 12.25));
}

#[test]
fn the_distortion_fit_needs_three_points_of_the_same_number_a_norm_and_points_away_from_the_centre()
{
    let c = (0.0, 0.0);
    let pts = [(10.0, 0.0), (0.0, 10.0), (-10.0, 0.0)];
    assert!(fit_k1(&pts, &pts[..2], c, 10.0).unwrap_err().contains('3'));
    assert!(fit_k1(&pts[..2], &pts[..2], c, 10.0).is_err());
    for norm in [0.0, -1.0, f64::NAN] {
        assert!(fit_k1(&pts, &pts, c, norm).unwrap_err().contains("norm"));
    }
    let centre_only = [(0.0, 0.0), (0.0, 0.0), (0.0, 0.0)];
    assert!(fit_k1(&centre_only, &centre_only, c, 10.0)
        .unwrap_err()
        .contains("centre"));
    assert!(near(fit_k1(&pts, &pts, c, 10.0).unwrap(), 0.0, 1e-12));
}
