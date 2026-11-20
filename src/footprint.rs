//! The footprint of the pads of a part: every pad of a picture is measured (its centre and its two sides in millimetres, as `measure` does) and judged on its own against the size the design
//! gives, with a tolerance in millimetres, so that a pad that is too small (paste or copper that did not print), too big (a bridge in the making) or missing is told apart from one that is
//! right. `pads` judges the pitch of a row; this judges each pad.

use crate::inspect::json_string;
use crate::measure::{fixed, Part};

/// The name of the format and its version, in the JSON file.
pub const FORMAT: &str = "eh3d-vision-footprint";
pub const FORMAT_VERSION: u32 = 1;
/// The line of column names of the CSV file.
pub const CSV_HEADER: &str =
    "pad,x_mm,y_mm,length_mm,width_mm,d_length_mm,d_width_mm,verdict,reason";

/// The reasons that a pad or the footprint is refused.
pub const REASON_LENGTH: &str = "length";
pub const REASON_WIDTH: &str = "width";
pub const REASON_COUNT: &str = "count";

/// The most pads that are judged in one picture.
pub const MAX_PADS: usize = 5000;
/// A side exactly on the tolerance passes: the sum in floating point may be a hair above it.
const EPSILON: f64 = 1e-9;

/// What the design gives and what is allowed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FootprintSettings {
    /// The size of a pad: the longer side and the shorter one, in millimetres.
    pub length_mm: f64,
    pub width_mm: f64,
    /// How far each side may be from the design, in millimetres.
    pub tolerance_mm: f64,
    /// How many pads the part has, when it is known.
    pub expected_count: Option<usize>,
}

/// Whether the settings make sense; the reason when they do not.
pub fn check(s: &FootprintSettings) -> Result<(), String> {
    if !(s.length_mm.is_finite() && s.width_mm.is_finite() && s.width_mm > 0.0) {
        return Err("bad_pad: the pad must have a length and a width above 0 millimetres".into());
    }
    if s.length_mm < s.width_mm {
        return Err("bad_pad: the length of a pad is its longer side, not below the width".into());
    }
    if !(s.tolerance_mm.is_finite() && s.tolerance_mm > 0.0 && s.tolerance_mm <= s.width_mm) {
        return Err(
            "the tolerance of a pad must be more than 0 and not more than its width in millimetres"
                .into(),
        );
    }
    if s.expected_count == Some(0) {
        return Err("the number of pads that is asked for must be 1 or more".into());
    }
    Ok(())
}

/// One pad, judged.
#[derive(Debug, Clone, PartialEq)]
pub struct PadJudged {
    /// The number of the pad, from 1: rows from the top of the picture down, each from left to right.
    pub index: usize,
    pub x_mm: f64,
    pub y_mm: f64,
    pub length_mm: f64,
    pub width_mm: f64,
    /// The measured side minus the one of the design.
    pub d_length_mm: f64,
    pub d_width_mm: f64,
    pub reasons: Vec<&'static str>,
}

/// The footprint, judged.
#[derive(Debug, Clone, PartialEq)]
pub struct Footprint {
    pub pads: Vec<PadJudged>,
    /// How many pads were asked for, if that was said.
    pub expected: Option<usize>,
    /// The mean difference of the sides from the design.
    pub mean_d_length_mm: f64,
    pub mean_d_width_mm: f64,
    /// The largest difference of any side from the design, in millimetres without a sign.
    pub worst_mm: f64,
    /// Why the footprint is refused: the reasons of its pads, once each, and `count`; empty when it is good.
    pub reasons: Vec<&'static str>,
}

impl Footprint {
    pub fn passes(&self) -> bool {
        self.reasons.is_empty()
    }

    /// How many pads pass.
    pub fn pads_ok(&self) -> usize {
        self.pads.iter().filter(|p| p.reasons.is_empty()).count()
    }
}

/// Judges the pads of a picture. Nothing found is `no_pads`, more than `MAX_PADS` is `too_many_pads`, and a pad whose centre or sides are not numbers is refused.
pub fn judge(parts: &[Part], s: &FootprintSettings) -> Result<Footprint, String> {
    check(s)?;
    if parts.is_empty() {
        return Err("no_pads: there is no pad in the picture".into());
    }
    if parts.len() > MAX_PADS {
        return Err(format!(
            "too_many_pads: {} pads, more than the {MAX_PADS} that are judged",
            parts.len()
        ));
    }
    if parts.iter().any(|p| {
        !(p.x_mm.is_finite()
            && p.y_mm.is_finite()
            && p.length_mm.is_finite()
            && p.width_mm.is_finite())
    }) {
        return Err("a pad has a place or a size that is not a number".into());
    }
    // rows from the top of the picture down, each from left to right; the rows are told apart by half the width of a pad
    let mut order: Vec<&Part> = parts.iter().collect();
    order.sort_by(|a, b| b.y_mm.total_cmp(&a.y_mm));
    let mut rows: Vec<Vec<&Part>> = Vec::new();
    for p in order {
        match rows.last_mut() {
            Some(row) if (row[0].y_mm - p.y_mm).abs() <= s.width_mm / 2.0 => row.push(p),
            _ => rows.push(vec![p]),
        }
    }
    let mut pads = Vec::with_capacity(parts.len());
    for mut row in rows {
        row.sort_by(|a, b| a.x_mm.total_cmp(&b.x_mm));
        for p in row {
            let (d_length_mm, d_width_mm) = (p.length_mm - s.length_mm, p.width_mm - s.width_mm);
            let mut reasons = Vec::new();
            if d_length_mm.abs() > s.tolerance_mm + EPSILON {
                reasons.push(REASON_LENGTH);
            }
            if d_width_mm.abs() > s.tolerance_mm + EPSILON {
                reasons.push(REASON_WIDTH);
            }
            pads.push(PadJudged {
                index: pads.len() + 1,
                x_mm: p.x_mm,
                y_mm: p.y_mm,
                length_mm: p.length_mm,
                width_mm: p.width_mm,
                d_length_mm,
                d_width_mm,
                reasons,
            });
        }
    }
    let n = pads.len() as f64;
    let mut reasons: Vec<&'static str> = Vec::new();
    for pad in &pads {
        for r in &pad.reasons {
            if !reasons.contains(r) {
                reasons.push(r);
            }
        }
    }
    if s.expected_count.is_some_and(|c| c != pads.len()) {
        reasons.push(REASON_COUNT);
    }
    Ok(Footprint {
        mean_d_length_mm: pads.iter().map(|p| p.d_length_mm).sum::<f64>() / n,
        mean_d_width_mm: pads.iter().map(|p| p.d_width_mm).sum::<f64>() / n,
        worst_mm: pads
            .iter()
            .map(|p| p.d_length_mm.abs().max(p.d_width_mm.abs()))
            .fold(0.0, f64::max),
        pads,
        expected: s.expected_count,
        reasons,
    })
}

fn verdict(reasons: &[&str]) -> &'static str {
    if reasons.is_empty() {
        "pass"
    } else {
        "fail"
    }
}

/// The result as CSV: the line `CSV_HEADER` and one line for each pad, with the verdict (`pass` or `fail`) and the reasons joined with `+`. A count that is not the one asked for is not a
/// pad's, so it is in the exit code and in the JSON file.
pub fn to_csv(f: &Footprint) -> String {
    let mut out = format!("{CSV_HEADER}\n");
    for p in &f.pads {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            p.index,
            fixed(p.x_mm, 4),
            fixed(p.y_mm, 4),
            fixed(p.length_mm, 4),
            fixed(p.width_mm, 4),
            fixed(p.d_length_mm, 4),
            fixed(p.d_width_mm, 4),
            verdict(&p.reasons),
            p.reasons.join("+")
        ));
    }
    out
}

/// The result as JSON: an object with `format`, `version`, the number of pads and of those that pass, the number that was asked for (`null` when none), the mean and the worst difference,
/// the `verdict`, the list of `reasons` and the list of `pads` with the fields of the columns of the CSV file.
pub fn to_json(f: &Footprint) -> String {
    let reasons: Vec<String> = f.reasons.iter().map(|r| json_string(r)).collect();
    let pads: Vec<String> = f
        .pads
        .iter()
        .map(|p| {
            let why: Vec<String> = p.reasons.iter().map(|r| json_string(r)).collect();
            format!(
                "    {{\"pad\": {}, \"x_mm\": {}, \"y_mm\": {}, \"length_mm\": {}, \"width_mm\": {}, \"d_length_mm\": {}, \"d_width_mm\": {}, \"verdict\": \"{}\", \"reasons\": [{}]}}",
                p.index,
                fixed(p.x_mm, 4),
                fixed(p.y_mm, 4),
                fixed(p.length_mm, 4),
                fixed(p.width_mm, 4),
                fixed(p.d_length_mm, 4),
                fixed(p.d_width_mm, 4),
                verdict(&p.reasons),
                why.join(", ")
            )
        })
        .collect();
    let expected = f
        .expected
        .map_or_else(|| "null".to_string(), |c| c.to_string());
    format!(
        "{{\n  \"format\": \"{FORMAT}\",\n  \"version\": {FORMAT_VERSION},\n  \"pads\": {},\n  \"pads_ok\": {},\n  \"expected\": {expected},\n  \"mean_d_length_mm\": {},\n  \"mean_d_width_mm\": {},\n  \"worst_mm\": {},\n  \"verdict\": \"{}\",\n  \"reasons\": [{}],\n  \"list\": [\n{}\n  ]\n}}\n",
        f.pads.len(),
        f.pads_ok(),
        fixed(f.mean_d_length_mm, 4),
        fixed(f.mean_d_width_mm, 4),
        fixed(f.worst_mm, 4),
        verdict(&f.reasons),
        reasons.join(", "),
        pads.join(",\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(x: f64, y: f64, length: f64, width: f64) -> Part {
        Part {
            x_mm: x,
            y_mm: y,
            angle_deg: 0.0,
            length_mm: length,
            width_mm: width,
            area_mm2: length * width,
        }
    }

    fn design() -> FootprintSettings {
        FootprintSettings {
            length_mm: 1.5,
            width_mm: 0.6,
            tolerance_mm: 0.05,
            expected_count: None,
        }
    }

    #[test]
    fn pads_that_are_right_pass_and_are_numbered_by_rows_and_columns() {
        // two rows of three, given in no order
        let parts = vec![
            pad(1.0, 0.0, 1.5, 0.6),
            pad(-1.0, 3.0, 1.51, 0.59),
            pad(0.0, 0.0, 1.49, 0.61),
            pad(1.0, 3.01, 1.5, 0.6),
            pad(-1.0, 0.0, 1.5, 0.6),
            pad(0.0, 2.99, 1.5, 0.6),
        ];
        let f = judge(&parts, &design()).unwrap();
        assert!(f.passes(), "{:?}", f.reasons);
        assert_eq!(f.pads.len(), 6);
        assert_eq!(f.pads_ok(), 6);
        let order: Vec<(i32, i32)> = f
            .pads
            .iter()
            .map(|p| (p.x_mm.round() as i32, p.y_mm.round() as i32))
            .collect();
        assert_eq!(order, [(-1, 3), (0, 3), (1, 3), (-1, 0), (0, 0), (1, 0)]);
        assert_eq!(f.pads[0].index, 1);
        assert_eq!(f.pads[5].index, 6);
        assert!(f.worst_mm <= 0.011 + 1e-9, "{}", f.worst_mm);
    }

    #[test]
    fn each_pad_that_is_off_is_named_with_its_side() {
        let parts = vec![
            pad(0.0, 0.0, 1.5, 0.6),
            pad(2.0, 0.0, 1.38, 0.6),
            pad(4.0, 0.0, 1.5, 0.71),
            pad(6.0, 0.0, 1.62, 0.52),
        ];
        let f = judge(&parts, &design()).unwrap();
        assert_eq!(f.pads_ok(), 1);
        assert_eq!(f.pads[1].reasons, [REASON_LENGTH]);
        assert_eq!(f.pads[2].reasons, [REASON_WIDTH]);
        assert_eq!(f.pads[3].reasons, [REASON_LENGTH, REASON_WIDTH]);
        assert_eq!(f.reasons, [REASON_LENGTH, REASON_WIDTH]);
        assert!((f.worst_mm - 0.12).abs() < 1e-9);
        assert!((f.pads[1].d_length_mm + 0.12).abs() < 1e-9);
        // a wider tolerance lets them all pass
        let loose = FootprintSettings {
            tolerance_mm: 0.15,
            ..design()
        };
        assert!(judge(&parts, &loose).unwrap().passes());
        // the edge of the tolerance is allowed
        let edge = vec![pad(0.0, 0.0, 1.55, 0.55), pad(1.0, 0.0, 1.45, 0.65)];
        assert!(judge(&edge, &design()).unwrap().passes());
    }

    #[test]
    fn a_missing_pad_is_told_by_the_count() {
        let parts = vec![pad(0.0, 0.0, 1.5, 0.6), pad(1.0, 0.0, 1.5, 0.6)];
        let want = FootprintSettings {
            expected_count: Some(3),
            ..design()
        };
        let f = judge(&parts, &want).unwrap();
        assert_eq!(f.reasons, [REASON_COUNT]);
        assert_eq!(f.pads_ok(), 2);
        let right = FootprintSettings {
            expected_count: Some(2),
            ..design()
        };
        assert!(judge(&parts, &right).unwrap().passes());
    }

    #[test]
    fn nothing_to_judge_or_nonsense_is_refused() {
        assert!(judge(&[], &design()).unwrap_err().starts_with("no_pads"));
        let many = vec![pad(0.0, 0.0, 1.5, 0.6); MAX_PADS + 1];
        assert!(judge(&many, &design())
            .unwrap_err()
            .starts_with("too_many_pads"));
        assert!(judge(&[pad(f64::NAN, 0.0, 1.5, 0.6)], &design()).is_err());
        assert!(judge(&[pad(0.0, 0.0, f64::INFINITY, 0.6)], &design()).is_err());
        let ok = design();
        for bad in [
            FootprintSettings {
                length_mm: 0.5,
                ..ok
            },
            FootprintSettings {
                width_mm: 0.0,
                ..ok
            },
            FootprintSettings {
                width_mm: f64::NAN,
                ..ok
            },
            FootprintSettings {
                tolerance_mm: 0.0,
                ..ok
            },
            FootprintSettings {
                tolerance_mm: 0.7,
                ..ok
            },
            FootprintSettings {
                tolerance_mm: f64::NAN,
                ..ok
            },
            FootprintSettings {
                expected_count: Some(0),
                ..ok
            },
        ] {
            assert!(check(&bad).is_err(), "{bad:?}");
        }
        assert!(check(&ok).is_ok());
    }

    #[test]
    fn the_files_have_the_columns_the_verdicts_and_the_reasons() {
        let parts = vec![pad(0.0, 0.0, 1.5, 0.6), pad(2.0, 0.0, 1.3, 0.6)];
        let want = FootprintSettings {
            expected_count: Some(3),
            ..design()
        };
        let f = judge(&parts, &want).unwrap();
        let csv = to_csv(&f);
        let mut lines = csv.lines();
        assert_eq!(lines.next().unwrap(), CSV_HEADER);
        let first: Vec<&str> = lines.next().unwrap().split(',').collect();
        assert_eq!(first.len(), CSV_HEADER.split(',').count());
        assert_eq!(&first[7..], ["pass", ""]);
        assert!(lines.next().unwrap().ends_with("fail,length"));
        assert!(lines.next().is_none());
        let json = to_json(&f);
        assert!(json.contains("\"format\": \"eh3d-vision-footprint\""));
        assert!(json.contains("\"pads\": 2") && json.contains("\"pads_ok\": 1"));
        assert!(json.contains("\"expected\": 3"));
        assert!(json.contains("\"reasons\": [\"length\", \"count\"]"));
        let none = judge(&parts, &design()).unwrap();
        assert!(to_json(&none).contains("\"expected\": null"));
    }
}
