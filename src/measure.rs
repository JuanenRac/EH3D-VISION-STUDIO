//! Parts in a picture: the spots of the picture are measured (centre, turn, length, width, area) in millimetres so that a program that checks a part before it is placed can read them
//! from a file. The file is plain CSV or JSON and nothing else is shared with that program (see `to_csv` and `to_json` for the exact columns and fields).

use crate::fiducial::{otsu, Polarity};
use crate::image::Gray;
use crate::scale::Scale;

/// A connected spot of the picture, measured in pixels. The pixels are numbered from the top left corner, and the second moments are taken around the centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blob {
    /// How many pixels the spot has.
    pub n: u64,
    pub cx: f64,
    pub cy: f64,
    pub x0: usize,
    pub x1: usize,
    pub y0: usize,
    pub y1: usize,
    /// The second moments around the centre, in square pixels: along x, across, and along y.
    pub mxx: f64,
    pub mxy: f64,
    pub myy: f64,
}

/// The connected spots of the pixels for which `on` is true (four neighbours), kept only when the number of pixels is from `min_n` to `max_n`. More than `limit` kept spots is an
/// error with the code `too_many_parts`, so that a noisy picture does not fill the memory with spots.
pub fn blobs(
    img: &Gray,
    on: &dyn Fn(u8) -> bool,
    min_n: u64,
    max_n: u64,
    limit: usize,
) -> Result<Vec<Blob>, String> {
    let (w, h) = (img.w, img.h);
    let mut seen = vec![false; w * h];
    let mut stack: Vec<usize> = Vec::new();
    let mut found = Vec::new();
    for start in 0..w * h {
        if seen[start] || !on(img.data[start]) {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let (mut n, mut sx, mut sy) = (0u64, 0.0f64, 0.0f64);
        let (mut sxx, mut sxy, mut syy) = (0.0f64, 0.0f64, 0.0f64);
        let (mut x0, mut x1, mut y0, mut y1) = (usize::MAX, 0usize, usize::MAX, 0usize);
        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            let (fx, fy) = (x as f64, y as f64);
            n += 1;
            sx += fx;
            sy += fy;
            sxx += fx * fx;
            sxy += fx * fy;
            syy += fy * fy;
            (x0, x1, y0, y1) = (x0.min(x), x1.max(x), y0.min(y), y1.max(y));
            for (nx, ny) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if nx < w && ny < h && !seen[ny * w + nx] && on(img.data[ny * w + nx]) {
                    seen[ny * w + nx] = true;
                    stack.push(ny * w + nx);
                }
            }
        }
        if n < min_n || n > max_n {
            continue;
        }
        if found.len() >= limit {
            return Err(format!(
                "too_many_parts: more than {limit} spots in the picture (max_parts in vision.cfg)"
            ));
        }
        let (cx, cy) = (sx / n as f64, sy / n as f64);
        found.push(Blob {
            n,
            cx,
            cy,
            x0,
            x1,
            y0,
            y1,
            mxx: (sxx / n as f64 - cx * cx).max(0.0),
            mxy: sxy / n as f64 - cx * cy,
            myy: (syy / n as f64 - cy * cy).max(0.0),
        });
    }
    Ok(found)
}

/// A part, measured in millimetres from the centre of the picture, with x to the right and y up (the axes of the machine).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Part {
    pub x_mm: f64,
    pub y_mm: f64,
    /// The turn of the long side, from -90 (not included) to 90 (included) degrees, counter-clockwise from the x axis. A part as wide as long has no long side and gets 0.
    pub angle_deg: f64,
    /// The side along the turn (the longer one).
    pub length_mm: f64,
    pub width_mm: f64,
    pub area_mm2: f64,
}

/// What is asked of the search for parts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartSettings {
    pub polarity: Polarity,
    /// `None` is the threshold of Otsu.
    pub threshold: Option<u8>,
    /// The fewest and the most pixels a part has.
    pub min_area_px: u64,
    pub max_area_px: u64,
    /// The most parts in a picture.
    pub max_parts: usize,
}

impl Default for PartSettings {
    fn default() -> Self {
        PartSettings {
            polarity: Polarity::Dark,
            threshold: None,
            min_area_px: 100,
            max_area_px: 50_000_000,
            max_parts: 1000,
        }
    }
}

/// The turn (degrees, y up) and the two sides (pixels) of the rectangle that has the same second moments as a spot. For a filled rectangle of `a` by `b` pixels the variance along a side is
/// `(a * a - 1) / 12`, so the side is `sqrt(12 * variance + 1)`.
fn principal(b: &Blob) -> (f64, f64, f64) {
    let half = (b.mxx + b.myy) / 2.0;
    let spread = (((b.mxx - b.myy) / 2.0).powi(2) + b.mxy.powi(2)).sqrt();
    let (l1, l2) = (half + spread, (half - spread).max(0.0));
    let (long, short) = ((12.0 * l1 + 1.0).sqrt(), (12.0 * l2 + 1.0).sqrt());
    let mut angle = if spread < 1e-9 {
        0.0
    } else {
        // the picture goes down, the machine goes up: the turn changes sign
        -(0.5 * (2.0 * b.mxy).atan2(b.mxx - b.myy)).to_degrees()
    };
    if angle <= -90.0 {
        angle += 180.0;
    }
    if angle > 90.0 {
        angle -= 180.0;
    }
    (angle, long, short)
}

/// The parts of a picture in millimetres, the largest first. A spot that touches the edge of the picture is cut and is left out. The scale is the millimetres of a pixel.
pub fn measure_parts(img: &Gray, s: &PartSettings, scale: Scale) -> Result<Vec<Part>, String> {
    if !(scale.mm_per_px.is_finite() && scale.mm_per_px > 0.0) {
        return Err("the scale must be more than 0 millimetres a pixel".into());
    }
    if s.min_area_px == 0 || s.max_area_px < s.min_area_px || s.max_parts == 0 {
        return Err(
            "the area of a part must be from 1 pixel up and the most not below the fewest, and the most parts above 0"
                .into(),
        );
    }
    let t = s.threshold.unwrap_or_else(|| otsu(img));
    let polarity = s.polarity;
    let on = move |v: u8| match polarity {
        Polarity::Dark => v <= t,
        Polarity::Light => v > t,
    };
    if img.data.iter().all(|&v| v == img.data[0]) {
        return Ok(Vec::new());
    }
    let spots = blobs(img, &on, s.min_area_px, s.max_area_px, s.max_parts)?;
    let centre = (img.w as f64 / 2.0, img.h as f64 / 2.0);
    let mut parts: Vec<(u64, Part)> = spots
        .iter()
        .filter(|b| b.x0 > 0 && b.y0 > 0 && b.x1 + 1 < img.w && b.y1 + 1 < img.h)
        .map(|b| {
            let (x, y) = scale.point(b.cx, b.cy, centre);
            let (angle, long, short) = principal(b);
            (
                b.n,
                Part {
                    x_mm: x,
                    y_mm: y,
                    angle_deg: angle,
                    length_mm: scale.mm(long),
                    width_mm: scale.mm(short),
                    area_mm2: b.n as f64 * scale.mm_per_px * scale.mm_per_px,
                },
            )
        })
        .collect();
    parts.sort_by_key(|a| std::cmp::Reverse(a.0));
    Ok(parts.into_iter().map(|(_, p)| p).collect())
}

/// The name of the format and its version, in the JSON file.
pub const FORMAT: &str = "eh3d-vision-parts";
pub const FORMAT_VERSION: u32 = 1;
/// The line of column names of the CSV file.
pub const CSV_HEADER: &str = "index,x_mm,y_mm,angle_deg,length_mm,width_mm,area_mm2";

/// A number with fixed decimals, never `-0.000` (a value that rounds to nothing has no sign).
fn fixed(v: f64, decimals: usize) -> String {
    let s = format!("{v:.decimals$}");
    match s.strip_prefix('-') {
        Some(rest) if rest.chars().all(|c| c == '0' || c == '.') => rest.to_string(),
        _ => s,
    }
}

/// The parts as CSV: the line `CSV_HEADER` and then one line per part, `index` from 1, with a dot for the decimals and the line end `\n`.
pub fn to_csv(parts: &[Part]) -> String {
    let mut out = format!("{CSV_HEADER}\n");
    for (i, p) in parts.iter().enumerate() {
        out.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            i + 1,
            fixed(p.x_mm, 4),
            fixed(p.y_mm, 4),
            fixed(p.angle_deg, 3),
            fixed(p.length_mm, 4),
            fixed(p.width_mm, 4),
            fixed(p.area_mm2, 4)
        ));
    }
    out
}

/// The parts as JSON: an object with `format`, `version`, `unit` ("mm"), the size of the picture (`image_width_px`, `image_height_px`), `mm_per_px` and the list `parts` of objects with
/// `index` and the same fields as the columns of the CSV file.
pub fn to_json(parts: &[Part], image_w: usize, image_h: usize, scale: Scale) -> String {
    let mut out = format!(
        "{{\n  \"format\": \"{FORMAT}\",\n  \"version\": {FORMAT_VERSION},\n  \"unit\": \"mm\",\n  \"image_width_px\": {image_w},\n  \"image_height_px\": {image_h},\n  \"mm_per_px\": {:.8},\n  \"parts\": [",
        scale.mm_per_px
    );
    for (i, p) in parts.iter().enumerate() {
        out.push_str(&format!(
            "{}\n    {{\"index\": {}, \"x_mm\": {}, \"y_mm\": {}, \"angle_deg\": {}, \"length_mm\": {}, \"width_mm\": {}, \"area_mm2\": {}}}",
            if i == 0 { "" } else { "," },
            i + 1,
            fixed(p.x_mm, 4),
            fixed(p.y_mm, 4),
            fixed(p.angle_deg, 3),
            fixed(p.length_mm, 4),
            fixed(p.width_mm, 4),
            fixed(p.area_mm2, 4)
        ));
    }
    out.push_str(if parts.is_empty() {
        "]\n}\n"
    } else {
        "\n  ]\n}\n"
    });
    out
}

/// The parts of a CSV file written by `to_csv`. The first line must be `CSV_HEADER`; a line with another number of fields or a field that is not a number is an error that names the line.
pub fn parse_csv(text: &str) -> Result<Vec<Part>, String> {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    if lines.next() != Some(CSV_HEADER) {
        return Err(format!("the first line must be {CSV_HEADER}"));
    }
    lines
        .enumerate()
        .map(|(i, line)| {
            let fields: Vec<f64> = line
                .split(',')
                .map(|f| f.trim().parse::<f64>().ok().filter(|v| v.is_finite()))
                .collect::<Option<Vec<f64>>>()
                .ok_or_else(|| format!("line {}: a field is not a number", i + 2))?;
            if fields.len() != 7 {
                return Err(format!("line {}: {} fields, not 7", i + 2, fields.len()));
            }
            Ok(Part {
                x_mm: fields[1],
                y_mm: fields[2],
                angle_deg: fields[3],
                length_mm: fields[4],
                width_mm: fields[5],
                area_mm2: fields[6],
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A light board with dark rectangles `(cx, cy, length, width, turn in degrees as the machine sees it)`; a pixel is dark when its centre lies inside the turned rectangle.
    fn board(rects: &[(f64, f64, f64, f64, f64)]) -> Gray {
        let (w, h) = (240usize, 160usize);
        let mut g = Gray::blank(w, h, 220).unwrap();
        for y in 0..h {
            for x in 0..w {
                for &(cx, cy, len, wid, turn) in rects {
                    // the machine goes up, the picture goes down
                    let a = (-turn).to_radians();
                    let (dx, dy) = (x as f64 - cx, y as f64 - cy);
                    let along = dx * a.cos() + dy * a.sin();
                    let across = -dx * a.sin() + dy * a.cos();
                    if along.abs() <= len / 2.0 && across.abs() <= wid / 2.0 {
                        g.data[y * w + x] = 30;
                    }
                }
            }
        }
        g
    }

    fn half_mm() -> Scale {
        Scale { mm_per_px: 0.5 }
    }

    #[test]
    fn a_rectangle_gives_its_centre_turn_and_sides() {
        let img = board(&[(100.0, 60.0, 60.0, 30.0, 0.0)]);
        let parts = measure_parts(&img, &PartSettings::default(), half_mm()).unwrap();
        assert_eq!(parts.len(), 1);
        let p = parts[0];
        // the centre of the picture is (120, 80): x to the right, y up
        assert!((p.x_mm - (100.0 - 120.0) * 0.5).abs() < 0.05, "{p:?}");
        assert!((p.y_mm - (80.0 - 60.0) * 0.5).abs() < 0.05, "{p:?}");
        assert!(p.angle_deg.abs() < 0.5, "{p:?}");
        // 61 by 31 pixels of half a millimetre
        assert!(
            (p.length_mm - 30.5).abs() < 0.2 && (p.width_mm - 15.5).abs() < 0.2,
            "{p:?}"
        );
        assert!((p.area_mm2 - 472.75).abs() < 1.0, "{p:?}");
    }

    #[test]
    fn the_turn_follows_the_axes_of_the_machine() {
        for turn in [30.0, 60.0, 89.0, -30.0, -60.0] {
            let img = board(&[(120.0, 80.0, 70.0, 24.0, turn)]);
            let p = measure_parts(&img, &PartSettings::default(), half_mm()).unwrap()[0];
            assert!((p.angle_deg - turn).abs() < 1.5, "turn {turn}: {p:?}");
            assert!(p.length_mm > p.width_mm);
        }
        // a turn of 100 degrees is the same line as -80
        let img = board(&[(120.0, 80.0, 70.0, 24.0, 100.0)]);
        let p = measure_parts(&img, &PartSettings::default(), half_mm()).unwrap()[0];
        assert!((p.angle_deg + 80.0).abs() < 1.5, "{p:?}");
    }

    #[test]
    fn parts_come_largest_first_and_a_part_cut_by_the_edge_is_left_out() {
        let mut img = board(&[
            (60.0, 50.0, 30.0, 20.0, 0.0),
            (160.0, 100.0, 50.0, 30.0, 0.0),
        ]);
        // a part that touches the left edge
        for y in 60..90 {
            for x in 0..25 {
                img.data[y * 240 + x] = 30;
            }
        }
        let parts = measure_parts(&img, &PartSettings::default(), half_mm()).unwrap();
        assert_eq!(parts.len(), 2, "{parts:?}");
        assert!(parts[0].area_mm2 > parts[1].area_mm2);
        // only the area decides what is a part: a limit above the small one leaves it out
        let s = PartSettings {
            min_area_px: 1000,
            ..PartSettings::default()
        };
        assert_eq!(measure_parts(&img, &s, half_mm()).unwrap().len(), 1);
    }

    #[test]
    fn light_parts_and_a_flat_picture_and_settings_that_make_no_sense() {
        let mut g = Gray::blank(100, 100, 20).unwrap();
        for y in 30..60 {
            for x in 30..70 {
                g.data[y * 100 + x] = 230;
            }
        }
        let s = PartSettings {
            polarity: Polarity::Light,
            ..PartSettings::default()
        };
        assert_eq!(measure_parts(&g, &s, half_mm()).unwrap().len(), 1);
        let flat = Gray::blank(50, 50, 90).unwrap();
        assert!(measure_parts(&flat, &PartSettings::default(), half_mm())
            .unwrap()
            .is_empty());
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(measure_parts(&g, &s, Scale { mm_per_px: bad }).is_err());
        }
        for s in [
            PartSettings {
                min_area_px: 0,
                ..PartSettings::default()
            },
            PartSettings {
                min_area_px: 500,
                max_area_px: 100,
                ..PartSettings::default()
            },
            PartSettings {
                max_parts: 0,
                ..PartSettings::default()
            },
        ] {
            assert!(measure_parts(&g, &s, half_mm()).is_err());
        }
    }

    #[test]
    fn too_many_spots_are_an_error_and_not_a_flood() {
        // a chessboard of small squares: every one is a spot of 16 pixels
        let (w, h) = (160usize, 160usize);
        let mut g = Gray::blank(w, h, 220).unwrap();
        for y in 0..h {
            for x in 0..w {
                if (x / 4 + y / 4) % 2 == 0 {
                    g.data[y * w + x] = 30;
                }
            }
        }
        let s = PartSettings {
            min_area_px: 10,
            max_parts: 50,
            ..PartSettings::default()
        };
        let why = measure_parts(&g, &s, half_mm()).unwrap_err();
        assert!(why.starts_with("too_many_parts"), "{why}");
    }

    #[test]
    fn the_csv_and_the_json_say_the_same_and_the_csv_reads_back() {
        let parts = vec![
            Part {
                x_mm: -10.5,
                y_mm: 2.25,
                angle_deg: -12.5,
                length_mm: 30.0,
                width_mm: 15.0,
                area_mm2: 450.0,
            },
            Part {
                x_mm: 0.0,
                y_mm: 0.0,
                angle_deg: 0.0,
                length_mm: 5.0,
                width_mm: 5.0,
                area_mm2: 25.0,
            },
        ];
        let csv = to_csv(&parts);
        assert_eq!(csv.lines().next().unwrap(), CSV_HEADER);
        assert_eq!(csv.lines().count(), 3);
        assert_eq!(
            csv.lines().nth(1).unwrap(),
            "1,-10.5000,2.2500,-12.500,30.0000,15.0000,450.0000"
        );
        assert_eq!(parse_csv(&csv).unwrap(), parts);
        let json = to_json(&parts, 640, 480, half_mm());
        assert!(json.contains("\"format\": \"eh3d-vision-parts\""), "{json}");
        assert!(json.contains("\"version\": 1") && json.contains("\"unit\": \"mm\""));
        assert!(
            json.contains("\"image_width_px\": 640") && json.contains("\"mm_per_px\": 0.50000000")
        );
        assert!(json.contains("{\"index\": 2, \"x_mm\": 0.0000"), "{json}");
        assert_eq!(json.matches("\"index\"").count(), 2);
        // no parts is a list with nothing in it, not a missing list
        let empty = to_json(&[], 10, 10, half_mm());
        assert!(empty.contains("\"parts\": []"), "{empty}");
        assert_eq!(to_csv(&[]), format!("{CSV_HEADER}\n"));
        assert!(parse_csv(&to_csv(&[])).unwrap().is_empty());
    }

    #[test]
    fn a_value_that_rounds_to_nothing_has_no_sign() {
        assert_eq!(fixed(-0.0, 3), "0.000");
        assert_eq!(fixed(-0.00004, 4), "0.0000");
        assert_eq!(fixed(-0.5, 1), "-0.5");
        assert_eq!(fixed(12.0, 2), "12.00");
    }

    #[test]
    fn a_csv_that_is_not_ours_is_refused_with_its_line() {
        assert!(parse_csv("").is_err());
        assert!(parse_csv("a,b,c\n1,2,3\n")
            .unwrap_err()
            .contains("first line"));
        let bad = format!("{CSV_HEADER}\n1,2,3\n");
        assert!(parse_csv(&bad).unwrap_err().contains("line 2: 3 fields"));
        let nan = format!("{CSV_HEADER}\n1,0,0,0,x,1,1\n");
        assert!(parse_csv(&nan).unwrap_err().contains("line 2: a field"));
        // Windows line ends are read
        let crlf = format!("{CSV_HEADER}\r\n1,0,0,0,1,1,1\r\n");
        assert_eq!(parse_csv(&crlf).unwrap().len(), 1);
    }
}
