//! Random input with a fixed seed for the readings of the camera programs: an edge across any line of any picture, the histogram of any picture, a picture against a reference through any mask,
//! the residuals of any set of marks, and documents of the convention of the server that are cut, flipped and filled with garbage. Whatever is put in must never panic, and what comes out must make
//! sense. The generator reads no clock, so a failure comes back the same way and says its seed and its round.

use eh3d_vision_studio::convention::{
    document, sha256_hex, validate, Finding, Source, KINDS, MAX_FINDINGS,
};
use eh3d_vision_studio::diff::DiffSettings;
use eh3d_vision_studio::edge::{locate, measure, EdgeSettings};
use eh3d_vision_studio::histogram::{histogram, report, BINS};
use eh3d_vision_studio::image::Gray;
use eh3d_vision_studio::lens::{residual_report, residuals_to_csv, residuals_to_json};
use eh3d_vision_studio::reference::compare;
use eh3d_vision_studio::server::sc;
use eh3d_vision_studio::sim::SimRng;
use eh3d_vision_studio::station::{findings, read_job, read_request, request};

const SEED: u64 = 0x5EED_0007;

/// A number in [0, 1).
fn unit(rng: &mut SimRng) -> f64 {
    rng.below(1_000_000) as f64 / 1_000_000.0
}

/// A picture of a random size with noise, with flat areas, or with a soft edge across it.
fn picture(rng: &mut SimRng, max: u64) -> Gray {
    let (w, h) = (rng.below(max) as usize + 1, rng.below(max) as usize + 1);
    let mut data: Vec<u8> = (0..w * h).map(|_| rng.below(256) as u8).collect();
    match rng.below(3) {
        0 => {}
        1 => data.iter_mut().for_each(|v| *v = 120),
        _ => {
            let cut = rng.below(w as u64) as usize;
            let (a, b) = (rng.below(256) as u8, rng.below(256) as u8);
            for y in 0..h {
                for x in 0..w {
                    data[y * w + x] = if x < cut { a } else { b };
                }
            }
        }
    }
    Gray::new(w, h, data).unwrap()
}

#[test]
fn an_edge_across_any_line_of_any_picture_never_panics_and_is_inside_the_line() {
    let mut rng = SimRng::new(SEED);
    let (mut found, mut refused) = (0, 0);
    for round in 0..600 {
        let g = picture(&mut rng, 60);
        // the ends are anywhere, a little outside the picture too
        let mut end = || {
            (
                unit(&mut rng) * (g.w as f64 + 4.0) - 2.0,
                unit(&mut rng) * (g.h as f64 + 4.0) - 2.0,
            )
        };
        let (a, b) = (end(), end());
        let s = EdgeSettings {
            step_px: 0.05 + unit(&mut rng) * 0.95,
            max_uncertainty_px: 0.001 + unit(&mut rng) * 4.0,
        };
        match measure(&g, a, b, &s) {
            Ok(e) => {
                found += 1;
                let length = (b.0 - a.0).hypot(b.1 - a.1);
                assert!(
                    e.position_px >= -1e-9 && e.position_px <= length + 1e-9,
                    "seed {SEED:#x} round {round}: {} of {length}",
                    e.position_px
                );
                assert!(
                    e.uncertainty_px.is_finite() && e.uncertainty_px > 0.0,
                    "round {round}"
                );
                assert!(
                    e.contrast >= 8.0 && e.slope > 0.0 && e.width_px > 0.0,
                    "round {round}"
                );
                assert_eq!(e.sure, e.uncertainty_px <= s.max_uncertainty_px);
                let (x, y) = (e.x, e.y);
                assert!(
                    x >= -1e-6 && x <= (g.w - 1) as f64 + 1e-6,
                    "round {round}: {x}"
                );
                assert!(
                    y >= -1e-6 && y <= (g.h - 1) as f64 + 1e-6,
                    "round {round}: {y}"
                );
            }
            Err(why) => {
                refused += 1;
                assert!(why.starts_with("edge"), "round {round}: {why}");
            }
        }
    }
    // both outcomes happen, so the test looks at both
    assert!(
        found > 20 && refused > 20,
        "{found} found, {refused} refused"
    );
}

#[test]
fn a_profile_of_any_numbers_is_read_or_refused_and_never_panics() {
    let mut rng = SimRng::new(SEED ^ 1);
    for round in 0..800 {
        let n = rng.below(40) as usize;
        let mut p: Vec<f64> = (0..n).map(|_| rng.below(256) as f64).collect();
        if rng.below(10) == 0 && !p.is_empty() {
            let k = rng.below(p.len() as u64) as usize;
            p[k] = [f64::NAN, f64::INFINITY, -f64::INFINITY][rng.below(3) as usize];
        }
        let step = if rng.below(8) == 0 {
            0.0
        } else {
            0.05 + unit(&mut rng)
        };
        if let Ok(e) = locate(&p, step, &EdgeSettings::default()) {
            assert!(
                e.position_px >= 0.0 && e.position_px <= step * (n - 1) as f64,
                "round {round}"
            );
            assert!(e.uncertainty_px.is_finite());
        }
    }
}

#[test]
fn the_histogram_of_any_picture_adds_up_and_its_cuts_are_levels() {
    let mut rng = SimRng::new(SEED ^ 2);
    for round in 0..300 {
        let g = picture(&mut rng, 50);
        let h = histogram(&g).unwrap();
        assert_eq!(
            h.counts.iter().sum::<u64>(),
            (g.w * g.h) as u64,
            "round {round}"
        );
        for bins in BINS {
            assert_eq!(h.grouped(bins).unwrap().iter().sum::<u64>(), h.total);
        }
        let r = report(&h);
        assert!(r.min <= r.median && r.median <= r.max, "round {round}");
        assert!(r.p01 <= r.p99);
        assert!(r.isodata <= 254 && r.triangle <= 254, "round {round}");
        assert!(
            (0.0..=1.0).contains(&r.separability),
            "round {round}: {}",
            r.separability
        );
        assert!((0.0..=1.0).contains(&r.dark_share));
        assert!((r.black_share + r.white_share) <= 1.0 + 1e-12);
        assert!(r.mean >= f64::from(r.min) - 1e-9 && r.mean <= f64::from(r.max) + 1e-9);
        // the percentiles never go down as the share goes up
        let mut last = 0u8;
        for k in 0..=20 {
            let v = h.percentile(k as f64 / 20.0);
            assert!(v >= last, "round {round}");
            last = v;
        }
    }
}

#[test]
fn a_picture_against_a_reference_through_any_mask_counts_every_pixel_once() {
    let mut rng = SimRng::new(SEED ^ 3);
    for round in 0..300 {
        let (w, h) = (rng.below(30) as usize + 1, rng.below(30) as usize + 1);
        let make = |rng: &mut SimRng, flat: bool| {
            let data: Vec<u8> = (0..w * h)
                .map(|_| if flat { 100 } else { rng.below(256) as u8 })
                .collect();
            Gray::new(w, h, data).unwrap()
        };
        let flat_a = rng.below(2) == 0;
        let a = make(&mut rng, flat_a);
        let flat_b = rng.below(2) == 0;
        let b = make(&mut rng, flat_b);
        let mask = (rng.below(2) == 0).then(|| {
            let data: Vec<u8> = (0..w * h)
                .map(|_| match rng.below(4) {
                    0 => 255,
                    1 => rng.below(255) as u8,
                    _ => 0,
                })
                .collect();
            Gray::new(w, h, data).unwrap()
        });
        let s = DiffSettings {
            threshold: rng.below(255) as u8,
            min_area: 1 + rng.below(6),
            max_changed: unit(&mut rng),
        };
        match compare(&a, &b, mask.as_ref(), &s) {
            Ok(c) => {
                assert_eq!(c.checked_px + c.ignored_px, (w * h) as u64, "round {round}");
                assert!(c.outside_px <= c.checked_px && c.checked_px > 0);
                assert!((0.0..=1.0).contains(&c.outside_share));
                assert_eq!(c.same, c.outside_share <= s.max_changed);
                assert_eq!(c.bbox.is_none(), c.outside_px == 0, "round {round}");
                if let Some((x0, y0, x1, y1)) = c.bbox {
                    assert!(x0 <= x1 && y0 <= y1 && x1 < w && y1 < h);
                }
                // a picture is never outside the tolerance of itself
                let same = compare(&a, &a.clone(), mask.as_ref(), &s).unwrap();
                assert_eq!(same.outside_px, 0);
            }
            Err(why) => {
                assert!(
                    why.starts_with("reference_nothing_checked"),
                    "round {round}: {why}"
                );
            }
        }
    }
}

#[test]
fn the_residuals_of_any_marks_are_written_or_refused_and_never_panic() {
    let mut rng = SimRng::new(SEED ^ 4);
    let (mut made, mut refused) = (0, 0);
    for round in 0..300 {
        let (cols, rows) = (rng.below(6) as usize + 1, rng.below(6) as usize + 1);
        let count = if rng.below(4) == 0 {
            (rng.below(40)) as usize
        } else {
            cols * rows
        };
        let pitch = 10.0 + unit(&mut rng) * 60.0;
        let pts: Vec<(f64, f64)> = (0..count)
            .map(|k| {
                (
                    100.0 + (k % cols.max(1)) as f64 * pitch + unit(&mut rng) * 3.0,
                    100.0 + (k / cols.max(1)) as f64 * pitch + unit(&mut rng) * 3.0,
                )
            })
            .collect();
        let k1 = unit(&mut rng) - 0.5;
        let k2 = unit(&mut rng) - 0.5;
        let norm = if rng.below(10) == 0 {
            0.0
        } else {
            100.0 + unit(&mut rng) * 300.0
        };
        match residual_report(&pts, cols, rows, (200.0, 150.0), norm, k1, k2) {
            Ok(r) => {
                made += 1;
                assert_eq!(r.marks.len(), cols * rows, "round {round}");
                assert!(
                    r.rms_before.is_finite() && r.rms_after.is_finite(),
                    "round {round}"
                );
                assert!(
                    r.marks[r.worst].after_distance()
                        >= r.rms_after / (r.marks.len() as f64).sqrt() - 1e-9
                );
                assert_eq!(
                    r.bands.iter().map(|b| b.marks).sum::<usize>(),
                    r.marks.len()
                );
                assert_eq!(residuals_to_csv(&r).lines().count(), r.marks.len() + 1);
                assert!(residuals_to_json(&r).contains("eh3d-vision-lens-residuals"));
            }
            Err(_) => refused += 1,
        }
    }
    assert!(made > 20 && refused > 20, "{made} made, {refused} refused");
}

/// A valid document of every kind with a few findings.
fn documents() -> Vec<String> {
    let source = Source {
        width: 640,
        height: 480,
        sha256: Some(sha256_hex(b"picture")),
    };
    KINDS
        .iter()
        .map(|kind| {
            let list = findings(kind, &source, 7, 8);
            document(kind, &source, &list, "picture").unwrap()
        })
        .collect()
}

#[test]
fn a_document_that_is_cut_flipped_or_filled_with_garbage_is_judged_and_never_panics() {
    let mut rng = SimRng::new(SEED ^ 5);
    let docs = documents();
    for d in &docs {
        assert!(validate(d).is_ok(), "{d}");
    }
    let mut refused = 0;
    for round in 0..1500 {
        let base = docs[rng.below(docs.len() as u64) as usize].clone();
        let mut bytes = base.into_bytes();
        match rng.below(4) {
            0 => bytes.truncate(rng.below(bytes.len() as u64) as usize),
            1 => {
                for _ in 0..=rng.below(3) {
                    let k = rng.below(bytes.len() as u64) as usize;
                    bytes[k] = rng.below(256) as u8;
                }
            }
            2 => {
                let k = rng.below(bytes.len() as u64) as usize;
                bytes.splice(k..k, (0..rng.below(8)).map(|_| rng.below(256) as u8));
            }
            _ => bytes = (0..rng.below(80)).map(|_| rng.below(256) as u8).collect(),
        }
        let text = String::from_utf8_lossy(&bytes).to_string();
        if let Ok(c) = validate(&text) {
            // what is accepted says what it is
            assert!(KINDS.contains(&c.kind.as_str()), "round {round}");
            assert!(c.findings <= MAX_FINDINGS);
        } else {
            refused += 1;
        }
    }
    assert!(refused > 700, "{refused}");
}

#[test]
fn requests_and_answers_of_the_station_made_of_garbage_are_refused_and_never_panic() {
    let mut rng = SimRng::new(SEED ^ 6);
    let pieces = [
        "{",
        "}",
        "[",
        "]",
        ",",
        ":",
        "\"kind\"",
        "\"detect\"",
        "\"what\"",
        "\"focus\"",
        "\"blobs\"",
        "\"image\"",
        "\"width\"",
        "\"height\"",
        "\"sha256\"",
        "\"seed\"",
        "\"output\"",
        "\"artifact\"",
        "\"state\"",
        "\"done\"",
        "\"view\"",
        "\"result\"",
        "\"id\"",
        "12",
        "-3",
        "20001",
        "null",
        "true",
        "\"x\"",
        "1e999",
        "\"failed\"",
    ];
    let (mut accepted, mut refused) = (0, 0);
    for _ in 0..3000 {
        let n = rng.below(14) as usize;
        let text: String = (0..n)
            .map(|_| pieces[rng.below(pieces.len() as u64) as usize])
            .collect::<Vec<_>>()
            .join(if rng.below(2) == 0 { "" } else { " " });
        if let Ok(j) = sc::Json::parse(&text) {
            match read_request(&j) {
                Ok(_) => accepted += 1,
                Err(_) => refused += 1,
            }
        }
        let _ = read_job(&text);
    }
    assert!(refused > 100, "{refused} refused, {accepted} accepted");
    // a request that is built is read back the same, over many pictures and seeds
    for round in 0..300 {
        let kind = KINDS[rng.below(4) as usize];
        let source = Source {
            width: rng.below(20_000) + 1,
            height: rng.below(20_000) + 1,
            sha256: (rng.below(2) == 0).then(|| sha256_hex(&rng.below(1_000_000).to_le_bytes())),
        };
        let seed = rng.below(u64::from(u32::MAX));
        let text = request(kind, &source, seed, rng.below(2) == 0, None).unwrap();
        let asked = read_request(&sc::Json::parse(&text).unwrap()).unwrap();
        assert_eq!(
            (asked.what.as_str(), asked.source, asked.seed),
            (kind, source, seed),
            "round {round}"
        );
    }
}

#[test]
fn a_document_of_any_findings_is_made_valid_or_refused_whole() {
    let mut rng = SimRng::new(SEED ^ 7);
    let (mut made, mut refused) = (0, 0);
    for round in 0..600 {
        let kind = KINDS[rng.below(4) as usize];
        let source = Source {
            width: rng.below(25_000),
            height: rng.below(25_000),
            sha256: (rng.below(3) == 0).then(|| "z".repeat(rng.below(70) as usize)),
        };
        let inside = source.width.min(source.height) as f64;
        let n = rng.below(6) as usize;
        let list: Vec<Finding> = (0..n)
            .map(|_| match kind {
                "fiducials" => Finding::Fiducial {
                    x: unit(&mut rng) * inside,
                    y: unit(&mut rng) * inside,
                    radius_px: 0.5 + unit(&mut rng) * 20.0,
                    score: unit(&mut rng),
                },
                "blobs" => Finding::Blob {
                    x: unit(&mut rng) * inside,
                    y: unit(&mut rng) * inside,
                    area_px: 1 + rng.below(500),
                },
                "focus" => Finding::Focus {
                    score: unit(&mut rng) * 100.0,
                    sharp: rng.below(2) == 0,
                },
                _ => Finding::Barcode {
                    format: "qr".into(),
                    text: "t".into(),
                    x: unit(&mut rng) * inside,
                    y: unit(&mut rng) * inside,
                },
            })
            .collect();
        let sides_ok =
            (1..=20_000).contains(&source.width) && (1..=20_000).contains(&source.height);
        let valid_source = sides_ok && source.sha256.is_none();
        match document(kind, &source, &list, "picture") {
            Ok(text) => {
                made += 1;
                assert!(
                    valid_source,
                    "round {round}: a document was made for a picture that does not fit"
                );
                // a document that is made follows the convention that checks what is read
                let c = validate(&text).unwrap_or_else(|e| {
                    panic!(
                        "round {round}: {e}
{text}"
                    )
                });
                assert_eq!((c.kind.as_str(), c.findings), (kind, n));
            }
            Err(_) => {
                refused += 1;
                assert!(
                    !valid_source,
                    "round {round}: a document for a good picture was refused"
                );
            }
        }
    }
    assert!(made > 20 && refused > 20, "{made} made, {refused} refused");
}
