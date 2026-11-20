//! The file of a calibration. It is plain text, `key = value`, with the residual of the fit, the three scales and the check against the measured distance written beside the map, so that a
//! person can read why it was accepted. Reading it back is not trusting it: every number is checked again against the settings of today (a stricter residual, a smaller gap, a ruler that
//! is now asked for), the numbers must agree with each other, and a file that is cut, that repeats a key, that has a key it does not know or that says something that is not true is
//! `calib_file_bad`. Nothing of the offset of the camera is in the file.

use super::solve::{Calibration, Check, Method, Settings};
use super::BoardSpec;
use crate::capture::{CapError, Code};
use crate::limit;
use crate::perspective::Homography;
use std::path::Path;

/// The first line that says what the file is.
pub const FORMAT: &str = "eh3d-vision-calibration/1";

const KEYS: [&str; 25] = [
    "format",
    "squares_x",
    "squares_y",
    "square_mm",
    "seed",
    "corners",
    "homography",
    "mm_per_px",
    "aspect",
    "rms_px",
    "max_px",
    "scale_homography",
    "scale_affine",
    "scale_similarity",
    "rms_mm_homography",
    "rms_mm_affine",
    "rms_mm_similarity",
    "gap",
    "scale_gap",
    "check",
    "check_measured_mm",
    "check_found_mm",
    "verified",
    "evidence",
    "accepted",
];

fn f(v: f64) -> String {
    format!("{v:?}")
}

impl Calibration {
    /// The text of the file.
    pub fn to_text(&self) -> String {
        let h = self.homography.0;
        let (check, measured, found) = match self.check {
            Check::NotMeasured => ("none", 0.0, 0.0),
            Check::Measured {
                measured_mm,
                found_mm,
                ..
            } => ("measured", measured_mm, found_mm),
        };
        let yes = |b: bool| if b { "yes" } else { "no" };
        let mut out =
            String::from("# The calibration of the camera: pixels to millimetres on the board.\n");
        for (k, v) in [
            ("format", FORMAT.to_string()),
            ("squares_x", self.spec.cols.to_string()),
            ("squares_y", self.spec.rows.to_string()),
            ("square_mm", f(self.spec.square_mm)),
            ("seed", self.spec.seed.to_string()),
            ("corners", self.corners.to_string()),
            (
                "homography",
                h.iter().map(|v| f(*v)).collect::<Vec<_>>().join(","),
            ),
            ("mm_per_px", f(self.mm_per_px)),
            ("aspect", f(self.aspect)),
            ("rms_px", f(self.rms_px)),
            ("max_px", f(self.max_px)),
            ("scale_homography", f(self.methods[0].mm_per_px)),
            ("scale_affine", f(self.methods[1].mm_per_px)),
            ("scale_similarity", f(self.methods[2].mm_per_px)),
            ("rms_mm_homography", f(self.methods[0].rms_mm)),
            ("rms_mm_affine", f(self.methods[1].rms_mm)),
            ("rms_mm_similarity", f(self.methods[2].rms_mm)),
            ("gap", f(self.gap)),
            ("scale_gap", f(self.scale_gap)),
            ("check", check.to_string()),
            ("check_measured_mm", f(measured)),
            ("check_found_mm", f(found)),
            ("verified", yes(self.verified).to_string()),
            ("evidence", self.evidence.to_string()),
            ("accepted", "yes".to_string()),
        ] {
            out.push_str(&format!("{k} = {v}\n"));
        }
        out
    }

    /// The calibration written in a text, checked again against `settings`.
    pub fn parse(text: &str, settings: &Settings) -> Result<Calibration, CapError> {
        let bad = |why: String| CapError::new(Code::CalibFileBad, why);
        let mut seen: Vec<(&str, &str)> = Vec::new();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (k, v) = line
                .split_once('=')
                .ok_or_else(|| bad(format!("line {}: it must be key = value", n + 1)))?;
            let (k, v) = (k.trim(), v.trim());
            if !KEYS.contains(&k) {
                return Err(bad(format!("line {}: {k} is not a key of the file", n + 1)));
            }
            if seen.iter().any(|(s, _)| *s == k) {
                return Err(bad(format!("line {}: {k} is written twice", n + 1)));
            }
            seen.push((k, v));
        }
        let get = |k: &str| -> Result<&str, CapError> {
            seen.iter()
                .find(|(s, _)| *s == k)
                .map(|(_, v)| *v)
                .ok_or_else(|| bad(format!("{k} is missing")))
        };
        let num = |k: &str| -> Result<f64, CapError> {
            get(k)?
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| bad(format!("{k} is not a number")))
        };
        let whole = |k: &str| -> Result<u64, CapError> {
            get(k)?
                .parse::<u64>()
                .map_err(|_| bad(format!("{k} is not a whole number")))
        };
        let yes_no = |k: &str| -> Result<bool, CapError> {
            match get(k)? {
                "yes" => Ok(true),
                "no" => Ok(false),
                _ => Err(bad(format!("{k} is not yes or no"))),
            }
        };
        if get("format")? != FORMAT {
            return Err(bad(format!("the format is not {FORMAT}")));
        }
        if !yes_no("accepted")? {
            return Err(bad("the calibration was not accepted".into()));
        }
        let spec = BoardSpec {
            cols: whole("squares_x")? as usize,
            rows: whole("squares_y")? as usize,
            square_mm: num("square_mm")?,
            seed: whole("seed")?,
        };
        spec.check().map_err(bad)?;
        let parts: Vec<f64> = get("homography")?
            .split(',')
            .map(|p| p.trim().parse::<f64>().ok().filter(|v| v.is_finite()))
            .collect::<Option<Vec<f64>>>()
            .ok_or_else(|| bad("homography has a value that is not a number".into()))?;
        let h: [f64; 9] = parts
            .try_into()
            .map_err(|_| bad("homography needs nine values".into()))?;
        let homography = Homography(h);
        if homography.inverse().is_none() {
            return Err(bad("the map cannot be undone".into()));
        }
        let method = |name, scale: &str, rms: &str| -> Result<Method, CapError> {
            Ok(Method {
                name,
                mm_per_px: num(scale)?,
                rms_mm: num(rms)?,
            })
        };
        let methods = [
            method("homography", "scale_homography", "rms_mm_homography")?,
            method("affine", "scale_affine", "rms_mm_affine")?,
            method("similarity", "scale_similarity", "rms_mm_similarity")?,
        ];
        if methods.iter().any(|m| m.mm_per_px <= 0.0 || m.rms_mm < 0.0) {
            return Err(bad("a scale or a residual is out of range".into()));
        }
        let mm_per_px = num("mm_per_px")?;
        if mm_per_px <= 0.0 || (mm_per_px - methods[0].mm_per_px).abs() > 1e-9 * mm_per_px.max(1.0)
        {
            return Err(bad("mm_per_px is not the scale of the homography".into()));
        }
        let hi = methods.iter().map(|m| m.mm_per_px).fold(f64::MIN, f64::max);
        let lo = methods.iter().map(|m| m.mm_per_px).fold(f64::MAX, f64::min);
        let mean = methods.iter().map(|m| m.mm_per_px).sum::<f64>() / 3.0;
        let scale_gap = num("scale_gap")?;
        if (scale_gap - (hi - lo) / mean).abs() > 1e-6 {
            return Err(bad("scale_gap is not what the three scales give".into()));
        }
        let (rms_px, max_px, gap, aspect) =
            (num("rms_px")?, num("max_px")?, num("gap")?, num("aspect")?);
        if rms_px < 0.0 || max_px < rms_px - 1e-9 || gap < 0.0 || aspect <= 0.0 {
            return Err(bad("a residual, a gap or the aspect is out of range".into()));
        }
        let corners = whole("corners")? as usize;
        let verified = yes_no("verified")?;
        let evidence = crate::convention::EVIDENCE
            .iter()
            .copied()
            .find(|e| *e == get("evidence").unwrap_or(""))
            .ok_or_else(|| bad("evidence is not camera, picture or simulator".into()))?;
        let check = match get("check")? {
            "none" => Check::NotMeasured,
            "measured" => {
                let (measured_mm, found_mm) = (num("check_measured_mm")?, num("check_found_mm")?);
                Check::Measured {
                    measured_mm,
                    found_mm,
                    error_mm: found_mm - measured_mm,
                }
            }
            other => return Err(bad(format!("check is {other}, not none or measured"))),
        };
        if verified != matches!(check, Check::Measured { .. }) {
            return Err(bad("verified does not agree with check".into()));
        }
        // the numbers are checked again against the settings of today
        let refuse = |code, key: &'static str, why: String| Err(CapError::with_key(code, key, why));
        if corners < settings.min_corners {
            return refuse(
                Code::CalibFewCorners,
                "calib_min_corners",
                format!("{corners} corners in the file"),
            );
        }
        if rms_px > settings.max_residual_px {
            return refuse(
                Code::CalibResidual,
                "calib_max_residual_px",
                format!("{rms_px:.3} px in the file"),
            );
        }
        if gap > settings.max_method_gap {
            return refuse(
                Code::CalibMethodsDisagree,
                "calib_max_method_gap",
                format!("{:.1} % in the file", gap * 100.0),
            );
        }
        if let Check::Measured { error_mm, .. } = check {
            if error_mm.abs() > settings.check_tol_mm {
                return refuse(
                    Code::CalibCheckFailed,
                    "calib_check_tol_mm",
                    format!("{error_mm:+.3} mm in the file"),
                );
            }
        } else if !settings.allow_unchecked {
            return refuse(
                Code::CalibUnverified,
                "calib_check_mm",
                "the file was not checked against a measured distance".into(),
            );
        }
        Ok(Calibration {
            spec,
            corners,
            homography,
            mm_per_px,
            aspect,
            rms_px,
            max_px,
            methods,
            gap,
            scale_gap,
            check,
            verified,
            evidence,
        })
    }
}

/// Writes a calibration to a new file (never over one without `replace`).
pub fn save(path: &Path, cal: &Calibration, replace: bool) -> Result<(), CapError> {
    limit::write_text(path, &cal.to_text(), replace)
        .map_err(|e| CapError::new(Code::CalibFileBad, e))
}

/// Reads a calibration from a file and checks it against the settings.
pub fn load(path: &Path, settings: &Settings) -> Result<Calibration, CapError> {
    let text =
        limit::read_text_limited(path, 1).map_err(|e| CapError::new(Code::CalibFileBad, e))?;
    Calibration::parse(&text, settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::charuco::detect::{Corner, Detection};
    use crate::charuco::solve::calibrate;

    fn good() -> Calibration {
        let spec = BoardSpec::default();
        let mut corners = Vec::new();
        for j in 1..spec.rows {
            for i in 1..spec.cols {
                let mm = spec.corner_mm(i, j);
                corners.push(Corner {
                    id: (i, j),
                    px: (100.0 + 8.0 * mm.0, 90.0 + 8.0 * mm.1),
                    mm,
                    quality: 1.0,
                });
            }
        }
        let det = Detection {
            corners,
            markers: Vec::new(),
            marker_rms_px: 0.0,
            square_px: 40.0,
        };
        calibrate(
            &det,
            &spec,
            &Settings {
                check_mm: 30.0,
                ..Settings::default()
            },
        )
        .unwrap()
    }

    /// The text with the line of a key replaced by a new value.
    fn with(text: &str, key: &str, value: &str) -> String {
        let mut done = false;
        let lines: Vec<String> = text
            .lines()
            .map(|l| {
                if l.starts_with(&format!("{key} = ")) {
                    done = true;
                    format!("{key} = {value}")
                } else {
                    l.to_string()
                }
            })
            .collect();
        assert!(done, "{key} is not in the text");
        lines.join("\n") + "\n"
    }

    fn settings() -> Settings {
        Settings {
            check_mm: 30.0,
            ..Settings::default()
        }
    }

    #[test]
    fn a_calibration_written_and_read_back_is_the_same_and_has_no_offset_in_it() {
        let c = good();
        let text = c.to_text();
        assert!(!text.contains("offset"));
        let back = Calibration::parse(&text, &settings()).unwrap();
        assert_eq!(back.spec, c.spec);
        assert_eq!(back.homography, c.homography);
        assert_eq!(back.mm_per_px, c.mm_per_px);
        assert_eq!(back.methods, c.methods);
        assert_eq!(back.check, c.check);
        assert_eq!(back.to_text(), text);
    }

    #[test]
    fn the_file_is_read_again_against_the_settings_of_today() {
        let text = good().to_text();
        let stricter = Settings {
            min_corners: 40,
            ..settings()
        };
        assert_eq!(
            Calibration::parse(&text, &stricter).unwrap_err().code,
            Code::CalibFewCorners
        );
        let tight = with(&with(&text, "rms_px", "3.0"), "max_px", "4.0");
        let e = Calibration::parse(&tight, &settings()).unwrap_err();
        assert_eq!(e.code, Code::CalibResidual);
        assert_eq!(e.key, Some("calib_max_residual_px"));
        let gap = with(&text, "gap", "5.0");
        assert_eq!(
            Calibration::parse(&gap, &settings()).unwrap_err().code,
            Code::CalibMethodsDisagree
        );
        let off = with(&text, "check_found_mm", "29.0");
        assert_eq!(
            Calibration::parse(&off, &settings()).unwrap_err().code,
            Code::CalibCheckFailed
        );
    }

    #[test]
    fn a_calibration_that_was_not_checked_is_refused_when_the_settings_ask_for_the_ruler() {
        let unchecked = Settings {
            allow_unchecked: true,
            ..Settings::default()
        };
        let spec = BoardSpec::default();
        let mut corners = Vec::new();
        for j in 1..spec.rows {
            for i in 1..spec.cols {
                let mm = spec.corner_mm(i, j);
                corners.push(Corner {
                    id: (i, j),
                    px: (10.0 + 8.0 * mm.0, 10.0 + 8.0 * mm.1),
                    mm,
                    quality: 1.0,
                });
            }
        }
        let det = Detection {
            corners,
            markers: Vec::new(),
            marker_rms_px: 0.0,
            square_px: 40.0,
        };
        let c = calibrate(&det, &spec, &unchecked).unwrap();
        assert!(Calibration::parse(&c.to_text(), &unchecked).is_ok());
        let e = Calibration::parse(&c.to_text(), &Settings::default()).unwrap_err();
        assert_eq!(e.code, Code::CalibUnverified);
    }

    #[test]
    fn a_file_that_says_something_untrue_or_is_damaged_is_calib_file_bad() {
        let text = good().to_text();
        let cases = [
            with(&text, "accepted", "no"),
            with(&text, "evidence", "magic"),
            with(&text, "format", "other/2"),
            with(&text, "mm_per_px", "9.0"),
            with(&text, "scale_gap", "1.0"),
            with(&text, "verified", "no"),
            with(&text, "check", "maybe"),
            format!("{text}extra = 1\n"),
            format!("{text}corners = 3\n"),
            text.lines()
                .filter(|l| !l.starts_with("rms_px"))
                .collect::<Vec<_>>()
                .join("\n"),
            with(&text, "aspect", "nan"),
            with(&text, "squares_x", "1"),
            "just words".to_string(),
            String::new(),
        ];
        for (k, bad) in cases.iter().enumerate() {
            let e = Calibration::parse(bad, &settings()).unwrap_err();
            assert_eq!(e.code, Code::CalibFileBad, "case {k}: {e}");
        }
    }

    #[test]
    fn the_evidence_is_kept_in_the_file_and_a_simulated_calibration_is_refused_for_a_real_camera() {
        let sim = good().with_evidence("simulator").unwrap();
        let back = Calibration::parse(&sim.to_text(), &settings()).unwrap();
        assert_eq!(back.evidence, "simulator");
        assert!(back.check_evidence("simulator").is_ok());
        let e = back.check_evidence("camera").unwrap_err();
        assert_eq!(e.code, Code::CalibFileBad);
        assert!(
            good().check_evidence("camera").is_ok(),
            "a calibration from a picture file is not blamed"
        );
        assert!(good().with_evidence("magic").is_err());
        assert_eq!(good().with_evidence("camera").unwrap().evidence, "camera");
    }

    #[test]
    fn a_homography_that_is_not_nine_numbers_is_refused() {
        let text = good().to_text();
        let line = text
            .lines()
            .find(|l| l.starts_with("homography"))
            .unwrap()
            .to_string();
        let short = text.replace(&line, "homography = 1,2,3");
        assert_eq!(
            Calibration::parse(&short, &settings()).unwrap_err().code,
            Code::CalibFileBad
        );
        let flat = text.replace(&line, "homography = 0,0,0,0,0,0,0,0,0");
        assert_eq!(
            Calibration::parse(&flat, &settings()).unwrap_err().code,
            Code::CalibFileBad
        );
    }

    #[test]
    fn the_file_is_saved_once_and_read_back_from_disk() {
        let dir = std::env::temp_dir().join(format!("eh3d_vision_cal_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("camera.cal");
        let c = good();
        save(&path, &c, false).unwrap();
        assert!(
            save(&path, &c, false).is_err(),
            "never over a file without being told"
        );
        assert_eq!(load(&path, &settings()).unwrap().homography, c.homography);
        std::fs::write(&path, "not a calibration").unwrap();
        assert_eq!(
            load(&path, &settings()).unwrap_err().code,
            Code::CalibFileBad
        );
        assert_eq!(
            load(&dir.join("none.cal"), &settings()).unwrap_err().code,
            Code::CalibFileBad
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
