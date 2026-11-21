//! Pure functions at their exact edges: the sizes of the ring marks, the angles, the lens, and the headers of a motion JPEG frame (codes that do not change, never a panic).

use eh3d_vision_studio::lens::{distort2, undistort2};
use eh3d_vision_studio::mjpeg::{
    check, code_of, decode, JpegSettings, BAD_MARKER, BAD_SIZE, BAD_TABLE, BYTES_RANGE, CODES,
    FILE_TOO_BIG, NOT_JPEG, PIXELS_RANGE, TRUNCATED, UNSUPPORTED,
};
use eh3d_vision_studio::orientation::wrap_deg;
use eh3d_vision_studio::ring::{classify, SizeClass};

fn close(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = (self.0 * 1_103_515_245 + 12_345) % 2_147_483_648;
        self.0 >> 4
    }

    fn unit(&mut self) -> f64 {
        (self.next() % 1_000_000) as f64 / 1_000_000.0
    }
}

// ---- the sizes of the ring marks ----

#[test]
fn a_given_split_puts_the_diameter_equal_to_it_among_the_large() {
    let c = classify(&[10.0, 20.0, 30.0], 20.0, 1.5);
    assert_eq!(c, [SizeClass::Small, SizeClass::Large, SizeClass::Large]);
}

#[test]
fn the_widest_gap_between_the_diameters_decides_and_the_order_is_kept() {
    let c = classify(&[30.0, 10.0, 32.0, 11.0], 0.0, 1.5);
    assert_eq!(
        c,
        [
            SizeClass::Large,
            SizeClass::Small,
            SizeClass::Large,
            SizeClass::Small
        ]
    );
    let c = classify(&[10.0, 16.0, 40.0], 0.0, 1.5);
    assert_eq!(c, [SizeClass::Small, SizeClass::Small, SizeClass::Large]);
}

#[test]
fn a_gap_exactly_as_wide_as_asked_still_counts() {
    assert_eq!(
        classify(&[10.0, 20.0], 0.0, 2.0),
        [SizeClass::Small, SizeClass::Large]
    );
    assert_eq!(
        classify(&[10.0, 19.9], 0.0, 2.0),
        [SizeClass::Single, SizeClass::Single]
    );
}

#[test]
fn rings_of_one_size_or_none_are_single_or_nothing() {
    assert!(classify(&[], 0.0, 1.5).is_empty());
    assert_eq!(classify(&[12.0], 0.0, 1.5), [SizeClass::Single]);
    assert_eq!(
        classify(&[10.0, 10.5, 11.0], 0.0, 1.5),
        [SizeClass::Single; 3]
    );
}

#[test]
fn a_split_that_is_not_positive_is_found_from_the_diameters() {
    let auto = classify(&[10.0, 11.0, 30.0], 0.0, 1.5);
    assert_eq!(classify(&[10.0, 11.0, 30.0], -5.0, 1.5), auto);
    assert_eq!(auto, [SizeClass::Small, SizeClass::Small, SizeClass::Large]);
}

#[test]
fn diameters_that_are_not_numbers_never_panic_the_classes() {
    let c = classify(&[f64::NAN, 10.0, 20.0], 0.0, 1.5);
    assert_eq!(c.len(), 3);
    let d = classify(&[f64::INFINITY, 10.0], 0.0, 1.5);
    assert_eq!(d.len(), 2);
}

#[test]
fn the_class_words_are_the_ones_the_reports_write() {
    assert_eq!(SizeClass::Single.word(), "single");
    assert_eq!(SizeClass::Small.word(), "small");
    assert_eq!(SizeClass::Large.word(), "large");
}

// ---- angles ----

#[test]
fn an_angle_is_brought_to_minus_one_eighty_exclusive_up_to_one_eighty() {
    let table = [
        (0.0, 0.0),
        (180.0, 180.0),
        (-180.0, 180.0),
        (181.0, -179.0),
        (-181.0, 179.0),
        (360.0, 0.0),
        (540.0, 180.0),
        (-540.0, 180.0),
        (359.0, -1.0),
        (-359.0, 1.0),
        (720.5, 0.5),
    ];
    for (a, want) in table {
        assert!(close(wrap_deg(a), want, 1e-9), "{a} gave {}", wrap_deg(a));
    }
}

#[test]
fn an_angle_that_is_not_finite_stays_not_a_number() {
    assert!(wrap_deg(f64::NAN).is_nan());
    assert!(wrap_deg(f64::INFINITY).is_nan());
    assert!(wrap_deg(f64::NEG_INFINITY).is_nan());
}

#[test]
fn wrapping_keeps_the_angle_modulo_a_turn_and_is_idempotent_from_a_fixed_seed() {
    let mut r = Lcg(4321);
    for _ in 0..500 {
        let a = r.unit() * 20_000.0 - 10_000.0;
        let w = wrap_deg(a);
        assert!(w > -180.0 - 1e-9 && w <= 180.0 + 1e-9, "{a} {w}");
        assert!(close(
            (a - w)
                .rem_euclid(360.0)
                .min(360.0 - (a - w).rem_euclid(360.0)),
            0.0,
            1e-6
        ));
        assert!(close(wrap_deg(w), w, 1e-9));
    }
}

// ---- the lens ----

const CENTRE: (f64, f64) = (320.0, 240.0);

#[test]
fn a_lens_with_no_distortion_leaves_every_point_where_it_is() {
    let mut r = Lcg(77);
    for _ in 0..100 {
        let p = (r.unit() * 640.0, r.unit() * 480.0);
        let u = undistort2(p, CENTRE, 0.0, 0.0, 500.0);
        let d = distort2(p, CENTRE, 0.0, 0.0, 500.0);
        assert!(close(u.0, p.0, 1e-9) && close(u.1, p.1, 1e-9));
        assert!(close(d.0, p.0, 1e-9) && close(d.1, p.1, 1e-9));
    }
}

#[test]
fn the_centre_of_the_lens_never_moves() {
    for (k1, k2) in [(-0.2, 0.05), (0.3, -0.1), (0.5, 0.5)] {
        assert_eq!(undistort2(CENTRE, CENTRE, k1, k2, 400.0), CENTRE);
        assert_eq!(distort2(CENTRE, CENTRE, k1, k2, 400.0), CENTRE);
    }
}

#[test]
fn bending_a_point_and_correcting_it_give_back_the_point_from_a_fixed_seed() {
    let mut r = Lcg(2024);
    for _ in 0..200 {
        let ideal = (
            CENTRE.0 + (r.unit() - 0.5) * 500.0,
            CENTRE.1 + (r.unit() - 0.5) * 380.0,
        );
        let bent = distort2(ideal, CENTRE, -0.05, 0.01, 500.0);
        let back = undistort2(bent, CENTRE, -0.05, 0.01, 500.0);
        assert!(
            close(back.0, ideal.0, 1e-6) && close(back.1, ideal.1, 1e-6),
            "{ideal:?}"
        );
    }
}

#[test]
fn the_correction_is_radial_it_keeps_the_direction_from_the_centre() {
    let mut r = Lcg(9);
    for _ in 0..100 {
        let p = (r.unit() * 640.0, r.unit() * 480.0);
        let u = undistort2(p, CENTRE, -0.1, 0.02, 500.0);
        let cross = (p.0 - CENTRE.0) * (u.1 - CENTRE.1) - (p.1 - CENTRE.1) * (u.0 - CENTRE.0);
        assert!(cross.abs() < 1e-6, "{p:?} {cross}");
    }
}

#[test]
fn a_positive_first_factor_pushes_the_points_outwards() {
    let p = (CENTRE.0 + 100.0, CENTRE.1 + 50.0);
    let u = undistort2(p, CENTRE, 0.1, 0.0, 400.0);
    let before = (p.0 - CENTRE.0).hypot(p.1 - CENTRE.1);
    let after = (u.0 - CENTRE.0).hypot(u.1 - CENTRE.1);
    assert!(after > before);
    let n = undistort2(p, CENTRE, -0.1, 0.0, 400.0);
    assert!((n.0 - CENTRE.0).hypot(n.1 - CENTRE.1) < before);
}

#[test]
fn a_bend_that_cannot_be_undone_gives_the_point_back_and_a_bad_point_never_panics() {
    let far = (1000.0, 0.0);
    assert_eq!(distort2(far, (0.0, 0.0), -1.0, 0.0, 100.0), far);
    let nan = distort2((f64::NAN, 0.0), CENTRE, -0.1, 0.0, 500.0);
    assert!(nan.0.is_nan());
    let _ = undistort2((f64::INFINITY, 0.0), CENTRE, -0.1, 0.0, 500.0);
}

// ---- the headers of a motion JPEG frame ----

const SOI: [u8; 2] = [0xFF, 0xD8];
const EOI: [u8; 2] = [0xFF, 0xD9];

fn seg(marker: u8, body: &[u8]) -> Vec<u8> {
    let len = body.len() + 2;
    let mut v = vec![0xFF, marker, (len >> 8) as u8, (len & 0xFF) as u8];
    v.extend_from_slice(body);
    v
}

fn frame(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut v = SOI.to_vec();
    for p in parts {
        v.extend_from_slice(p);
    }
    v.extend_from_slice(&EOI);
    v
}

fn code(bytes: &[u8]) -> String {
    match decode(bytes, &JpegSettings::default()) {
        Ok(_) => "ok".to_string(),
        Err(e) => code_of(&e).to_string(),
    }
}

fn sof(n: u8, comps: &[[u8; 3]]) -> Vec<u8> {
    let mut body = vec![8, 0, 8, 0, 8, n];
    for c in comps {
        body.extend_from_slice(c);
    }
    seg(0xC0, &body)
}

fn grey_sof() -> Vec<u8> {
    sof(1, &[[1, 0x11, 0]])
}

fn dqt_ones() -> Vec<u8> {
    let mut b = vec![0x00];
    b.extend_from_slice(&[1u8; 64]);
    seg(0xDB, &b)
}

fn dht_one_symbol() -> Vec<u8> {
    let mut b = vec![0x00, 1];
    b.extend_from_slice(&[0u8; 15]);
    b.push(5);
    seg(0xC4, &b)
}

#[test]
fn the_limits_of_the_settings_are_exact() {
    let ok = |b: usize, p: usize| {
        check(&JpegSettings {
            max_bytes: b,
            max_pixels: p,
        })
        .is_ok()
    };
    assert!(ok(BYTES_RANGE.0, PIXELS_RANGE.0));
    assert!(ok(BYTES_RANGE.1, PIXELS_RANGE.1));
    assert!(!ok(BYTES_RANGE.0 - 1, PIXELS_RANGE.0));
    assert!(!ok(BYTES_RANGE.1 + 1, PIXELS_RANGE.0));
    assert!(!ok(BYTES_RANGE.0, PIXELS_RANGE.0 - 1));
    assert!(!ok(BYTES_RANGE.0, PIXELS_RANGE.1 + 1));
    assert!(check(&JpegSettings::default()).is_ok());
}

#[test]
fn settings_that_are_out_of_range_are_refused_before_the_frame_is_looked_at() {
    let bad = JpegSettings {
        max_bytes: 10,
        max_pixels: 100,
    };
    let e = decode(&frame(&[]), &bad).err().unwrap();
    assert!(e.contains("most bytes"), "{e}");
}

#[test]
fn the_codes_are_the_start_of_the_text_and_are_all_different() {
    assert_eq!(code_of("jpeg_truncated: cut"), "jpeg_truncated");
    assert_eq!(code_of("no colon"), "no colon");
    assert_eq!(code_of(""), "");
    let mut codes = CODES.to_vec();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), CODES.len());
    assert!(CODES.iter().all(|c| c.starts_with("jpeg_")));
}

#[test]
fn what_does_not_start_as_a_picture_is_not_a_jpeg() {
    let cases: [&[u8]; 6] = [
        &[],
        &[0xFF],
        &[0xFF, 0xD9],
        &[0xD8, 0xFF],
        b"GIF89a",
        b"plain text",
    ];
    for bytes in cases {
        assert_eq!(code(bytes), NOT_JPEG, "{bytes:?}");
    }
}

#[test]
fn a_frame_of_exactly_the_most_bytes_is_looked_at_and_one_more_is_too_big() {
    let s = JpegSettings {
        max_bytes: BYTES_RANGE.0,
        max_pixels: 100,
    };
    let mut at = SOI.to_vec();
    at.resize(BYTES_RANGE.0, 0);
    assert_eq!(code_of(&decode(&at, &s).err().unwrap()), TRUNCATED);
    at.push(0);
    assert_eq!(code_of(&decode(&at, &s).err().unwrap()), FILE_TOO_BIG);
}

#[test]
fn a_frame_that_ends_before_the_picture_is_truncated() {
    assert_eq!(code(&SOI), TRUNCATED);
    assert_eq!(code(&frame(&[])), TRUNCATED);
    assert_eq!(code(&[0xFF, 0xD8, 0xFF, 0xDB]), TRUNCATED);
    assert_eq!(code(&[0xFF, 0xD8, 0xFF, 0xDB, 0x00]), TRUNCATED);
    assert_eq!(code(&[0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0xFF, 0x00]), TRUNCATED);
    assert_eq!(code(&[0xFF, 0xD8, 0xFF, 0xFF, 0xFF, 0xD9]), TRUNCATED);
    assert_eq!(
        code(&frame(&[seg(0xE0, b"JFIF\0"), seg(0xFE, b"hi")])),
        TRUNCATED
    );
}

#[test]
fn a_marker_that_cannot_be_where_it_is_is_a_bad_marker() {
    assert_eq!(code(&[0xFF, 0xD8, 0xFF, 0xD8]), BAD_MARKER);
    assert_eq!(code(&[0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x01]), BAD_MARKER);
    assert_eq!(code(&[0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x00]), BAD_MARKER);
}

#[test]
fn the_restart_interval_has_one_length() {
    assert_eq!(code(&frame(&[seg(0xDD, &[0, 0, 0])])), BAD_MARKER);
    assert_eq!(code(&frame(&[seg(0xDD, &[0])])), BAD_MARKER);
    assert_eq!(code(&frame(&[seg(0xDD, &[0, 4])])), TRUNCATED);
}

#[test]
fn kinds_of_jpeg_that_are_not_read_are_named_as_such() {
    for m in [
        0xC2u8, 0xC3, 0xC5, 0xC6, 0xC7, 0xC9, 0xCA, 0xCB, 0xCD, 0xCE, 0xCF,
    ] {
        assert_eq!(code(&frame(&[seg(m, &[])])), UNSUPPORTED, "{m:#x}");
    }
}

#[test]
fn the_frame_header_is_checked_field_by_field() {
    assert_eq!(code(&frame(&[grey_sof()])), TRUNCATED);
    assert_eq!(
        code(&frame(&[seg(0xC1, &[8, 0, 8, 0, 8, 1, 1, 0x11, 0])])),
        TRUNCATED
    );
    assert_eq!(code(&frame(&[seg(0xC0, &[8, 0, 8])])), BAD_MARKER);
    assert_eq!(code(&frame(&[seg(0xC0, &[8, 0, 8, 0, 8, 1])])), BAD_MARKER);
    assert_eq!(code(&frame(&[grey_sof(), grey_sof()])), BAD_MARKER);
    assert_eq!(
        code(&frame(&[seg(0xC0, &[12, 0, 8, 0, 8, 1, 1, 0x11, 0])])),
        UNSUPPORTED
    );
    assert_eq!(code(&frame(&[seg(0xC0, &[8, 0, 8, 0, 8, 0])])), UNSUPPORTED);
    assert_eq!(
        code(&frame(&[sof(2, &[[1, 0x11, 0], [2, 0x11, 0]])])),
        UNSUPPORTED
    );
    let four = [[1, 0x11, 0], [2, 0x11, 0], [3, 0x11, 0], [4, 0x11, 0]];
    assert_eq!(code(&frame(&[sof(4, &four)])), UNSUPPORTED);
}

#[test]
fn a_size_of_nothing_or_over_the_pixels_allowed_is_a_bad_size() {
    let zero_w = seg(0xC0, &[8, 0, 8, 0, 0, 1, 1, 0x11, 0]);
    let zero_h = seg(0xC0, &[8, 0, 0, 0, 8, 1, 1, 0x11, 0]);
    assert_eq!(code(&frame(&[zero_w])), BAD_SIZE);
    assert_eq!(code(&frame(&[zero_h])), BAD_SIZE);
    let tight = JpegSettings {
        max_bytes: BYTES_RANGE.0,
        max_pixels: 64,
    };
    let eight = frame(&[grey_sof()]);
    assert_eq!(code_of(&decode(&eight, &tight).err().unwrap()), TRUNCATED);
    let nine = frame(&[seg(0xC0, &[8, 0, 9, 0, 8, 1, 1, 0x11, 0])]);
    assert_eq!(code_of(&decode(&nine, &tight).err().unwrap()), BAD_SIZE);
}

#[test]
fn a_component_that_cannot_be_is_a_bad_marker() {
    for bad in [
        [1u8, 0x01, 0],
        [1, 0x10, 0],
        [1, 0x51, 0],
        [1, 0x15, 0],
        [1, 0x11, 4],
    ] {
        assert_eq!(code(&frame(&[sof(1, &[bad])])), BAD_MARKER, "{bad:?}");
    }
    let twice = [[1, 0x11, 0], [1, 0x11, 0], [3, 0x11, 0]];
    assert_eq!(code(&frame(&[sof(3, &twice)])), BAD_MARKER);
    let colour = [[1, 0x22, 0], [2, 0x11, 1], [3, 0x11, 1]];
    assert_eq!(code(&frame(&[sof(3, &colour)])), TRUNCATED);
}

#[test]
fn the_tables_of_quantisation_are_checked() {
    let mut precision = vec![0x20];
    precision.extend_from_slice(&[1u8; 64]);
    let mut number = vec![0x04];
    number.extend_from_slice(&[1u8; 64]);
    let mut cut = vec![0x00];
    cut.extend_from_slice(&[1u8; 10]);
    let mut zero = vec![0x00];
    zero.extend_from_slice(&[0u8; 64]);
    let mut zero16 = vec![0x10];
    zero16.extend_from_slice(&[0u8; 128]);
    for body in [precision, number, cut, zero, zero16] {
        assert_eq!(code(&frame(&[seg(0xDB, &body)])), BAD_TABLE, "{body:?}");
    }
    let mut wide = vec![0x10];
    for _ in 0..64 {
        wide.extend_from_slice(&[0, 1]);
    }
    assert_eq!(code(&frame(&[seg(0xDB, &wide)])), TRUNCATED);
    let mut two = vec![0x00];
    two.extend_from_slice(&[1u8; 64]);
    two.push(0x01);
    two.extend_from_slice(&[1u8; 64]);
    assert_eq!(code(&frame(&[seg(0xDB, &two)])), TRUNCATED);
    assert_eq!(code(&frame(&[seg(0xDB, &[])])), TRUNCATED);
}

#[test]
fn the_tables_of_codes_are_checked() {
    let with = |first: u8, counts: &[u8], symbols: &[u8]| {
        let mut b = vec![first];
        let mut c = [0u8; 16];
        c[..counts.len()].copy_from_slice(counts);
        b.extend_from_slice(&c);
        b.extend_from_slice(symbols);
        seg(0xC4, &b)
    };
    assert_eq!(code(&frame(&[with(0x20, &[1], &[5])])), BAD_TABLE);
    assert_eq!(code(&frame(&[with(0x04, &[1], &[5])])), BAD_TABLE);
    assert_eq!(code(&frame(&[with(0x00, &[], &[])])), BAD_TABLE);
    assert_eq!(code(&frame(&[with(0x00, &[2], &[5])])), BAD_TABLE);
    assert_eq!(code(&frame(&[with(0x00, &[3], &[1, 2, 3])])), BAD_TABLE);
    assert_eq!(code(&frame(&[seg(0xC4, &[0x00, 1, 2])])), BAD_TABLE);
    assert_eq!(code(&frame(&[with(0x00, &[1], &[5])])), TRUNCATED);
    assert_eq!(code(&frame(&[with(0x10, &[1], &[5])])), TRUNCATED);
    assert_eq!(code(&frame(&[with(0x00, &[2], &[5, 6])])), TRUNCATED);
}

#[test]
fn all_the_tables_and_the_header_together_still_lack_the_coded_picture() {
    let all = frame(&[dqt_ones(), grey_sof(), dht_one_symbol()]);
    assert_eq!(code(&all), TRUNCATED);
    for n in 0..all.len() {
        let c = code(&all[..n]);
        assert!(CODES.contains(&c.as_str()), "{n}: {c}");
    }
}

#[test]
fn frames_of_noise_after_the_start_marker_are_refused_with_a_known_code_from_a_fixed_seed() {
    let mut r = Lcg(31_337);
    for _ in 0..400 {
        let n = (r.next() % 90) as usize;
        let mut bytes = SOI.to_vec();
        for _ in 0..n {
            bytes.push(if r.next() % 5 == 4 {
                0xFF
            } else {
                (r.next() & 0xFF) as u8
            });
        }
        if r.next() & 1 == 0 {
            bytes.extend_from_slice(&EOI);
        }
        let c = code(&bytes);
        assert!(c == "ok" || CODES.contains(&c.as_str()), "{c} {bytes:?}");
    }
}
