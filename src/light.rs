//! Whether the light of a picture is good for measuring from it: how bright it is, how much of it is burnt out, how even it is from one zone to another and whether it fades from one side
//! to the other. This is a check of the picture and nothing else: it does not know the lamp, so it names what is wrong with the picture and never what to buy.
//!
//! The picture is cut into `cells` by `cells` zones, each with its mean grey level. The unevenness is the distance between the brightest and the darkest zone as a share of the mean of
//! the zones, and the tilt along each axis is the mean of one half minus the mean of the other as a share of the mean of the picture. Every limit is a setting (the `light_*` keys),
//! each reason has a code that does not change, and a picture is good only when it has no reason. A picture that is all one grey level has no zones to compare: it is judged by its
//! brightness alone.

use crate::image::Gray;

/// The code of the refusal of a picture with more pixels than the setting allows.
pub const TOO_BIG: &str = "light_too_big";
/// The code of the refusal of a picture too small for its zones.
pub const TOO_SMALL: &str = "light_too_small";

/// Reasons a picture is not good, in the order they are listed.
pub const SATURATED: &str = "light_saturated";
pub const DIM: &str = "light_dim";
pub const BRIGHT: &str = "light_bright";
pub const UNEVEN: &str = "light_uneven";
pub const TILTED: &str = "light_tilted";

/// Every reason, in order.
pub const REASONS: [&str; 5] = [SATURATED, DIM, BRIGHT, UNEVEN, TILTED];

/// The smallest side of a zone in pixels.
pub const MIN_CELL: usize = 4;

/// What is asked of the check.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightSettings {
    /// The zones along each side (2 to 16).
    pub cells: usize,
    /// The grey level from which a pixel counts as burnt out (1 to 255).
    pub saturation_level: u8,
    /// The most pixels that may be burnt out, as a share (0 to 1).
    pub max_saturated: f64,
    /// The mean grey level must be inside this range (0 to 255).
    pub min_mean: f64,
    pub max_mean: f64,
    /// The most that the zones may differ, as a share of their mean (0 to 4).
    pub max_uneven: f64,
    /// The most that one half may differ from the other, as a share of the mean (0 to 2).
    pub max_tilt: f64,
    /// The most pixels of a picture.
    pub max_pixels: usize,
}

impl Default for LightSettings {
    fn default() -> Self {
        LightSettings {
            cells: 4,
            saturation_level: 250,
            max_saturated: 0.02,
            min_mean: 40.0,
            max_mean: 215.0,
            max_uneven: 0.35,
            max_tilt: 0.25,
            max_pixels: 25_000_000,
        }
    }
}

/// Whether the settings make sense; the reason when they do not.
pub fn check(s: &LightSettings) -> Result<(), String> {
    if !(2..=16).contains(&s.cells) {
        return Err("the zones of the light check must be from 2 to 16 on a side".into());
    }
    if s.saturation_level == 0 {
        return Err("the level of saturation must be from 1 to 255".into());
    }
    let share = |v: f64, top: f64| v.is_finite() && (0.0..=top).contains(&v);
    if !share(s.max_saturated, 1.0) {
        return Err("the most saturated share must be from 0 to 1".into());
    }
    if !(share(s.min_mean, 255.0) && share(s.max_mean, 255.0) && s.min_mean <= s.max_mean) {
        return Err(
            "the mean grey levels must be from 0 to 255 with the least not above the most".into(),
        );
    }
    if !share(s.max_uneven, 4.0) {
        return Err("the most unevenness must be from 0 to 4".into());
    }
    if !share(s.max_tilt, 2.0) {
        return Err("the most tilt must be from 0 to 2".into());
    }
    if !(64..=25_000_000).contains(&s.max_pixels) {
        return Err("the most pixels of the light check must be from 64 to 25000000".into());
    }
    Ok(())
}

/// What the check found in a picture.
#[derive(Debug, Clone, PartialEq)]
pub struct LightReport {
    pub width: usize,
    pub height: usize,
    pub mean: f64,
    pub min: u8,
    pub max: u8,
    /// The share of the pixels at or above the level of saturation.
    pub saturated: f64,
    /// The mean of each zone, row after row.
    pub cells: Vec<f64>,
    pub cells_per_side: usize,
    /// The distance between the brightest and the darkest zone as a share of the mean of the zones.
    pub uneven: f64,
    /// The mean of the right half minus the left half, and of the lower half minus the upper half, as a share of the mean of the picture.
    pub tilt_x: f64,
    pub tilt_y: f64,
    /// The codes of what is wrong; empty when the light is good.
    pub reasons: Vec<&'static str>,
}

impl LightReport {
    /// Whether the picture is good to measure from.
    pub fn good(&self) -> bool {
        self.reasons.is_empty()
    }
}

/// Checks the light of a picture.
pub fn analyse(img: &Gray, s: &LightSettings) -> Result<LightReport, String> {
    check(s)?;
    let (w, h) = (img.w, img.h);
    if img.data.len() != w * h || w == 0 || h == 0 {
        return Err(format!("{TOO_SMALL}: the picture is not a picture"));
    }
    if (w as u64) * (h as u64) > s.max_pixels as u64 {
        return Err(format!(
            "{TOO_BIG}: {w} by {h} pixels is more than the {} allowed",
            s.max_pixels
        ));
    }
    let n = s.cells;
    if w < n * MIN_CELL || h < n * MIN_CELL {
        return Err(format!(
            "{TOO_SMALL}: {w} by {h} pixels is too small for {n} by {n} zones of at least {MIN_CELL} pixels"
        ));
    }
    let mut sums = vec![0u64; n * n];
    let mut counts = vec![0u64; n * n];
    let (mut total, mut burnt) = (0u64, 0u64);
    let (mut left, mut right, mut upper, mut lower) = (0u64, 0u64, 0u64, 0u64);
    let (mut nl, mut nr, mut nu, mut nd) = (0u64, 0u64, 0u64, 0u64);
    let (mut lo, mut hi) = (255u8, 0u8);
    for y in 0..h {
        let cy = y * n / h;
        for x in 0..w {
            let v = img.data[y * w + x];
            let cell = cy * n + x * n / w;
            sums[cell] += u64::from(v);
            counts[cell] += 1;
            total += u64::from(v);
            if v >= s.saturation_level {
                burnt += 1;
            }
            lo = lo.min(v);
            hi = hi.max(v);
            if x < w / 2 {
                left += u64::from(v);
                nl += 1;
            } else if x >= w - w / 2 {
                right += u64::from(v);
                nr += 1;
            }
            if y < h / 2 {
                upper += u64::from(v);
                nu += 1;
            } else if y >= h - h / 2 {
                lower += u64::from(v);
                nd += 1;
            }
        }
    }
    let pixels = (w * h) as f64;
    let mean = total as f64 / pixels;
    let cells: Vec<f64> = sums
        .iter()
        .zip(&counts)
        .map(|(&a, &c)| a as f64 / c.max(1) as f64)
        .collect();
    let cell_mean = cells.iter().sum::<f64>() / cells.len() as f64;
    let spread = cells.iter().cloned().fold(f64::MIN, f64::max)
        - cells.iter().cloned().fold(f64::MAX, f64::min);
    let (uneven, tilt_x, tilt_y) = if mean < 1.0 {
        (0.0, 0.0, 0.0)
    } else {
        (
            spread / cell_mean.max(1.0),
            (right as f64 / nr.max(1) as f64 - left as f64 / nl.max(1) as f64) / mean,
            (lower as f64 / nd.max(1) as f64 - upper as f64 / nu.max(1) as f64) / mean,
        )
    };
    let saturated = burnt as f64 / pixels;
    let mut reasons = Vec::new();
    if saturated > s.max_saturated {
        reasons.push(SATURATED);
    }
    if mean < s.min_mean {
        reasons.push(DIM);
    }
    if mean > s.max_mean {
        reasons.push(BRIGHT);
    }
    if uneven > s.max_uneven {
        reasons.push(UNEVEN);
    }
    if tilt_x.abs() > s.max_tilt || tilt_y.abs() > s.max_tilt {
        reasons.push(TILTED);
    }
    Ok(LightReport {
        width: w,
        height: h,
        mean,
        min: lo,
        max: hi,
        saturated,
        cells,
        cells_per_side: n,
        uneven,
        tilt_x,
        tilt_y,
        reasons,
    })
}

/// The report as JSON with keys that do not change.
pub fn to_json(r: &LightReport) -> String {
    let cells: Vec<String> = r.cells.iter().map(|c| format!("{c:.2}")).collect();
    let reasons: Vec<String> = r.reasons.iter().map(|c| format!("\"{c}\"")).collect();
    format!(
        "{{\"command\":\"light-check\",\"width\":{},\"height\":{},\"mean\":{:.3},\"min\":{},\"max\":{},\"saturated\":{:.5},\"cells_per_side\":{},\"cells\":[{}],\"uneven\":{:.4},\"tilt_x\":{:.4},\"tilt_y\":{:.4},\"good\":{},\"reasons\":[{}]}}\n",
        r.width,
        r.height,
        r.mean,
        r.min,
        r.max,
        r.saturated,
        r.cells_per_side,
        cells.join(","),
        r.uneven,
        r.tilt_x,
        r.tilt_y,
        r.good(),
        reasons.join(",")
    )
}

/// The report as CSV: a header and one row of the figures, then the reasons joined with `+` (or `-`).
pub fn to_csv(r: &LightReport) -> String {
    let why = if r.reasons.is_empty() {
        "-".to_string()
    } else {
        r.reasons.join("+")
    };
    format!(
        "width,height,mean,min,max,saturated,uneven,tilt_x,tilt_y,good,reasons\n{},{},{:.3},{},{},{:.5},{:.4},{:.4},{:.4},{},{}\n",
        r.width,
        r.height,
        r.mean,
        r.min,
        r.max,
        r.saturated,
        r.uneven,
        r.tilt_x,
        r.tilt_y,
        r.good(),
        why
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(w: usize, h: usize, v: u8) -> Gray {
        Gray::new(w, h, vec![v; w * h]).unwrap()
    }

    #[test]
    fn an_even_picture_of_a_middle_grey_is_good() {
        let r = analyse(&flat(64, 48, 128), &LightSettings::default()).unwrap();
        assert!(r.good(), "{:?}", r.reasons);
        assert!((r.mean - 128.0).abs() < 1e-9);
        assert_eq!(r.cells.len(), 16);
        assert_eq!(r.uneven, 0.0);
    }

    #[test]
    fn a_dark_a_bright_and_a_burnt_picture_say_why() {
        let s = LightSettings::default();
        assert_eq!(analyse(&flat(64, 48, 10), &s).unwrap().reasons, [DIM]);
        let r = analyse(&flat(64, 48, 252), &s).unwrap();
        assert_eq!(r.reasons, [SATURATED, BRIGHT]);
    }

    #[test]
    fn a_fade_from_one_side_is_uneven_and_tilted() {
        let (w, h) = (64usize, 48usize);
        let data: Vec<u8> = (0..w * h).map(|i| (60 + (i % w) * 2) as u8).collect();
        let r = analyse(&Gray::new(w, h, data).unwrap(), &LightSettings::default()).unwrap();
        assert!(r.reasons.contains(&UNEVEN) && r.reasons.contains(&TILTED));
        assert!(r.tilt_x > 0.3 && r.tilt_y.abs() < 1e-9);
    }

    #[test]
    fn a_picture_that_is_too_big_or_too_small_is_refused_with_its_code() {
        let s = LightSettings::default();
        assert!(analyse(&flat(8, 8, 100), &s)
            .unwrap_err()
            .starts_with(TOO_SMALL));
        let small = LightSettings {
            max_pixels: 64,
            ..s
        };
        assert!(analyse(&flat(16, 16, 100), &small)
            .unwrap_err()
            .starts_with(TOO_BIG));
    }

    #[test]
    fn settings_that_make_no_sense_are_refused() {
        let d = LightSettings::default();
        for bad in [
            LightSettings { cells: 1, ..d },
            LightSettings { cells: 17, ..d },
            LightSettings {
                saturation_level: 0,
                ..d
            },
            LightSettings {
                max_saturated: 2.0,
                ..d
            },
            LightSettings {
                min_mean: 200.0,
                max_mean: 100.0,
                ..d
            },
            LightSettings {
                max_uneven: f64::NAN,
                ..d
            },
            LightSettings { max_tilt: 3.0, ..d },
            LightSettings { max_pixels: 1, ..d },
        ] {
            assert!(check(&bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_json_and_the_csv_carry_the_same_verdict() {
        let r = analyse(&flat(64, 48, 10), &LightSettings::default()).unwrap();
        assert!(to_json(&r).contains("\"good\":false,\"reasons\":[\"light_dim\"]"));
        assert!(to_csv(&r).ends_with("false,light_dim\n"));
    }
}
