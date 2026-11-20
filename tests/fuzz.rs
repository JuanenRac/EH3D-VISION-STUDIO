//! Random input with a fixed seed: whatever is put in (pictures of any size and grey levels, corners anywhere, text that is almost a configuration file or a JSON object or an HTTP answer)
//! must never panic, and what comes out must make sense. The generator reads no clock, so a failure comes back the same way and says its seed and its round.

use eh3d_vision_studio::config::Config;
use eh3d_vision_studio::correlate::{correlate, CorrelateSettings};
use eh3d_vision_studio::diff::{compare, mask, DiffSettings};
use eh3d_vision_studio::fiducial::{find, Settings};
use eh3d_vision_studio::image::{from_bytes, Gray};
use eh3d_vision_studio::measure::{measure_parts, Part, PartSettings};
use eh3d_vision_studio::pads::{measure_row, PadSettings};
use eh3d_vision_studio::perspective::{convex, rectify, Homography};
use eh3d_vision_studio::quality;
use eh3d_vision_studio::scale::Scale;
use eh3d_vision_studio::server::sc::transport::parse_response;
use eh3d_vision_studio::server::{
    apply_remote, base64_decode, base64_encode, parse_config_text, parse_object,
};
use eh3d_vision_studio::sim::SimRng;

const SEED: u64 = 0x5EED_0005;

/// A number in [0, 1).
fn unit(rng: &mut SimRng) -> f64 {
    rng.below(1_000_000) as f64 / 1_000_000.0
}

/// A picture of a random size (1 to `max` on each side) with random grey levels, or with a few flat areas, so that both noise and edges are met.
fn picture(rng: &mut SimRng, max: u64) -> Gray {
    let (w, h) = (rng.below(max) as usize + 1, rng.below(max) as usize + 1);
    let mut data: Vec<u8> = (0..w * h).map(|_| rng.below(256) as u8).collect();
    if rng.below(2) == 0 {
        // flat areas on a flat board
        let board = rng.below(256) as u8;
        data.iter_mut().for_each(|v| *v = board);
        for _ in 0..rng.below(4) {
            let (x0, y0) = (rng.below(w as u64) as usize, rng.below(h as u64) as usize);
            let (x1, y1) = (
                (x0 + rng.below(12) as usize + 1).min(w),
                (y0 + rng.below(12) as usize + 1).min(h),
            );
            let v = rng.below(256) as u8;
            for y in y0..y1 {
                for x in x0..x1 {
                    data[y * w + x] = v;
                }
            }
        }
    }
    Gray::new(w, h, data).unwrap()
}

#[test]
fn pictures_of_any_size_go_through_every_search_without_a_panic() {
    let mut rng = SimRng::new(SEED);
    for round in 0..300 {
        let img = picture(&mut rng, 48);
        let ctx = format!("seed {SEED:#x}, round {round}, {} x {}", img.w, img.h);
        if let Ok(q) = quality::measure(&img) {
            assert!(q.mean >= 0.0 && q.mean <= 255.0, "{ctx}");
            assert!(q.contrast >= 0.0 && q.sharpness >= 0.0, "{ctx}");
        }
        if let Ok(found) = find(&img, &Settings::default()) {
            for f in found {
                assert!(
                    f.x >= 0.0 && f.x <= img.w as f64 && f.y >= 0.0 && f.y <= img.h as f64,
                    "{ctx}"
                );
            }
        }
        if let Ok(parts) = measure_parts(&img, &PartSettings::default(), Scale { mm_per_px: 0.05 })
        {
            for p in parts {
                assert!(
                    p.length_mm >= p.width_mm - 1e-9 && p.area_mm2 > 0.0,
                    "{ctx}"
                );
                assert!(
                    p.angle_deg > -90.0 - 1e-9 && p.angle_deg <= 90.0 + 1e-9,
                    "{ctx}"
                );
            }
        }
        let diameter = 4.0 + unit(&mut rng) * 20.0;
        let s = CorrelateSettings {
            diameter,
            min_score: 0.3 + unit(&mut rng) * 0.7,
            ..CorrelateSettings::default()
        };
        if let Ok(found) = correlate(&img, &s) {
            let mut last = f64::INFINITY;
            for m in &found {
                assert!(m.score >= s.min_score && m.score <= 1.0, "{ctx}: {m:?}");
                assert!(m.score <= last, "{ctx}: not the best first");
                last = m.score;
                assert!(
                    m.x >= 0.0 && m.x <= img.w as f64 && m.y >= 0.0 && m.y <= img.h as f64,
                    "{ctx}: {m:?}"
                );
            }
            // two marks are never closer than a diameter
            for (i, a) in found.iter().enumerate() {
                for b in &found[i + 1..] {
                    // the centres are worked out to a fraction of a pixel, so a little less than a diameter is allowed
                    assert!(
                        ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt() >= diameter - 1.0,
                        "{ctx}"
                    );
                }
            }
        }
    }
}

#[test]
fn two_pictures_are_compared_without_a_panic_and_the_numbers_make_sense() {
    let mut rng = SimRng::new(SEED + 1);
    for round in 0..200 {
        let a = picture(&mut rng, 40);
        let same_size = rng.below(4) != 0;
        let b = if same_size {
            let mut b = a.clone();
            for v in b.data.iter_mut() {
                if rng.below(5) == 0 {
                    *v = rng.below(256) as u8;
                }
            }
            b
        } else {
            picture(&mut rng, 40)
        };
        let s = DiffSettings {
            threshold: rng.below(255) as u8,
            min_area: rng.below(6) + 1,
            max_changed: unit(&mut rng),
        };
        let ctx = format!("seed {:#x}, round {round}", SEED + 1);
        match compare(&a, &b, &s) {
            Ok(d) => {
                assert!(same_size || (a.w, a.h) == (b.w, b.h), "{ctx}");
                assert_eq!(d.total_px, (a.w * a.h) as u64, "{ctx}");
                assert!(
                    d.changed_px <= d.total_px && (0.0..=1.0).contains(&d.changed_share),
                    "{ctx}"
                );
                assert_eq!(d.same, d.changed_share <= s.max_changed, "{ctx}");
                assert_eq!(d.bbox.is_none(), d.changed_px == 0, "{ctx}");
                assert!(d.regions as u64 <= d.changed_px, "{ctx}");
                assert_eq!(
                    mask(&a, &b, &s)
                        .unwrap()
                        .data
                        .iter()
                        .filter(|v| **v == 255)
                        .count() as u64,
                    d.changed_px,
                    "{ctx}"
                );
                // the other way round is the same
                assert_eq!(compare(&b, &a, &s).unwrap(), d, "{ctx}");
                // a picture is never different from itself
                let own = compare(&a, &a, &s).unwrap();
                assert!(own.same && own.changed_px == 0 && own.max_abs == 0, "{ctx}");
            }
            Err(e) => {
                assert!(e.starts_with("size_mismatch"), "{ctx}: {e}");
                assert!((a.w, a.h) != (b.w, b.h), "{ctx}");
            }
        }
    }
}

/// Four corners that are a convex four-sided shape: the corners of a rectangle, each moved by up to a fifth of the sides.
fn quad(rng: &mut SimRng, w: f64, h: f64) -> [(f64, f64); 4] {
    let jitter = |rng: &mut SimRng, side: f64| (unit(rng) - 0.5) * 0.4 * side;
    let c = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
    let mut out = [(0.0, 0.0); 4];
    for (o, p) in out.iter_mut().zip(c) {
        *o = (p.0 + jitter(rng, w), p.1 + jitter(rng, h));
    }
    out
}

#[test]
fn corners_anywhere_are_refused_or_give_a_picture_of_the_size_asked() {
    let mut rng = SimRng::new(SEED + 2);
    for round in 0..200 {
        let img = picture(&mut rng, 40);
        let wild = |rng: &mut SimRng| (unit(rng) * 200.0 - 80.0, unit(rng) * 200.0 - 80.0);
        let corners = [
            wild(&mut rng),
            wild(&mut rng),
            wild(&mut rng),
            wild(&mut rng),
        ];
        let size =
            (rng.below(3) != 0).then(|| (rng.below(60) as usize + 1, rng.below(60) as usize + 1));
        let ctx = format!("seed {:#x}, round {round}, {corners:?}, {size:?}", SEED + 2);
        match rectify(&img, &corners, size, 0, 4096) {
            Ok(out) => {
                assert!(convex(&corners), "{ctx}");
                if let Some((w, h)) = size {
                    assert_eq!((out.w, out.h), (w, h), "{ctx}");
                }
                assert_eq!(out.data.len(), out.w * out.h, "{ctx}");
            }
            Err(e) => {
                assert!(
                    e.starts_with("bad_corners") || e.starts_with("bad_size"),
                    "{ctx}: {e}"
                );
            }
        }
    }
}

#[test]
fn a_transform_of_good_corners_can_be_undone() {
    let mut rng = SimRng::new(SEED + 3);
    let rect = [(0.0, 0.0), (100.0, 0.0), (100.0, 60.0), (0.0, 60.0)];
    let mut done = 0;
    for round in 0..300 {
        let to = quad(&mut rng, 100.0, 60.0);
        if !convex(&to) {
            continue;
        }
        let h =
            Homography::from_points(&rect, &to).unwrap_or_else(|| panic!("round {round}: {to:?}"));
        let back = h
            .inverse()
            .unwrap_or_else(|| panic!("round {round}: no inverse"));
        for (a, b) in rect.iter().zip(to.iter()) {
            let p = h.apply(*a).unwrap();
            assert!(
                (p.0 - b.0).abs() < 1e-5 && (p.1 - b.1).abs() < 1e-5,
                "round {round}"
            );
            let q = back.apply(p).unwrap();
            assert!(
                (q.0 - a.0).abs() < 1e-5 && (q.1 - a.1).abs() < 1e-5,
                "round {round}"
            );
        }
        // a point inside the rectangle goes inside the shape: its place is not outside the box of the corners
        let inside = h.apply((50.0, 30.0)).unwrap();
        let (xs, ys): (Vec<f64>, Vec<f64>) = to.iter().cloned().unzip();
        assert!(
            inside.0 >= xs.iter().cloned().fold(f64::INFINITY, f64::min) - 1e-6,
            "round {round}"
        );
        assert!(
            inside.0 <= xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + 1e-6,
            "round {round}"
        );
        assert!(
            inside.1 >= ys.iter().cloned().fold(f64::INFINITY, f64::min) - 1e-6,
            "round {round}"
        );
        assert!(
            inside.1 <= ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + 1e-6,
            "round {round}"
        );
        done += 1;
    }
    assert!(done > 250, "only {done} of the shapes were convex");
}

#[test]
fn rows_of_pads_with_random_places_give_numbers_that_make_sense() {
    let mut rng = SimRng::new(SEED + 4);
    for round in 0..300 {
        let n = rng.below(9) as usize;
        let parts: Vec<Part> = (0..n)
            .map(|_| Part {
                x_mm: unit(&mut rng) * 10.0 - 5.0,
                y_mm: unit(&mut rng) * 10.0 - 5.0,
                angle_deg: 0.0,
                length_mm: 1.0 + unit(&mut rng),
                width_mm: 0.2 + unit(&mut rng) * 0.5,
                area_mm2: 0.5,
            })
            .collect();
        let s = PadSettings {
            tolerance: 0.01 + unit(&mut rng) * 0.99,
            expected_pitch_mm: (rng.below(2) == 0).then(|| 0.1 + unit(&mut rng) * 3.0),
        };
        let ctx = format!("seed {:#x}, round {round}, {n} pads", SEED + 4);
        match measure_row(&parts, &s) {
            Ok(p) => {
                assert!(n >= 3 && p.count == n, "{ctx}");
                assert!(
                    p.pitch_min_mm <= p.pitch_mm + 1e-9 && p.pitch_mm <= p.pitch_max_mm + 1e-9,
                    "{ctx}"
                );
                assert!(p.spread >= 0.0 && p.row_deviation_mm >= 0.0, "{ctx}");
                assert!(
                    p.row_angle_deg > -90.0 && p.row_angle_deg <= 90.0,
                    "{ctx}: {}",
                    p.row_angle_deg
                );
                assert_eq!(p.passes(), p.reasons.is_empty(), "{ctx}");
            }
            Err(e) => assert!(
                e.starts_with("few_pads") || e.starts_with("pads_overlap"),
                "{ctx}: {e}"
            ),
        }
    }
}

const SAMPLE_CFG: &str = include_str!("../examples/vision.cfg");

#[test]
fn a_configuration_file_with_random_damage_is_read_or_refused_without_a_panic() {
    let mut rng = SimRng::new(SEED + 5);
    let alphabet = b"abcdefghijklmnopqrstuvwxyz_0123456789=.-# \n:/[]\\\"";
    let mut accepted = 0;
    for round in 0..400 {
        let mut text: Vec<u8> = SAMPLE_CFG.bytes().collect();
        for _ in 0..rng.below(6) + 1 {
            let at = rng.below(text.len() as u64) as usize;
            match rng.below(3) {
                0 => text[at] = alphabet[rng.below(alphabet.len() as u64) as usize],
                1 => text.insert(at, alphabet[rng.below(alphabet.len() as u64) as usize]),
                _ => {
                    text.remove(at);
                }
            }
        }
        let text = String::from_utf8_lossy(&text).to_string();
        if let Ok(c) = Config::parse(&text) {
            // what is accepted is valid, and is what it says when it is read again
            assert!(c.validate().is_ok(), "round {round}");
            accepted += 1;
        }
    }
    assert!(accepted > 0, "no damaged file was ever accepted");
    // a file of nothing but noise
    for _ in 0..200 {
        let len = rng.below(80) as usize;
        let noise: String = (0..len)
            .map(|_| alphabet[rng.below(alphabet.len() as u64) as usize] as char)
            .collect();
        let _ = Config::parse(&noise);
    }
}

const SAMPLE_JSON: &str = r#"{"polarity": "light", "threshold": 120, "n": null, "list": [1, {"a": "]"}], "text": "caf\u00e9 \ud83d\ude00", "quality_check": true, "pitch_mm": -1.5e2}"#;

#[test]
fn json_that_is_damaged_is_read_or_refused_without_a_panic() {
    let mut rng = SimRng::new(SEED + 6);
    let alphabet = b"{}[]\":,\\ untrfalse0123456789.-eE\n";
    assert!(parse_object(SAMPLE_JSON).is_ok());
    for round in 0..600 {
        let mut text: Vec<u8> = SAMPLE_JSON.bytes().collect();
        for _ in 0..rng.below(4) + 1 {
            let at = rng.below(text.len() as u64) as usize;
            match rng.below(3) {
                0 => text[at] = alphabet[rng.below(alphabet.len() as u64) as usize],
                1 => text.insert(at, alphabet[rng.below(alphabet.len() as u64) as usize]),
                _ => {
                    text.remove(at);
                }
            }
        }
        // a cut that falls inside a letter with accents is damaged text too
        if rng.below(4) == 0 {
            text.truncate(rng.below(text.len() as u64) as usize);
        }
        let text = String::from_utf8_lossy(&text).to_string();
        if let Ok(pairs) = parse_object(&text) {
            // the keys of what is read are unique, and what the program takes of it is all or nothing
            for (i, p) in pairs.iter().enumerate() {
                assert!(
                    !pairs[i + 1..].iter().any(|q| q.0 == p.0),
                    "round {round}: {text}"
                );
            }
            let before = Config::default();
            match apply_remote(&before, &pairs) {
                Ok((next, done)) => {
                    assert!(next.validate().is_ok(), "round {round}");
                    assert_eq!(next.server_address, before.server_address);
                    assert_eq!(next.server_token, before.server_token);
                    assert!(done.applied.iter().all(|k| !k.starts_with("server_")));
                }
                Err(e) => assert!(!e.is_empty()),
            }
        }
    }
    // deeply nested lists do not use the stack
    let deep = format!("{{\"a\": {}{}}}", "[".repeat(100_000), "]".repeat(100_000));
    assert!(parse_object(&deep).unwrap().is_empty());
}

#[test]
fn http_answers_that_are_cut_or_damaged_are_read_or_refused_without_a_panic() {
    let mut rng = SimRng::new(SEED + 7);
    let good: &[&[u8]] = &[
        b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\n{\"a\": 1}\n",
        b"HTTP/1.1 201 Created\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n{\"a\"\r\n5\r\n: 1}\n\r\n0\r\n\r\n",
    ];
    for round in 0..600 {
        let mut raw = good[rng.below(2) as usize].to_vec();
        match rng.below(3) {
            0 => raw.truncate(rng.below(raw.len() as u64) as usize),
            1 => {
                let at = rng.below(raw.len() as u64) as usize;
                raw[at] = rng.below(256) as u8;
            }
            _ => {
                let at = rng.below(raw.len() as u64) as usize;
                raw.insert(at, rng.below(256) as u8);
            }
        }
        if let Ok(reply) = parse_response(&raw) {
            // whatever is left of the answer is a piece of what came in, never more
            assert!(reply.body.len() <= raw.len() * 3, "round {round}");
        }
    }
    // pure noise
    for _ in 0..300 {
        let len = rng.below(120) as usize;
        let noise: Vec<u8> = (0..len).map(|_| rng.below(256) as u8).collect();
        let _ = parse_response(&noise);
    }
}

#[test]
fn files_of_noise_are_not_pictures() {
    let mut rng = SimRng::new(SEED + 8);
    for round in 0..300 {
        let len = rng.below(200) as usize;
        let mut bytes: Vec<u8> = (0..len).map(|_| rng.below(256) as u8).collect();
        // sometimes a header that says that a picture follows
        if rng.below(2) == 0 {
            let head = format!("P5\n{} {}\n255\n", rng.below(40) + 1, rng.below(40) + 1);
            bytes.splice(0..0, head.into_bytes());
        }
        if let Ok(g) = from_bytes(&bytes) {
            assert_eq!(g.data.len(), g.w * g.h, "round {round}");
        }
    }
}

#[test]
fn the_same_seed_gives_the_same_results() {
    let run = |seed: u64| {
        let mut rng = SimRng::new(seed);
        let img = picture(&mut rng, 48);
        let found = correlate(
            &img,
            &CorrelateSettings {
                diameter: 6.0,
                min_score: 0.4,
                ..CorrelateSettings::default()
            },
        )
        .unwrap_or_default();
        (
            img.data,
            found
                .iter()
                .map(|m| (m.x.to_bits(), m.y.to_bits(), m.score.to_bits()))
                .collect::<Vec<_>>(),
        )
    };
    assert_eq!(run(SEED), run(SEED));
    assert_ne!(run(SEED).0, run(SEED + 1).0);
}

const SAMPLE_LINES: &str = "# tuned on the bench\npolarity = light\nthreshold = 120   # the lamp\nquality_check = yes\npitch_mm = 0.5\n; and a note\nserver_token = stolen\n";

#[test]
fn a_configuration_kept_on_the_server_is_read_or_refused_and_base64_goes_there_and_back() {
    let mut rng = SimRng::new(SEED + 9);
    let alphabet = b"abc =#;\n\r\t0123456789.-_{}\"";
    assert!(parse_config_text(SAMPLE_LINES).is_ok());
    for round in 0..600 {
        let mut text: Vec<u8> = SAMPLE_LINES.bytes().collect();
        for _ in 0..rng.below(4) + 1 {
            let at = rng.below(text.len() as u64) as usize;
            match rng.below(3) {
                0 => text[at] = alphabet[rng.below(alphabet.len() as u64) as usize],
                1 => text.insert(at, alphabet[rng.below(alphabet.len() as u64) as usize]),
                _ => {
                    text.remove(at);
                }
            }
        }
        let text = String::from_utf8_lossy(&text).to_string();
        if let Ok(pairs) = parse_config_text(&text) {
            for (i, p) in pairs.iter().enumerate() {
                assert!(
                    !pairs[i + 1..].iter().any(|q| q.0 == p.0),
                    "round {round}: {text}"
                );
            }
            // what is read is taken all or nothing, and a server never moves the program or changes its token
            let before = Config::default();
            if let Ok((next, done)) = apply_remote(&before, &pairs) {
                assert!(next.validate().is_ok(), "round {round}");
                assert_eq!(next.server_token, before.server_token);
                assert!(done.applied.iter().all(|k| !k.starts_with("server_")));
            }
        }
    }
    // bytes go there and back, and any text is decoded or refused without a panic
    for round in 0..300 {
        let len = rng.below(200) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.below(256) as u8).collect();
        assert_eq!(
            base64_decode(&base64_encode(&bytes)).unwrap(),
            bytes,
            "round {round}"
        );
        let noise: String = (0..rng.below(60))
            .map(|_| (rng.below(95) as u8 + 32) as char)
            .collect();
        if let Some(decoded) = base64_decode(&noise) {
            assert!(decoded.len() <= noise.len(), "round {round}");
        }
    }
}
