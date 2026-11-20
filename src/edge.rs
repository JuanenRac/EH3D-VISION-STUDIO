//! An edge to a fraction of a pixel, with how sure the position is. A line is laid across the edge in the picture (from a point on one side to a point on the other); the picture is read
//! along it with bilinear interpolation, the two flat grounds at its ends give the grey levels on each side and the noise, and the edge is where the profile crosses the level half
//! way between them, found between two samples by interpolation. The uncertainty is one standard deviation of that position: the noise of the grounds (with the rounding of the grey
//! levels to whole numbers added) divided by the slope of the profile where it crosses. It is a model of the noise of this picture, not the calibration of a camera.
//!
//! The profile between two pixels is the straight line between them, so an edge sharper than about a pixel leans by up to 0.05 px toward the nearer sample; an edge of a normal lens
//! (a transition of two pixels or more) does not.
//!
//! Pixels are numbered from the top left corner and their centres lie on whole numbers, as everywhere else in this program. The line should start and end on flat ground: the first and
//! the last fifth of the profile are taken as the two grounds.

use crate::image::Gray;
use crate::measure::fixed;

/// The name of the format and its version, in the JSON file.
pub const FORMAT: &str = "eh3d-vision-edge";
pub const FORMAT_VERSION: u32 = 1;
pub const CSV_HEADER: &str =
    "position_px,x,y,rising,contrast,slope,width_px,noise,uncertainty_px,samples,verdict";
/// The code of a line with no edge on it (the two grounds are about the same grey level).
pub const FLAT: &str = "edge_flat";
/// The code of a line that is too short to read, or that leaves the picture.
pub const BAD_LINE: &str = "edge_line";
/// The code of a profile that does not cross the middle level.
pub const NOT_FOUND: &str = "edge_not_found";
/// The least difference between the two grounds, in grey levels, for there to be an edge.
pub const MIN_CONTRAST: f64 = 8.0;
/// The shortest line, in pixels, and the most samples of a profile.
pub const MIN_LENGTH_PX: f64 = 4.0;
pub const MAX_SAMPLES: usize = 200_000;

/// What is asked of the reading.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeSettings {
    /// The distance between two samples along the line, in pixels (0.05 to 1).
    pub step_px: f64,
    /// The largest uncertainty of the position that the verdict accepts, in pixels (more than 0, at most 5).
    pub max_uncertainty_px: f64,
}

impl Default for EdgeSettings {
    fn default() -> Self {
        EdgeSettings {
            step_px: 0.25,
            max_uncertainty_px: 0.25,
        }
    }
}

/// Whether the settings make sense; the reason, naming the key, when they do not.
pub fn check(s: &EdgeSettings) -> Result<(), String> {
    if !(s.step_px.is_finite() && (0.05..=1.0).contains(&s.step_px)) {
        return Err("edge_step_px must be from 0.05 to 1".into());
    }
    if !(s.max_uncertainty_px.is_finite()
        && s.max_uncertainty_px > 0.0
        && s.max_uncertainty_px <= 5.0)
    {
        return Err("edge_max_uncertainty_px must be more than 0 and at most 5".into());
    }
    Ok(())
}

/// What was read across an edge.
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    /// The distance from the start of the line to the edge, in pixels.
    pub position_px: f64,
    /// The place of the edge in the picture.
    pub x: f64,
    pub y: f64,
    /// Whether the grey level rises from the start of the line to its end.
    pub rising: bool,
    /// The difference between the two grounds, in grey levels, and the slope of the profile where it crosses the middle level, in grey levels per pixel (always positive).
    pub contrast: f64,
    pub slope: f64,
    /// The width of the transition from 10 to 90 percent, in pixels (a sharp edge of a sharp lens is about one pixel; more means blur).
    pub width_px: f64,
    /// The noise of the grounds, in grey levels (standard deviation, the rounding of 8 bits not included).
    pub noise: f64,
    /// One standard deviation of the position, in pixels.
    pub uncertainty_px: f64,
    /// How many samples the profile has.
    pub samples: usize,
    /// Whether the uncertainty is within the limit that was asked.
    pub sure: bool,
}

/// The grey level of the picture at a place between pixels (bilinear); the place must be inside the picture.
fn sample(img: &Gray, x: f64, y: f64) -> f64 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (ix, iy) = (x0 as usize, y0 as usize);
    let ix1 = (ix + 1).min(img.w - 1);
    let iy1 = (iy + 1).min(img.h - 1);
    let v = |xx: usize, yy: usize| f64::from(img.data[yy * img.w + xx]);
    let top = v(ix, iy) * (1.0 - fx) + v(ix1, iy) * fx;
    let bottom = v(ix, iy1) * (1.0 - fx) + v(ix1, iy1) * fx;
    top * (1.0 - fy) + bottom * fy
}

/// The profile along a line: the samples, and the distance between two of them (the line is covered from end to end, so this is a little different from the wanted step).
pub fn profile(
    img: &Gray,
    from: (f64, f64),
    to: (f64, f64),
    step_px: f64,
) -> Result<(Vec<f64>, f64), String> {
    if img.data.len() != img.w * img.h || img.w == 0 || img.h == 0 {
        return Err(format!("{BAD_LINE}: the picture is empty"));
    }
    let inside = |p: (f64, f64)| {
        p.0.is_finite()
            && p.1.is_finite()
            && p.0 >= 0.0
            && p.1 >= 0.0
            && p.0 <= (img.w - 1) as f64
            && p.1 <= (img.h - 1) as f64
    };
    if !inside(from) || !inside(to) {
        return Err(format!(
            "{BAD_LINE}: both ends of the line must be inside the picture (0 to {} across, 0 to {} down)",
            img.w - 1,
            img.h - 1
        ));
    }
    if !(step_px.is_finite() && step_px > 0.0) {
        return Err(format!("{BAD_LINE}: the step must be more than 0"));
    }
    let length = (to.0 - from.0).hypot(to.1 - from.1);
    if length < MIN_LENGTH_PX {
        return Err(format!(
            "{BAD_LINE}: the line is {length:.1} pixels long; it needs at least {MIN_LENGTH_PX}"
        ));
    }
    let n = (length / step_px).floor() as usize + 1;
    if n > MAX_SAMPLES {
        return Err(format!(
            "{BAD_LINE}: {n} samples are too many (the most is {MAX_SAMPLES}); use a larger step"
        ));
    }
    let n = n.max(8);
    let eff = length / (n - 1) as f64;
    let samples = (0..n)
        .map(|i| {
            let t = i as f64 / (n - 1) as f64;
            sample(
                img,
                from.0 + (to.0 - from.0) * t,
                from.1 + (to.1 - from.1) * t,
            )
        })
        .collect();
    Ok((samples, eff))
}

/// The mean of the grey levels and the sum of the squares of what lies around it.
fn mean_and_spread(v: &[f64]) -> (f64, f64) {
    let m = v.iter().sum::<f64>() / v.len() as f64;
    (m, v.iter().map(|x| (x - m) * (x - m)).sum())
}

/// The edge in a profile whose samples are `step` pixels apart. See the module for what is assumed.
pub fn locate(p: &[f64], step: f64, s: &EdgeSettings) -> Result<Edge, String> {
    check(s)?;
    if p.len() < 8 || !(step.is_finite() && step > 0.0) || p.iter().any(|v| !v.is_finite()) {
        return Err(format!(
            "{BAD_LINE}: the profile is too short or has no numbers"
        ));
    }
    let m = (p.len() / 5).max(2);
    let (start, ss_a) = mean_and_spread(&p[..m]);
    let (end, ss_b) = mean_and_spread(&p[p.len() - m..]);
    let contrast = (end - start).abs();
    if contrast < MIN_CONTRAST {
        return Err(format!(
            "{FLAT}: the two ends of the line differ by {contrast:.1} grey levels; an edge needs {MIN_CONTRAST}"
        ));
    }
    let rising = end > start;
    let noise = ((ss_a + ss_b) / (2 * m - 2) as f64).sqrt();
    let level = (start + end) / 2.0;
    // where the steepest rise (in the direction of the edge) is, to choose among crossings of a noisy profile
    let sign = if rising { 1.0 } else { -1.0 };
    let mut steepest = 1usize;
    let mut best = f64::MIN;
    for i in 1..p.len() - 1 {
        let g = sign * (p[i + 1] - p[i - 1]);
        if g > best {
            best = g;
            steepest = i;
        }
    }
    // the crossing of the middle level that is nearest to it, by linear interpolation between two samples
    let mut at: Option<(f64, usize)> = None;
    for j in 0..p.len() - 1 {
        let (a, b) = (p[j] - level, p[j + 1] - level);
        if a == b || (a > 0.0 && b > 0.0) || (a < 0.0 && b < 0.0) {
            continue;
        }
        let t = j as f64 + a.abs() / (a - b).abs();
        let near = (t - steepest as f64).abs();
        if at.is_none_or(|(u, _)| near < (u - steepest as f64).abs()) {
            at = Some((t, j));
        }
    }
    let Some((t, j)) = at else {
        return Err(format!(
            "{NOT_FOUND}: the profile does not cross the middle level"
        ));
    };
    // the slope where it crosses, over half a pixel on each side
    let w = ((0.5 / step).round() as usize).max(1);
    let (lo, hi) = (j.saturating_sub(w - 1), (j + w).min(p.len() - 1));
    let slope = ((p[hi] - p[lo]) / ((hi - lo) as f64 * step)) * sign;
    if slope <= 0.0 {
        return Err(format!(
            "{FLAT}: the profile does not rise across the middle level"
        ));
    }
    // the width from 10 to 90 percent of the way, taken on the line that has the slope and passes through the crossing
    let width_px = 0.8 * contrast / slope;
    // one grey level of rounding is a uniform error of 1/sqrt(12) of a level
    let sigma = (noise * noise + 1.0 / 12.0).sqrt();
    let uncertainty_px = sigma / slope;
    Ok(Edge {
        position_px: t * step,
        x: 0.0,
        y: 0.0,
        rising,
        contrast,
        slope,
        width_px,
        noise,
        uncertainty_px,
        samples: p.len(),
        sure: uncertainty_px <= s.max_uncertainty_px,
    })
}

/// The edge across a line of the picture, from `from` to `to` (pixels).
pub fn measure(
    img: &Gray,
    from: (f64, f64),
    to: (f64, f64),
    s: &EdgeSettings,
) -> Result<Edge, String> {
    check(s)?;
    let (p, step) = profile(img, from, to, s.step_px)?;
    let mut e = locate(&p, step, s)?;
    let length = (to.0 - from.0).hypot(to.1 - from.1);
    let k = e.position_px / length;
    e.x = from.0 + (to.0 - from.0) * k;
    e.y = from.1 + (to.1 - from.1) * k;
    Ok(e)
}

fn verdict(e: &Edge) -> &'static str {
    if e.sure {
        "sure"
    } else {
        "unsure"
    }
}

/// The reading as CSV: the line `CSV_HEADER` and one line with the numbers and the verdict (`sure` or `unsure`).
pub fn to_csv(e: &Edge) -> String {
    format!(
        "{CSV_HEADER}\n{},{},{},{},{},{},{},{},{},{},{}\n",
        fixed(e.position_px, 4),
        fixed(e.x, 4),
        fixed(e.y, 4),
        e.rising,
        fixed(e.contrast, 2),
        fixed(e.slope, 2),
        fixed(e.width_px, 3),
        fixed(e.noise, 3),
        fixed(e.uncertainty_px, 4),
        e.samples,
        verdict(e)
    )
}

/// The reading as JSON: an object with `format`, `version`, `unit` ("px"), the numbers of the columns of the CSV file and the `verdict`.
pub fn to_json(e: &Edge) -> String {
    format!(
        "{{\n  \"format\": \"{FORMAT}\",\n  \"version\": {FORMAT_VERSION},\n  \"unit\": \"px\",\n  \"position_px\": {},\n  \"x\": {},\n  \"y\": {},\n  \"rising\": {},\n  \"contrast\": {},\n  \"slope\": {},\n  \"width_px\": {},\n  \"noise\": {},\n  \"uncertainty_px\": {},\n  \"samples\": {},\n  \"verdict\": \"{}\"\n}}\n",
        fixed(e.position_px, 4),
        fixed(e.x, 4),
        fixed(e.y, 4),
        e.rising,
        fixed(e.contrast, 2),
        fixed(e.slope, 2),
        fixed(e.width_px, 3),
        fixed(e.noise, 3),
        fixed(e.uncertainty_px, 4),
        e.samples,
        verdict(e)
    )
}

/// A line of text for the command line.
pub fn summary(e: &Edge, max_uncertainty_px: f64) -> String {
    format!(
        "edge at {:.3} px along the line, at ({:.3}, {:.3}); uncertainty {:.3} px (one standard deviation, the limit is {max_uncertainty_px}); contrast {:.1}, width {:.2} px, noise {:.2} grey levels; {}",
        e.position_px,
        e.x,
        e.y,
        e.uncertainty_px,
        e.contrast,
        e.width_px,
        e.noise,
        verdict(e)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The error function (Abramowitz and Stegun 7.1.26, an error below 2e-7).
    fn erf(x: f64) -> f64 {
        let t = 1.0 / (1.0 + 0.327_591_1 * x.abs());
        let poly = t
            * (0.254_829_592
                + t * (-0.284_496_736
                    + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
        (1.0 - poly * (-x * x).exp()).copysign(x)
    }

    /// A vertical edge at `x0` between the grey levels `a` (left) and `b` (right), blurred as a lens blurs it (a Gaussian of half a pixel), plus noise of the given amplitude from a
    /// generator that reads no clock.
    fn edge_picture(x0: f64, a: f64, b: f64, noise: f64, seed: u64) -> Gray {
        let (w, h) = (48usize, 24usize);
        let mut state = seed | 1;
        let mut data = Vec::with_capacity(w * h);
        for _y in 0..h {
            for x in 0..w {
                let cover = 0.5 * (1.0 + erf((x as f64 - x0) / (0.5 * std::f64::consts::SQRT_2)));
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let r = (state % 20001) as f64 / 10000.0 - 1.0;
                let v = a + (b - a) * cover + noise * r;
                data.push(v.round().clamp(0.0, 255.0) as u8);
            }
        }
        Gray::new(w, h, data).unwrap()
    }

    #[test]
    fn a_sharp_edge_is_found_to_a_fraction_of_a_pixel() {
        for x0 in [20.0, 20.3, 20.5, 20.8, 25.25] {
            let g = edge_picture(x0, 40.0, 210.0, 0.0, 1);
            let e = measure(&g, (6.0, 12.0), (40.0, 12.0), &EdgeSettings::default()).unwrap();
            let found = 6.0 + e.position_px;
            assert!((found - x0).abs() < 0.06, "{x0}: {found}");
            assert!(e.rising);
            assert!((e.contrast - 170.0).abs() < 1.5, "{}", e.contrast);
            assert!(e.sure, "{}", e.uncertainty_px);
            assert!((e.y - 12.0).abs() < 1e-9);
            assert!((e.x - found).abs() < 1e-6);
            // a sharp edge: the width is a little over one pixel (2.56 times the half pixel of the blur)
            assert!(e.width_px > 0.8 && e.width_px < 1.8, "{}", e.width_px);
        }
    }

    #[test]
    fn the_direction_of_the_line_does_not_matter() {
        let g = edge_picture(24.4, 200.0, 30.0, 0.0, 1);
        let forward = measure(&g, (8.0, 10.0), (40.0, 10.0), &EdgeSettings::default()).unwrap();
        let backward = measure(&g, (40.0, 10.0), (8.0, 10.0), &EdgeSettings::default()).unwrap();
        assert!(!forward.rising && backward.rising);
        assert!((forward.x - 24.4).abs() < 0.03 && (backward.x - 24.4).abs() < 0.03);
        // a slanted line across the same edge finds the same place on it
        let slanted = measure(&g, (14.0, 4.0), (34.0, 20.0), &EdgeSettings::default()).unwrap();
        assert!((slanted.x - 24.4).abs() < 0.08, "{}", slanted.x);
        assert!(slanted.position_px > 0.0);
    }

    #[test]
    fn noise_raises_the_uncertainty_and_the_error_stays_within_it() {
        let quiet = measure(
            &edge_picture(22.6, 40.0, 210.0, 0.0, 3),
            (6.0, 8.0),
            (40.0, 8.0),
            &EdgeSettings::default(),
        )
        .unwrap();
        let mut worst: f64 = 0.0;
        let mut noisy_unc = 0.0;
        for seed in 1..=60 {
            let g = edge_picture(22.6, 40.0, 210.0, 12.0, seed);
            let e = measure(&g, (6.0, 8.0), (40.0, 8.0), &EdgeSettings::default()).unwrap();
            noisy_unc = e.uncertainty_px;
            let err = (6.0 + e.position_px - 22.6).abs();
            worst = worst.max(err / e.uncertainty_px);
            assert!(e.noise > 3.0 && e.noise < 12.0, "{}", e.noise);
        }
        assert!(
            noisy_unc > 3.0 * quiet.uncertainty_px,
            "{noisy_unc} {}",
            quiet.uncertainty_px
        );
        // the error of any of the sixty pictures is not beyond five standard deviations
        assert!(worst < 5.0, "{worst}");
    }

    #[test]
    fn a_blurred_edge_is_wide_and_less_sure() {
        let mut g = edge_picture(24.0, 40.0, 210.0, 2.0, 7);
        for _ in 0..3 {
            let mut next = g.clone();
            for y in 0..g.h {
                for x in 1..g.w - 1 {
                    let i = y * g.w + x;
                    next.data[i] = ((u32::from(g.data[i - 1])
                        + 2 * u32::from(g.data[i])
                        + u32::from(g.data[i + 1]))
                        / 4) as u8;
                }
            }
            g = next;
        }
        let sharp = measure(
            &edge_picture(24.0, 40.0, 210.0, 2.0, 7),
            (6.0, 12.0),
            (42.0, 12.0),
            &EdgeSettings::default(),
        )
        .unwrap();
        let soft = measure(&g, (6.0, 12.0), (42.0, 12.0), &EdgeSettings::default()).unwrap();
        assert!(
            soft.width_px > 1.5 * sharp.width_px,
            "{} {}",
            soft.width_px,
            sharp.width_px
        );
        assert!(soft.uncertainty_px > sharp.uncertainty_px);
        assert!((6.0 + soft.position_px - 24.0).abs() < 0.2);
    }

    #[test]
    fn a_limit_on_the_uncertainty_decides_the_verdict() {
        let g = edge_picture(24.0, 40.0, 210.0, 10.0, 5);
        let tight = EdgeSettings {
            max_uncertainty_px: 0.001,
            ..EdgeSettings::default()
        };
        let e = measure(&g, (6.0, 12.0), (42.0, 12.0), &tight).unwrap();
        assert!(!e.sure);
        assert!(to_csv(&e).trim_end().ends_with("unsure"));
        assert!(to_json(&e).contains("\"verdict\": \"unsure\""));
        let loose = EdgeSettings {
            max_uncertainty_px: 5.0,
            ..EdgeSettings::default()
        };
        assert!(measure(&g, (6.0, 12.0), (42.0, 12.0), &loose).unwrap().sure);
    }

    #[test]
    fn a_flat_line_a_bad_line_and_bad_settings_are_refused_with_their_code() {
        let flat = Gray::blank(48, 24, 100).unwrap();
        let e = measure(&flat, (6.0, 12.0), (40.0, 12.0), &EdgeSettings::default()).unwrap_err();
        assert!(e.starts_with(FLAT), "{e}");
        let g = edge_picture(24.0, 40.0, 210.0, 0.0, 1);
        for (a, b) in [
            ((6.0, 12.0), (6.5, 12.0)),
            ((-1.0, 12.0), (40.0, 12.0)),
            ((6.0, 12.0), (48.0, 12.0)),
            ((6.0, 24.0), (40.0, 12.0)),
            ((f64::NAN, 1.0), (40.0, 12.0)),
        ] {
            let e = measure(&g, a, b, &EdgeSettings::default()).unwrap_err();
            assert!(e.starts_with(BAD_LINE), "{a:?} {b:?}: {e}");
        }
        // the line ends on the same side of the edge: it is not crossed
        let e = measure(&g, (4.0, 12.0), (14.0, 12.0), &EdgeSettings::default()).unwrap_err();
        assert!(e.starts_with(FLAT), "{e}");
        for bad in [
            EdgeSettings {
                step_px: 0.0,
                ..EdgeSettings::default()
            },
            EdgeSettings {
                step_px: 1.5,
                ..EdgeSettings::default()
            },
            EdgeSettings {
                step_px: f64::NAN,
                ..EdgeSettings::default()
            },
            EdgeSettings {
                max_uncertainty_px: 0.0,
                ..EdgeSettings::default()
            },
            EdgeSettings {
                max_uncertainty_px: 6.0,
                ..EdgeSettings::default()
            },
        ] {
            assert!(check(&bad).is_err(), "{bad:?}");
            assert!(measure(&g, (6.0, 12.0), (40.0, 12.0), &bad).is_err());
        }
        assert!(check(&EdgeSettings::default()).is_ok());
        assert!(locate(&[1.0, 2.0], 0.25, &EdgeSettings::default()).is_err());
        assert!(locate(&[f64::NAN; 12], 0.25, &EdgeSettings::default()).is_err());
    }

    #[test]
    fn the_files_have_the_numbers_of_the_reading() {
        let g = edge_picture(24.0, 40.0, 210.0, 0.0, 1);
        let e = measure(&g, (6.0, 12.0), (40.0, 12.0), &EdgeSettings::default()).unwrap();
        let csv = to_csv(&e);
        let mut lines = csv.lines();
        assert_eq!(lines.next(), Some(CSV_HEADER));
        let row: Vec<&str> = lines.next().unwrap().split(',').collect();
        assert_eq!(row.len(), CSV_HEADER.split(',').count());
        assert_eq!(row[3], "true");
        assert_eq!(row[10], "sure");
        let json = to_json(&e);
        assert!(json.contains("\"format\": \"eh3d-vision-edge\""));
        assert!(json.contains("\"unit\": \"px\""));
        assert!(json.contains("\"rising\": true"));
        assert!(summary(&e, 0.25).contains("sure"));
    }

    #[test]
    fn a_profile_covers_the_line_from_end_to_end_with_the_step_asked_or_a_little_less() {
        let g = edge_picture(24.0, 40.0, 210.0, 0.0, 1);
        let (p, step) = profile(&g, (6.0, 12.0), (40.0, 12.0), 0.25).unwrap();
        assert!((p.len() - 1) as f64 * step - 34.0 < 1e-9);
        assert!(step <= 0.25 + 1e-12 && step > 0.24);
        // too many samples are refused
        let wide = Gray::blank(20_000, 4, 9).unwrap();
        let e = profile(&wide, (0.0, 1.0), (19_999.0, 1.0), 0.05).unwrap_err();
        assert!(e.starts_with(BAD_LINE), "{e}");
    }
}
