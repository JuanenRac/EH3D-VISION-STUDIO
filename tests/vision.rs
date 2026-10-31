use eh3d_vision_studio::align::{accept, fit, Transform};
use eh3d_vision_studio::fiducial::{find, otsu, Polarity, Settings};
use eh3d_vision_studio::image::{from_bytes, from_pgm, from_png, Gray, MAX_PIXELS};
use eh3d_vision_studio::scale::{fit_k1, undistort, Scale};

fn board(marks: &[(f64, f64, f64)], dark_marks: bool) -> Gray {
    let (bg, fg) = if dark_marks { (220, 30) } else { (30, 220) };
    let mut g = Gray::blank(300, 200, bg).unwrap();
    for &(x, y, r) in marks {
        g.disc(x, y, r, fg);
    }
    g
}

#[test]
fn the_threshold_falls_between_the_two_levels() {
    let g = board(&[(100.0, 100.0, 30.0)], true);
    let t = otsu(&g);
    assert!((30..220).contains(&t), "{t}");
}

#[test]
fn round_dark_marks_are_found_with_their_centre_and_diameter() {
    let g = board(&[(60.0, 50.0, 10.0), (240.0, 150.0, 14.0)], true);
    let f = find(&g, &Settings::default()).unwrap();
    assert_eq!(f.len(), 2);
    for m in &f {
        let (ex, ey, ed) = if m.x < 150.0 {
            (60.0, 50.0, 20.0)
        } else {
            (240.0, 150.0, 28.0)
        };
        assert!((m.x - ex).abs() < 0.3 && (m.y - ey).abs() < 0.3, "{m:?}");
        assert!((m.diameter - ed).abs() < 1.5, "{m:?}");
        assert!(m.score > 0.9);
    }
}

#[test]
fn light_marks_are_found_when_the_polarity_says_so() {
    let g = board(&[(100.0, 80.0, 12.0)], false);
    assert_eq!(
        find(
            &g,
            &Settings {
                polarity: Polarity::Light,
                ..Settings::default()
            }
        )
        .unwrap()
        .len(),
        1
    );
    let wrong = find(&g, &Settings::default()).unwrap();
    assert!(
        wrong.is_empty(),
        "the dark background touches the edge and is not a disc: {wrong:?}"
    );
}

#[test]
fn what_is_not_a_round_mark_of_the_right_size_is_left_out() {
    let mut g = board(&[(60.0, 50.0, 10.0)], true);
    for y in 100..110 {
        for x in 100..160 {
            g.data[y * g.w + x] = 30; // a bar: too long to be a disc
        }
    }
    g.disc(250.0, 100.0, 2.0, 30); // a speck: too small
    g.disc(150.0, 170.0, 60.0, 30); // a spot that touches the edge
    let f = find(&g, &Settings::default()).unwrap();
    assert_eq!(f.len(), 1, "{f:?}");
    assert!((f[0].x - 60.0).abs() < 0.5);
}

#[test]
fn a_mark_cut_by_the_edge_of_the_picture_is_not_trusted() {
    let g = board(&[(5.0, 100.0, 12.0), (150.0, 100.0, 12.0)], true);
    let f = find(&g, &Settings::default()).unwrap();
    assert_eq!(f.len(), 1);
    assert!((f[0].x - 150.0).abs() < 0.5);
}

#[test]
fn a_flat_picture_and_bad_settings() {
    assert!(
        find(&Gray::blank(50, 50, 128).unwrap(), &Settings::default())
            .unwrap()
            .is_empty()
    );
    let g = board(&[(60.0, 50.0, 10.0)], true);
    for bad in [
        Settings {
            min_diameter: 0.0,
            ..Settings::default()
        },
        Settings {
            max_diameter: 1.0,
            ..Settings::default()
        },
        Settings {
            min_fill: 0.0,
            ..Settings::default()
        },
        Settings {
            max_elongation: 0.5,
            ..Settings::default()
        },
        Settings {
            min_diameter: f64::NAN,
            ..Settings::default()
        },
    ] {
        assert!(find(&g, &bad).is_err(), "{bad:?}");
    }
}

#[test]
fn pgm_and_png_files_are_read_and_damaged_ones_are_refused() {
    let g = from_pgm(b"P2\n# c\n2 2\n255\n0 255\n255 0\n").unwrap();
    assert_eq!((g.w, g.h, g.data), (2, 2, vec![0, 255, 255, 0]));
    for bad in [
        &b""[..],
        b"P6\n1 1\n255\n\0\0\0",
        b"P5\n3 3\n255\n\x01",
        b"P2\n99999 99999\n255\n",
        b"P2\n2 2\n65535\n1 1 1 1",
    ] {
        assert!(from_pgm(bad).is_err());
    }
    let mut png_bytes = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut png_bytes, 2, 1);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()
            .unwrap()
            .write_image_data(&[0, 0, 0, 255, 255, 255, 255, 0])
            .unwrap();
    }
    assert_eq!(
        from_png(&png_bytes).unwrap().data,
        vec![0, 255],
        "opaque black, then transparent white"
    );
    assert_eq!(from_bytes(&png_bytes).unwrap().w, 2);
    assert!(from_png(&png_bytes[..png_bytes.len() / 2]).is_err());
    assert!(Gray::new(MAX_PIXELS, 2, vec![]).is_err() && Gray::new(2, 2, vec![0]).is_err());
}

#[test]
fn the_scale_comes_from_a_mark_of_a_known_size() {
    let s = Scale::from_known_diameter(40.0, 1.0).unwrap();
    assert!((s.mm_per_px - 0.025).abs() < 1e-12);
    assert!((s.mm(100.0) - 2.5).abs() < 1e-12);
    let (x, y) = s.point(160.0, 60.0, (150.0, 100.0));
    assert!(
        (x - 0.25).abs() < 1e-12 && (y - 1.0).abs() < 1e-12,
        "y goes up: {x} {y}"
    );
    for (d, mm) in [
        (0.0, 1.0),
        (-1.0, 1.0),
        (10.0, 0.0),
        (f64::NAN, 1.0),
        (10.0, f64::INFINITY),
    ] {
        assert!(Scale::from_known_diameter(d, mm).is_err());
    }
}

#[test]
fn the_distortion_of_a_lens_is_recovered_from_a_grid_of_marks() {
    let (centre, norm) = ((320.0, 240.0), 400.0);
    let ideal: Vec<(f64, f64)> = (0..5)
        .flat_map(|r| {
            (0..7).map(move |c| (60.0 + f64::from(c) * 87.0, 40.0 + f64::from(r) * 100.0))
        })
        .collect();
    let k1 = -0.08;
    // what the camera sees is the ideal grid bent by the opposite of the correction
    let measured: Vec<(f64, f64)> = ideal
        .iter()
        .map(|&p| {
            let mut m = p;
            for _ in 0..30 {
                let u = undistort(m, centre, k1, norm);
                m = (m.0 + (p.0 - u.0), m.1 + (p.1 - u.1));
            }
            m
        })
        .collect();
    let found = fit_k1(&measured, &ideal, centre, norm).unwrap();
    assert!((found - k1).abs() < 0.01, "{found} against {k1}");
    let back = undistort(measured[3], centre, found, norm);
    assert!(((back.0 - ideal[3].0).powi(2) + (back.1 - ideal[3].1).powi(2)).sqrt() < 1.0);
    assert!(fit_k1(&measured[..2], &ideal[..2], centre, norm).is_err());
    assert!(fit_k1(&measured, &ideal[..5], centre, norm).is_err());
    assert!(fit_k1(&[centre; 4], &[centre; 4], centre, norm).is_err());
    assert!(fit_k1(&measured, &ideal, centre, 0.0).is_err());
}

fn moved(design: &[(f64, f64)], theta_deg: f64, scale: f64, d: (f64, f64)) -> Vec<(f64, f64)> {
    let t = Transform {
        dx: d.0,
        dy: d.1,
        theta_deg,
        scale,
        rms_mm: 0.0,
    };
    design.iter().map(|&p| t.apply(p)).collect()
}

#[test]
fn the_board_that_was_turned_and_shifted_is_found_exactly() {
    let design = [(5.0, 5.0), (95.0, 5.0), (95.0, 65.0), (5.0, 65.0)];
    let measured = moved(&design, 1.5, 1.0, (12.0, -3.0));
    let t = fit(&design, &measured, false).unwrap();
    assert!(
        (t.theta_deg - 1.5).abs() < 1e-9
            && (t.dx - 12.0).abs() < 1e-9
            && (t.dy + 3.0).abs() < 1e-9
            && t.rms_mm < 1e-9,
        "{t:?}"
    );
    accept(&t, 0.1, 0.01).unwrap();
    let p = t.apply((50.0, 30.0));
    let back = t.inverse(p);
    assert!((back.0 - 50.0).abs() < 1e-9 && (back.1 - 30.0).abs() < 1e-9);
}

#[test]
fn two_fiducials_are_enough_and_the_size_is_found_when_allowed() {
    let design = [(0.0, 0.0), (80.0, 0.0)];
    let t = fit(&design, &moved(&design, -2.0, 1.002, (1.0, 2.0)), true).unwrap();
    assert!(
        (t.theta_deg + 2.0).abs() < 1e-6 && (t.scale - 1.002).abs() < 1e-9,
        "{t:?}"
    );
    let fixed = fit(&design, &moved(&design, -2.0, 1.002, (1.0, 2.0)), false).unwrap();
    assert!(
        fixed.rms_mm > 0.05 && fixed.scale == 1.0,
        "without a size the stretch stays as a residual"
    );
}

#[test]
fn a_wrong_fiducial_or_a_stretched_board_is_not_accepted() {
    let design = [(5.0, 5.0), (95.0, 5.0), (95.0, 65.0), (5.0, 65.0)];
    let mut m = moved(&design, 0.5, 1.0, (2.0, 1.0));
    m[2].0 += 1.5; // one mark was taken for another
    let t = fit(&design, &m, false).unwrap();
    assert!(accept(&t, 0.1, 0.01)
        .unwrap_err()
        .contains("fit the design"));
    let big = fit(&design, &moved(&design, 0.0, 1.03, (0.0, 0.0)), true).unwrap();
    assert!(accept(&big, 0.1, 0.01)
        .unwrap_err()
        .contains("bigger or smaller"));
}

#[test]
fn points_that_cannot_give_a_transform_are_refused() {
    assert!(fit(&[(0.0, 0.0)], &[(1.0, 1.0)], false).is_err());
    assert!(fit(&[(0.0, 0.0), (1.0, 1.0)], &[(0.0, 0.0)], false).is_err());
    assert!(
        fit(&[(3.0, 3.0), (3.0, 3.0)], &[(0.0, 0.0), (1.0, 1.0)], false)
            .unwrap_err()
            .contains("same point")
    );
    assert!(fit(
        &[(0.0, 0.0), (f64::NAN, 1.0)],
        &[(0.0, 0.0), (1.0, 1.0)],
        false
    )
    .is_err());
    assert!(
        fit(&[(0.0, 0.0), (1.0, 0.0)], &[(0.0, 0.0), (0.0, 0.0)], true).is_err(),
        "all the measured points on one place cannot be a board"
    );
}
