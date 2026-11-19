//! Whether a picture is good enough to measure from: not flat, not too dark, not too bright and, when asked, not blurred. The sharpness is the variance of the Laplacian (the sum of the four
//! neighbours minus four times the pixel): the sharper the edges, the larger it is. Only grey levels are looked at, so it works on any picture and on the pictures of the simulated camera;
//! the values that tell a good picture from a bad one depend on the camera and the lens, so they are in the configuration file.

use crate::image::Gray;

/// The reasons that a picture is refused.
pub const REASON_FLAT: &str = "flat";
pub const REASON_DARK: &str = "dark";
pub const REASON_BRIGHT: &str = "bright";
pub const REASON_BLURRY: &str = "blurry";
/// The line of column names of the CSV file.
pub const CSV_HEADER: &str = "sharpness,contrast,mean,verdict,reason";
/// The name of the format and its version, in the JSON file.
pub const FORMAT: &str = "eh3d-vision-quality";
pub const FORMAT_VERSION: u32 = 1;

/// What was measured in a picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quality {
    /// The mean grey level, 0 to 255.
    pub mean: f64,
    /// The standard deviation of the grey levels: how much the picture varies.
    pub contrast: f64,
    /// The variance of the Laplacian over the pixels that are not on the edge of the picture.
    pub sharpness: f64,
}

/// What is asked of a picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QualitySettings {
    /// The least contrast (standard deviation of the grey levels) that a picture may have.
    pub min_contrast: f64,
    /// The darkest and the brightest mean grey level that is accepted.
    pub min_mean: f64,
    pub max_mean: f64,
    /// The least sharpness that is accepted; 0 does not check it (it depends on the lens and the light, so it is off until it is tuned).
    pub min_sharpness: f64,
}

impl Default for QualitySettings {
    fn default() -> Self {
        QualitySettings {
            min_contrast: 10.0,
            min_mean: 20.0,
            max_mean: 235.0,
            min_sharpness: 0.0,
        }
    }
}

/// The mean, the contrast and the sharpness of a picture. A picture smaller than 3 by 3 pixels has no Laplacian and is refused.
pub fn measure(img: &Gray) -> Result<Quality, String> {
    let (w, h) = (img.w, img.h);
    if w < 3 || h < 3 || img.data.len() != w * h {
        return Err("picture_small: the picture is smaller than 3 by 3 pixels".into());
    }
    let count = (w * h) as f64;
    let sum: f64 = img.data.iter().map(|&v| f64::from(v)).sum();
    let mean = sum / count;
    let variance = img
        .data
        .iter()
        .map(|&v| (f64::from(v) - mean).powi(2))
        .sum::<f64>()
        / count;
    let at = |x: usize, y: usize| i32::from(img.data[y * w + x]);
    let (mut lap_sum, mut lap_sq) = (0f64, 0f64);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let l =
                f64::from(at(x - 1, y) + at(x + 1, y) + at(x, y - 1) + at(x, y + 1) - 4 * at(x, y));
            lap_sum += l;
            lap_sq += l * l;
        }
    }
    let n = ((w - 2) * (h - 2)) as f64;
    let lap_mean = lap_sum / n;
    Ok(Quality {
        mean,
        contrast: variance.sqrt(),
        sharpness: (lap_sq / n - lap_mean * lap_mean).max(0.0),
    })
}

/// Whether the settings make sense; the reason when they do not.
pub fn check(s: &QualitySettings) -> Result<(), String> {
    let ok = [s.min_contrast, s.min_mean, s.max_mean, s.min_sharpness]
        .iter()
        .all(|v| v.is_finite())
        && s.min_contrast >= 0.0
        && s.min_sharpness >= 0.0
        && s.min_mean >= 0.0
        && s.max_mean <= 255.0
        && s.min_mean <= s.max_mean;
    if ok {
        Ok(())
    } else {
        Err("the settings of the picture check make no sense (numbers, a contrast and a sharpness of 0 or more, and the means from 0 to 255 with the darkest not above the brightest)".into())
    }
}

/// Why a picture is refused: `flat` (too little contrast), `dark`, `bright` (the mean is outside the range) and `blurry` (the sharpness is below the least, only when it is above 0 and the
/// picture is not flat, since a flat picture has no edges to be sharp). Empty when the picture is good.
pub fn reasons(q: &Quality, s: &QualitySettings) -> Result<Vec<&'static str>, String> {
    check(s)?;
    let mut out = Vec::new();
    let flat = q.contrast < s.min_contrast;
    if flat {
        out.push(REASON_FLAT);
    }
    if q.mean < s.min_mean {
        out.push(REASON_DARK);
    }
    if q.mean > s.max_mean {
        out.push(REASON_BRIGHT);
    }
    if !flat && s.min_sharpness > 0.0 && q.sharpness < s.min_sharpness {
        out.push(REASON_BLURRY);
    }
    Ok(out)
}

/// Measures the picture and refuses it with `poor_picture` and the reasons joined with `+` when it is not good enough; used before a part is measured when the check is on.
pub fn require_good(img: &Gray, s: &QualitySettings) -> Result<Quality, String> {
    let q = measure(img)?;
    let why = reasons(&q, s)?;
    if why.is_empty() {
        Ok(q)
    } else {
        Err(format!(
            "poor_picture: {} (sharpness {:.2}, contrast {:.2}, mean {:.2}); the picture is not good enough to measure from",
            why.join("+"),
            q.sharpness,
            q.contrast,
            q.mean
        ))
    }
}

/// The result as CSV: the line `CSV_HEADER` and one line with the sharpness, the contrast, the mean, the verdict (`pass` or `fail`) and the reasons joined with `+`.
pub fn to_csv(q: &Quality, why: &[&str]) -> String {
    format!(
        "{CSV_HEADER}\n{:.4},{:.4},{:.4},{},{}\n",
        q.sharpness,
        q.contrast,
        q.mean,
        if why.is_empty() { "pass" } else { "fail" },
        why.join("+")
    )
}

/// The result as JSON: an object with `format`, `version`, `sharpness`, `contrast`, `mean`, the `verdict` and the list of `reasons`.
pub fn to_json(q: &Quality, why: &[&str]) -> String {
    let reasons: Vec<String> = why.iter().map(|r| crate::inspect::json_string(r)).collect();
    format!(
        "{{\n  \"format\": \"{FORMAT}\",\n  \"version\": {FORMAT_VERSION},\n  \"sharpness\": {:.4},\n  \"contrast\": {:.4},\n  \"mean\": {:.4},\n  \"verdict\": \"{}\",\n  \"reasons\": [{}]\n}}\n",
        q.sharpness,
        q.contrast,
        q.mean,
        if why.is_empty() { "pass" } else { "fail" },
        reasons.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture of 40 by 30 with a dark square of 16 by 12 on a light board, with the edges as sharp as they come.
    fn board() -> Gray {
        let mut g = Gray::blank(40, 30, 220).unwrap();
        for y in 9..21 {
            for x in 12..28 {
                g.data[y * 40 + x] = 30;
            }
        }
        g
    }

    /// The picture with every pixel averaged with its neighbours `times` times (a blur).
    fn blurred(g: &Gray, times: usize) -> Gray {
        let mut cur = g.clone();
        for _ in 0..times {
            let mut next = cur.clone();
            for y in 1..cur.h - 1 {
                for x in 1..cur.w - 1 {
                    let mut total = 0u32;
                    for dy in 0..3 {
                        for dx in 0..3 {
                            total += u32::from(cur.data[(y + dy - 1) * cur.w + x + dx - 1]);
                        }
                    }
                    next.data[y * cur.w + x] = (total / 9) as u8;
                }
            }
            cur = next;
        }
        cur
    }

    #[test]
    fn a_blurred_picture_is_less_sharp_and_a_flat_one_has_no_contrast() {
        let sharp = measure(&board()).unwrap();
        let soft = measure(&blurred(&board(), 2)).unwrap();
        let softer = measure(&blurred(&board(), 5)).unwrap();
        assert!(sharp.sharpness > soft.sharpness * 2.0, "{sharp:?} {soft:?}");
        assert!(soft.sharpness > softer.sharpness, "{soft:?} {softer:?}");
        assert!(sharp.contrast > 50.0 && sharp.mean > 100.0);
        let flat = measure(&Gray::blank(20, 20, 128).unwrap()).unwrap();
        assert_eq!(flat.contrast, 0.0);
        assert_eq!(flat.sharpness, 0.0);
        assert_eq!(flat.mean, 128.0);
        // a plain ramp has edges nowhere: the Laplacian of a straight slope is zero
        let mut ramp = Gray::blank(32, 8, 0).unwrap();
        for y in 0..8 {
            for x in 0..32 {
                ramp.data[y * 32 + x] = (x * 4) as u8;
            }
        }
        assert!(measure(&ramp).unwrap().sharpness < 1e-9);
    }

    #[test]
    fn a_picture_that_is_flat_dark_bright_or_blurred_is_refused_with_each_reason() {
        let s = QualitySettings {
            min_sharpness: 400.0,
            ..QualitySettings::default()
        };
        let good = measure(&board()).unwrap();
        assert!(good.sharpness > 400.0, "{good:?}");
        assert!(reasons(&good, &s).unwrap().is_empty());
        let soft = measure(&blurred(&board(), 4)).unwrap();
        assert_eq!(reasons(&soft, &s).unwrap(), vec![REASON_BLURRY]);
        // without a least sharpness a blurred picture passes
        assert!(reasons(&soft, &QualitySettings::default())
            .unwrap()
            .is_empty());
        let flat = measure(&Gray::blank(20, 20, 128).unwrap()).unwrap();
        assert_eq!(reasons(&flat, &s).unwrap(), vec![REASON_FLAT]);
        let dark = measure(&Gray::blank(20, 20, 3).unwrap()).unwrap();
        assert_eq!(reasons(&dark, &s).unwrap(), vec![REASON_FLAT, REASON_DARK]);
        let bright = measure(&Gray::blank(20, 20, 250).unwrap()).unwrap();
        assert_eq!(
            reasons(&bright, &s).unwrap(),
            vec![REASON_FLAT, REASON_BRIGHT]
        );
    }

    #[test]
    fn pictures_too_small_and_settings_that_make_no_sense_are_refused() {
        assert!(measure(&Gray::blank(2, 9, 10).unwrap())
            .unwrap_err()
            .starts_with("picture_small"));
        assert!(measure(&Gray::blank(9, 2, 10).unwrap()).is_err());
        assert!(measure(&Gray::blank(3, 3, 10).unwrap()).is_ok());
        let q = measure(&board()).unwrap();
        let d = QualitySettings::default();
        for bad in [
            QualitySettings {
                min_contrast: -1.0,
                ..d
            },
            QualitySettings {
                min_sharpness: -1.0,
                ..d
            },
            QualitySettings {
                min_mean: 200.0,
                max_mean: 100.0,
                ..d
            },
            QualitySettings {
                max_mean: 256.0,
                ..d
            },
            QualitySettings {
                min_mean: -1.0,
                ..d
            },
            QualitySettings {
                min_contrast: f64::NAN,
                ..d
            },
        ] {
            assert!(reasons(&q, &bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_check_before_a_measure_says_what_is_wrong_with_the_picture() {
        let d = QualitySettings::default();
        assert!(require_good(&board(), &d).is_ok());
        let e = require_good(&Gray::blank(20, 20, 3).unwrap(), &d).unwrap_err();
        assert!(e.starts_with("poor_picture: flat+dark ("), "{e}");
        let e = require_good(&Gray::blank(2, 2, 3).unwrap(), &d).unwrap_err();
        assert!(e.starts_with("picture_small"), "{e}");
    }

    #[test]
    fn the_files_have_the_numbers_the_verdict_and_the_reasons() {
        let q = Quality {
            mean: 120.5,
            contrast: 33.25,
            sharpness: 812.125,
        };
        let csv = to_csv(&q, &[]);
        assert_eq!(
            csv,
            format!("{CSV_HEADER}\n812.1250,33.2500,120.5000,pass,\n")
        );
        let bad = to_csv(&q, &[REASON_FLAT, REASON_DARK]);
        assert!(bad.ends_with(",fail,flat+dark\n"), "{bad}");
        let json = to_json(&q, &[REASON_BLURRY]);
        assert!(
            json.contains("\"format\": \"eh3d-vision-quality\""),
            "{json}"
        );
        assert!(json.contains("\"sharpness\": 812.1250"), "{json}");
        assert!(
            json.contains("\"verdict\": \"fail\"") && json.contains("\"reasons\": [\"blurry\"]"),
            "{json}"
        );
        assert!(to_json(&q, &[]).contains("\"reasons\": []"));
    }
}
