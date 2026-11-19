//! The pads of a part that has them in a row (a connector, a QFP side, the legs of a SOIC): the parts found in a picture by `measure` are the pads, and from their places this works out
//! the pitch (the distance between neighbours along the row), how much it varies, how far the pads lie from the straight line of the row and their mean size. A bent leg shows as a
//! pitch that is off or a pad out of the row. The row is the line of best fit through the centres, so it need not be along an axis of the picture.

use crate::inspect::json_string;
use crate::measure::{fixed, Part};

/// The reason for a pitch that is not the same all along the row.
pub const REASON_PITCH: &str = "pitch";
/// The reason for a pad that lies off the line of the row.
pub const REASON_ROW: &str = "row";
/// The reason for a median pitch that is not the pitch that was asked for.
pub const REASON_EXPECTED: &str = "expected";
/// The fewest pads that make a row.
pub const MIN_PADS: usize = 3;
/// The line of column names of the CSV file.
pub const CSV_HEADER: &str = "pads,pitch_mm,pitch_min_mm,pitch_max_mm,spread,row_deviation_mm,row_angle_deg,length_mm,width_mm,verdict,reason";
/// The name of the format and its version, in the JSON file.
pub const FORMAT: &str = "eh3d-vision-pads";
pub const FORMAT_VERSION: u32 = 1;

/// What is asked of a row of pads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PadSettings {
    /// How much the pitches (and the distance of a pad from the row, against the pitch) may differ from the median, as a share of it (0.05 is five percent).
    pub tolerance: f64,
    /// The pitch that the row should have, in millimetres; `None` does not check it.
    pub expected_pitch_mm: Option<f64>,
}

impl Default for PadSettings {
    fn default() -> Self {
        PadSettings {
            tolerance: 0.05,
            expected_pitch_mm: None,
        }
    }
}

/// What was measured on a row of pads.
#[derive(Debug, Clone, PartialEq)]
pub struct Pads {
    pub count: usize,
    /// The median of the distances between neighbouring pads along the row.
    pub pitch_mm: f64,
    pub pitch_min_mm: f64,
    pub pitch_max_mm: f64,
    /// The widest difference of two pitches, as a share of the median pitch.
    pub spread: f64,
    /// The largest distance of a centre from the line of the row.
    pub row_deviation_mm: f64,
    /// The direction of the row, in degrees counter-clockwise from the x axis, above -90 and up to 90.
    pub row_angle_deg: f64,
    /// The mean length and width of a pad.
    pub length_mm: f64,
    pub width_mm: f64,
    /// Why the row is refused (`pitch`, `row`, `expected`); empty when it is good.
    pub reasons: Vec<&'static str>,
}

impl Pads {
    /// Whether the row passes.
    pub fn passes(&self) -> bool {
        self.reasons.is_empty()
    }
}

/// Whether the settings make sense; the reason when they do not.
pub fn check(s: &PadSettings) -> Result<(), String> {
    if !(s.tolerance > 0.0 && s.tolerance <= 1.0) {
        return Err("the tolerance of the pitch must be more than 0 and at most 1".into());
    }
    if let Some(p) = s.expected_pitch_mm {
        if !(p.is_finite() && p > 0.0) {
            return Err("the pitch that is asked for must be more than 0 millimetres".into());
        }
    }
    Ok(())
}

/// The median of a list of numbers that is not empty.
fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

/// The pitch and the straightness of a row of pads. Fewer than three pads is the error `few_pads`; pads on top of each other (a median pitch of nothing) is `pads_overlap`.
pub fn measure_row(parts: &[Part], s: &PadSettings) -> Result<Pads, String> {
    check(s)?;
    let n = parts.len();
    if n < MIN_PADS {
        return Err(format!(
            "few_pads: {n} pads are not a row (at least {MIN_PADS} are needed)"
        ));
    }
    let count = n as f64;
    let (mx, my) = (
        parts.iter().map(|p| p.x_mm).sum::<f64>() / count,
        parts.iter().map(|p| p.y_mm).sum::<f64>() / count,
    );
    let (mut sxx, mut syy, mut sxy) = (0f64, 0f64, 0f64);
    for p in parts {
        let (dx, dy) = (p.x_mm - mx, p.y_mm - my);
        sxx += dx * dx;
        syy += dy * dy;
        sxy += dx * dy;
    }
    // the direction of the line of best fit through the centres
    let theta = 0.5 * (2.0 * sxy).atan2(sxx - syy);
    let (ux, uy) = (theta.cos(), theta.sin());
    let mut along: Vec<f64> = Vec::with_capacity(n);
    let mut row_deviation_mm = 0f64;
    for p in parts {
        let (dx, dy) = (p.x_mm - mx, p.y_mm - my);
        along.push(dx * ux + dy * uy);
        row_deviation_mm = row_deviation_mm.max((-dx * uy + dy * ux).abs());
    }
    along.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut pitches: Vec<f64> = along.windows(2).map(|w| w[1] - w[0]).collect();
    pitches.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pitch_mm = median(&pitches);
    if !(pitch_mm.is_finite() && pitch_mm > 1e-9) {
        return Err("pads_overlap: the pads lie on top of each other, there is no pitch".into());
    }
    let (pitch_min_mm, pitch_max_mm) = (pitches[0], pitches[pitches.len() - 1]);
    let spread = (pitch_max_mm - pitch_min_mm) / pitch_mm;
    let mut row_angle_deg = theta.to_degrees();
    if row_angle_deg <= -90.0 {
        row_angle_deg += 180.0;
    }
    if row_angle_deg > 90.0 {
        row_angle_deg -= 180.0;
    }
    let mut reasons = Vec::new();
    if spread > s.tolerance {
        reasons.push(REASON_PITCH);
    }
    if row_deviation_mm > s.tolerance * pitch_mm {
        reasons.push(REASON_ROW);
    }
    if let Some(want) = s.expected_pitch_mm {
        if (pitch_mm - want).abs() > s.tolerance * want {
            reasons.push(REASON_EXPECTED);
        }
    }
    Ok(Pads {
        count: n,
        pitch_mm,
        pitch_min_mm,
        pitch_max_mm,
        spread,
        row_deviation_mm,
        row_angle_deg,
        length_mm: parts.iter().map(|p| p.length_mm).sum::<f64>() / count,
        width_mm: parts.iter().map(|p| p.width_mm).sum::<f64>() / count,
        reasons,
    })
}

fn verdict(p: &Pads) -> &'static str {
    if p.passes() {
        "pass"
    } else {
        "fail"
    }
}

/// The result as CSV: the line `CSV_HEADER` and one line with the numbers, the verdict (`pass` or `fail`) and the reasons joined with `+`.
pub fn to_csv(p: &Pads) -> String {
    format!(
        "{CSV_HEADER}\n{},{},{},{},{},{},{},{},{},{},{}\n",
        p.count,
        fixed(p.pitch_mm, 4),
        fixed(p.pitch_min_mm, 4),
        fixed(p.pitch_max_mm, 4),
        fixed(p.spread, 4),
        fixed(p.row_deviation_mm, 4),
        fixed(p.row_angle_deg, 3),
        fixed(p.length_mm, 4),
        fixed(p.width_mm, 4),
        verdict(p),
        p.reasons.join("+")
    )
}

/// The result as JSON: an object with `format`, `version`, the same fields as the columns of the CSV file, the `verdict` and the list of `reasons`.
pub fn to_json(p: &Pads) -> String {
    let reasons: Vec<String> = p.reasons.iter().map(|r| json_string(r)).collect();
    format!(
        "{{\n  \"format\": \"{FORMAT}\",\n  \"version\": {FORMAT_VERSION},\n  \"pads\": {},\n  \"pitch_mm\": {},\n  \"pitch_min_mm\": {},\n  \"pitch_max_mm\": {},\n  \"spread\": {},\n  \"row_deviation_mm\": {},\n  \"row_angle_deg\": {},\n  \"length_mm\": {},\n  \"width_mm\": {},\n  \"verdict\": \"{}\",\n  \"reasons\": [{}]\n}}\n",
        p.count,
        fixed(p.pitch_mm, 4),
        fixed(p.pitch_min_mm, 4),
        fixed(p.pitch_max_mm, 4),
        fixed(p.spread, 4),
        fixed(p.row_deviation_mm, 4),
        fixed(p.row_angle_deg, 3),
        fixed(p.length_mm, 4),
        fixed(p.width_mm, 4),
        verdict(p),
        reasons.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(x: f64, y: f64) -> Part {
        Part {
            x_mm: x,
            y_mm: y,
            angle_deg: 90.0,
            length_mm: 1.2,
            width_mm: 0.3,
            area_mm2: 0.36,
        }
    }

    fn row(n: usize, pitch: f64, angle_deg: f64) -> Vec<Part> {
        let (s, c) = angle_deg.to_radians().sin_cos();
        (0..n)
            .map(|k| {
                let t = k as f64 * pitch - 2.0;
                pad(t * c + 1.0, t * s - 3.0)
            })
            .collect()
    }

    #[test]
    fn a_straight_row_has_its_pitch_and_passes() {
        let p = measure_row(&row(8, 0.5, 0.0), &PadSettings::default()).unwrap();
        assert_eq!(p.count, 8);
        assert!((p.pitch_mm - 0.5).abs() < 1e-9);
        assert!(p.spread < 1e-9 && p.row_deviation_mm < 1e-9);
        assert!(p.row_angle_deg.abs() < 1e-6);
        assert!((p.length_mm - 1.2).abs() < 1e-12 && (p.width_mm - 0.3).abs() < 1e-12);
        assert!(p.passes());
    }

    #[test]
    fn the_row_may_be_turned_and_the_pitch_is_the_same() {
        for angle in [30.0, 90.0, -60.0, 135.0, -135.0] {
            let p = measure_row(&row(6, 0.8, angle), &PadSettings::default()).unwrap();
            assert!((p.pitch_mm - 0.8).abs() < 1e-9, "{angle}: {}", p.pitch_mm);
            assert!(p.passes(), "{angle}");
            assert!(p.row_angle_deg > -90.0 && p.row_angle_deg <= 90.0);
            let want = {
                let mut a = angle;
                while a <= -90.0 {
                    a += 180.0;
                }
                while a > 90.0 {
                    a -= 180.0;
                }
                a
            };
            assert!(
                (p.row_angle_deg - want).abs() < 1e-6,
                "{angle}: {}",
                p.row_angle_deg
            );
        }
    }

    #[test]
    fn a_pad_moved_along_the_row_is_a_pitch_that_is_off() {
        let mut pads = row(8, 0.5, 0.0);
        pads[3].x_mm += 0.2;
        let p = measure_row(&pads, &PadSettings::default()).unwrap();
        assert!(p.reasons.contains(&REASON_PITCH));
        assert!(p.spread > 0.05);
        assert!(!p.passes());
        // with a wide tolerance it passes
        let loose = PadSettings {
            tolerance: 1.0,
            ..PadSettings::default()
        };
        assert!(measure_row(&pads, &loose).unwrap().passes());
    }

    #[test]
    fn a_bent_leg_is_a_pad_off_the_row() {
        let mut pads = row(9, 0.5, 0.0);
        pads[4].y_mm += 0.1;
        let p = measure_row(&pads, &PadSettings::default()).unwrap();
        assert!(p.reasons.contains(&REASON_ROW), "{p:?}");
        assert!(p.row_deviation_mm > 0.05);
    }

    #[test]
    fn the_pitch_that_is_asked_for_is_checked() {
        let pads = row(6, 0.5, 20.0);
        let ask = |pitch: f64| PadSettings {
            expected_pitch_mm: Some(pitch),
            ..PadSettings::default()
        };
        assert!(measure_row(&pads, &ask(0.5)).unwrap().passes());
        let p = measure_row(&pads, &ask(0.65)).unwrap();
        assert_eq!(p.reasons, vec![REASON_EXPECTED]);
    }

    #[test]
    fn few_pads_overlapping_pads_and_bad_settings_are_refused() {
        assert!(measure_row(&row(2, 0.5, 0.0), &PadSettings::default())
            .unwrap_err()
            .starts_with("few_pads"));
        assert!(measure_row(&[], &PadSettings::default()).is_err());
        let same = vec![pad(1.0, 1.0); 4];
        assert!(measure_row(&same, &PadSettings::default())
            .unwrap_err()
            .starts_with("pads_overlap"));
        for bad in [
            PadSettings {
                tolerance: 0.0,
                expected_pitch_mm: None,
            },
            PadSettings {
                tolerance: 1.5,
                expected_pitch_mm: None,
            },
            PadSettings {
                tolerance: 0.05,
                expected_pitch_mm: Some(0.0),
            },
            PadSettings {
                tolerance: 0.05,
                expected_pitch_mm: Some(f64::NAN),
            },
        ] {
            assert!(check(&bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_median_of_an_even_and_an_odd_list() {
        assert_eq!(median(&[1.0, 2.0, 9.0]), 2.0);
        assert_eq!(median(&[1.0, 2.0, 4.0, 9.0]), 3.0);
        assert_eq!(median(&[5.0]), 5.0);
    }

    #[test]
    fn the_files_have_the_numbers_the_verdict_and_the_reasons() {
        let mut pads = row(8, 0.5, 0.0);
        pads[3].x_mm += 0.2;
        let p = measure_row(&pads, &PadSettings::default()).unwrap();
        let csv = to_csv(&p);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], CSV_HEADER);
        assert_eq!(lines[1].split(',').count(), CSV_HEADER.split(',').count());
        assert!(lines[1].starts_with("8,0.5000,"));
        assert!(lines[1].ends_with(",fail,pitch"));
        let json = to_json(&p);
        assert!(json.contains("\"format\": \"eh3d-vision-pads\""));
        assert!(json.contains("\"verdict\": \"fail\""));
        assert!(json.contains("\"reasons\": [\"pitch\"]"));
        let good = measure_row(&row(4, 1.27, 0.0), &PadSettings::default()).unwrap();
        assert!(to_csv(&good).trim_end().ends_with(",pass,"));
        assert!(to_json(&good).contains("\"reasons\": []"));
    }
}
