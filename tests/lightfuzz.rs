//! The check of the light, the search for the holes of a tape and the card of calibration against pictures and settings of every kind, with a fixed seed: whatever is asked, the answer is a
//! report whose numbers are inside their ranges or an error with a code (or the reason of a setting), never a panic, and a picture with a hole in its data or numbers that are not numbers
//! are refused. The pictures that the reports are about are drawn by the tests.

use eh3d_vision_studio::dotcard::{self, CardSettings, BAD_SETTING, TOO_BIG as CARD_TOO_BIG};
use eh3d_vision_studio::image::Gray;
use eh3d_vision_studio::lang::Lang;
use eh3d_vision_studio::light::{self, LightSettings, REASONS as LIGHT_REASONS};
use eh3d_vision_studio::tape::{self, TapeSettings, REASONS as TAPE_REASONS};

struct Rng(u64, bool);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// A number that is often reasonable and sometimes not a number or absurd.
    fn odd(&mut self, lo: f64, hi: f64) -> f64 {
        if !self.1 {
            return lo + (hi - lo) * self.unit();
        }
        match self.below(12) {
            0 => f64::NAN,
            1 => f64::INFINITY,
            2 => -1.0,
            3 => 1.0e18,
            _ => lo + (hi - lo) * self.unit(),
        }
    }
}

fn noise(rng: &mut Rng, w: usize, h: usize, base: u8, spread: usize) -> Gray {
    let data = (0..w * h)
        .map(|_| (usize::from(base) + rng.below(spread + 1)).min(255) as u8)
        .collect();
    Gray::new(w, h, data).unwrap()
}

#[test]
fn the_light_of_any_picture_with_any_settings_is_a_report_or_a_refusal() {
    let mut rng = Rng(0xC0FF_EE11, false);
    let (mut ok, mut refused) = (0usize, 0usize);
    for round in 0..600 {
        rng.1 = round % 2 == 1;
        let (w, h) = (
            if rng.1 { 1 } else { 16 } + rng.below(90),
            if rng.1 { 1 } else { 16 } + rng.below(70),
        );
        let (base, spread) = (rng.below(200) as u8, rng.below(60));
        let img = noise(&mut rng, w, h, base, spread);
        let s = LightSettings {
            cells: if rng.1 {
                rng.below(20)
            } else {
                2 + rng.below(3)
            },
            saturation_level: if rng.1 {
                rng.below(256) as u8
            } else {
                200 + rng.below(56) as u8
            },
            max_saturated: rng.odd(0.0, 1.0),
            min_mean: rng.odd(0.0, 255.0),
            max_mean: rng.odd(0.0, 255.0),
            max_uneven: rng.odd(0.0, 4.0),
            max_tilt: rng.odd(0.0, 2.0),
            max_pixels: if rng.1 {
                [0usize, 63, 64, 100, 5000, 25_000_000, 25_000_001][rng.below(7)]
            } else {
                25_000_000
            },
        };
        match light::analyse(&img, &s) {
            Ok(r) => {
                ok += 1;
                assert!((0.0..=1.0).contains(&r.saturated), "{r:?}");
                assert!((0.0..=255.0).contains(&r.mean) && r.min <= r.max);
                assert!(r.uneven >= 0.0 && r.tilt_x.is_finite() && r.tilt_y.is_finite());
                assert_eq!(r.cells.len(), r.cells_per_side * r.cells_per_side);
                assert!(r.reasons.iter().all(|c| LIGHT_REASONS.contains(c)));
                assert_eq!(r.good(), r.reasons.is_empty());
                assert!(light::to_json(&r).starts_with("{\"command\":\"light-check\""));
            }
            Err(e) => {
                refused += 1;
                assert!(!e.is_empty(), "{s:?}");
            }
        }
    }
    assert!(
        ok > 150 && refused > 150,
        "{ok} reports and {refused} refusals"
    );
}

#[test]
fn a_picture_whose_data_does_not_fit_its_size_is_refused_by_the_light_check() {
    let mut img = Gray::blank(32, 32, 100).unwrap();
    img.data.truncate(100);
    let e = light::analyse(&img, &LightSettings::default()).unwrap_err();
    assert!(e.starts_with(light::TOO_SMALL), "{e}");
}

#[test]
fn the_holes_of_any_picture_with_any_scale_are_a_report_or_a_refusal() {
    let mut rng = Rng(0xFEED_5EED, false);
    let (mut ok, mut refused) = (0usize, 0usize);
    for round in 0..120 {
        rng.1 = round % 2 == 1;
        let (w, h) = (60 + rng.below(100), 40 + rng.below(80));
        let mut img = noise(&mut rng, w, h, 150, 30);
        for _ in 0..rng.below(14) {
            img.disc(
                rng.unit() * w as f64,
                rng.unit() * h as f64,
                1.0 + rng.unit() * 9.0,
                rng.below(60) as u8,
            );
        }
        let scale = match (rng.1, round % 6) {
            (true, 0) => 0.0,
            (true, 1) => f64::NAN,
            (true, 3) => 1.0e6,
            _ => 0.02 + rng.unit() * 0.2,
        };
        let s = TapeSettings {
            hole_mm: rng.odd(0.5, 2.5),
            pitch_mm: rng.odd(3.0, 8.0),
            tolerance_mm: rng.odd(0.01, 0.5),
            line_tolerance_mm: rng.odd(0.01, 0.5),
            size_tolerance: rng.odd(0.0, 1.0),
            min_holes: if rng.1 {
                rng.below(8)
            } else {
                2 + rng.below(4)
            },
            max_holes: if rng.1 {
                [1usize, 3, 50, 400, 20_000][rng.below(5)]
            } else {
                400
            },
            pocket_pitch_mm: rng.odd(1.0, 10.0),
            pocket_along_mm: rng.odd(-5.0, 5.0),
            pocket_across_mm: rng.odd(-5.0, 5.0),
            polarity: eh3d_vision_studio::fiducial::Polarity::Dark,
            max_pixels: if rng.1 {
                [10usize, 64, 100_000, 25_000_000][rng.below(4)]
            } else {
                25_000_000
            },
        };
        match tape::locate(&img, scale, &s) {
            Ok(r) => {
                ok += 1;
                assert!(r.holes.len() <= s.max_holes, "{r:?}");
                assert!(r.reasons.iter().all(|c| TAPE_REASONS.contains(c)));
                assert!(r.pockets.len() <= s.max_holes);
                assert!(r.pockets.iter().all(|p| p.0.is_finite() && p.1.is_finite()));
                if let Some(i) = r.pick {
                    assert!(i < r.pockets.len());
                }
                assert!(tape::to_json(&r).starts_with("{\"command\":\"tape-locate\""));
            }
            Err(e) => {
                refused += 1;
                assert!(!e.is_empty());
            }
        }
    }
    assert!(
        ok > 30 && refused > 20,
        "{ok} reports and {refused} refusals"
    );
}

#[test]
fn the_card_of_any_settings_is_a_drawing_with_a_dot_for_each_place_or_a_refusal_with_its_code() {
    let mut rng = Rng(0xCA2D_0001, false);
    let (mut ok, mut refused) = (0usize, 0usize);
    for round in 0..800 {
        rng.1 = round % 2 == 1;
        let s = CardSettings {
            columns: if rng.1 {
                rng.below(45)
            } else {
                3 + rng.below(8)
            },
            rows: if rng.1 {
                rng.below(45)
            } else {
                3 + rng.below(8)
            },
            pitch_mm: rng.odd(if rng.1 { 1.0 } else { 6.0 }, 40.0),
            dot_mm: rng.odd(
                if rng.1 { 0.1 } else { 0.5 },
                if rng.1 { 12.0 } else { 3.0 },
            ),
            margin_mm: rng.odd(0.0, 30.0),
            paper_w_mm: rng.odd(if rng.1 { 40.0 } else { 300.0 }, 600.0),
            paper_h_mm: rng.odd(if rng.1 { 40.0 } else { 300.0 }, 600.0),
        };
        let lang = Lang::ALL[rng.below(8)];
        match dotcard::svg(&s, lang) {
            Ok(svg) => {
                ok += 1;
                assert_eq!(svg.matches("<circle").count(), s.columns * s.rows);
                assert!(svg.ends_with("</svg>\n") && !svg.contains("NaN") && !svg.contains("inf"));
                assert_eq!(svg, dotcard::svg(&s, lang).unwrap());
                // every dot is on the page
                for (x, y) in dotcard::dots(&s).unwrap() {
                    assert!(x > 0.0 && y > 0.0 && x < s.paper_w_mm && y < s.paper_h_mm);
                }
            }
            Err(e) => {
                refused += 1;
                assert!(
                    e.starts_with(BAD_SETTING) || e.starts_with(CARD_TOO_BIG),
                    "{e}"
                );
            }
        }
    }
    assert!(
        ok > 100 && refused > 100,
        "{ok} cards and {refused} refusals"
    );
}
