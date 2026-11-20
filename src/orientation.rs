//! Which way a part with a mark on it is held: a diode with its band, a chip with the dot of pin 1, a capacitor with its stripe. The part is the biggest spot that differs from the board, the
//! mark is what differs from the body inside it, and the direction from the middle of the part to the mark is the answer: in degrees counter-clockwise from the x axis with y up (the axes of
//! the machine), the quarter of a turn it is nearest to, how far it is from that quarter, and, when the way it should point is given, how far it is from that.

use crate::image::Gray;
use crate::inspect::json_string;
use crate::measure::fixed;

/// The name of the format and its version, in the JSON file.
pub const FORMAT: &str = "eh3d-vision-orientation";
pub const FORMAT_VERSION: u32 = 1;
/// The line of column names of the CSV file.
pub const CSV_HEADER: &str = "mark_px,mark_share,x_mm,y_mm,distance_mm,direction_deg,quarter_deg,skew_deg,error_deg,body_long_mm,body_short_mm,verdict,reason";

/// The reasons that a part is refused.
pub const REASON_NO_MARK: &str = "no_mark";
pub const REASON_CENTRED: &str = "centred";
pub const REASON_BIG: &str = "mark_big";
pub const REASON_WRONG_WAY: &str = "wrong_way";
pub const REASON_SKEWED: &str = "skewed";

/// The smallest picture that is looked at.
const MIN_SIDE: usize = 5;
/// A mark closer than this share of the shorter side of the part to its middle does not point anywhere.
const CENTRED_SHARE: f64 = 0.08;
/// A mark that fills more than this share of the part is not a mark.
const BIG_SHARE: f64 = 0.3;

/// What is asked of the search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientationSettings {
    /// How much a pixel must differ from the board (grey levels) to belong to the part.
    pub body_contrast: f64,
    /// How much a pixel inside the part must differ from the body (grey levels) to belong to the mark.
    pub mark_contrast: f64,
    /// The fewest pixels a mark has.
    pub min_mark_px: u64,
    /// How far from the wanted direction (or, with none, from a quarter of a turn) the mark may point, in degrees.
    pub tolerance_deg: f64,
    /// The direction the mark should point, counter-clockwise from the x axis with y up, when it is known.
    pub expected_deg: Option<f64>,
}

impl Default for OrientationSettings {
    fn default() -> Self {
        OrientationSettings {
            body_contrast: 30.0,
            mark_contrast: 40.0,
            min_mark_px: 4,
            tolerance_deg: 10.0,
            expected_deg: None,
        }
    }
}

/// Whether the settings make sense; the reason when they do not.
pub fn check(s: &OrientationSettings) -> Result<(), String> {
    for (v, name) in [
        (s.body_contrast, "orientation_body_contrast"),
        (s.mark_contrast, "orientation_mark_contrast"),
    ] {
        if !(v.is_finite() && (1.0..=255.0).contains(&v)) {
            return Err(format!("{name} must be from 1 to 255 grey levels"));
        }
    }
    if s.min_mark_px == 0 {
        return Err("orientation_min_mark_px must be 1 or more".into());
    }
    if !(s.tolerance_deg.is_finite() && (0.0..=90.0).contains(&s.tolerance_deg)) {
        return Err("orientation_tolerance_deg must be from 0 to 90".into());
    }
    if s.expected_deg.is_some_and(|d| !d.is_finite()) {
        return Err("the direction that is asked for must be a number".into());
    }
    Ok(())
}

/// What was found.
#[derive(Debug, Clone, PartialEq)]
pub struct Orientation {
    /// The pixels of the mark and the share of the part (0 to 1) they fill.
    pub mark_px: u64,
    pub mark_share: f64,
    /// Where the mark is from the middle of the part, in millimetres with y up, and how far.
    pub x_mm: f64,
    pub y_mm: f64,
    pub distance_mm: f64,
    /// The direction of the mark from the middle of the part, from -180 (not included) to 180 degrees; 0 when there is no mark.
    pub direction_deg: f64,
    /// The quarter of a turn nearest to it (0, 90, 180 or -90) and how far it is from that, from -45 (not included) to 45.
    pub quarter_deg: f64,
    pub skew_deg: f64,
    /// How far the direction is from the wanted one, from -180 (not included) to 180, when one was asked for.
    pub error_deg: Option<f64>,
    /// The sides of the part as the picture shows it, in millimetres.
    pub body_long_mm: f64,
    pub body_short_mm: f64,
    /// Why the part is refused; empty when it is good.
    pub reasons: Vec<&'static str>,
}

impl Orientation {
    pub fn passes(&self) -> bool {
        self.reasons.is_empty()
    }
}

/// An angle brought into (-180, 180].
pub fn wrap_deg(a: f64) -> f64 {
    let mut d = a % 360.0;
    if d <= -180.0 {
        d += 360.0;
    }
    if d > 180.0 {
        d -= 360.0;
    }
    d
}

/// The median of a list of numbers that is not empty (it is sorted here).
fn median(values: &mut [u8]) -> f64 {
    values.sort_unstable();
    let n = values.len();
    if n % 2 == 1 {
        f64::from(values[n / 2])
    } else {
        (f64::from(values[n / 2 - 1]) + f64::from(values[n / 2])) / 2.0
    }
}

/// The pixels of the biggest spot of the mask (four neighbours), as indices.
fn largest_component(mask: &[bool], w: usize, h: usize) -> Vec<usize> {
    let mut seen = vec![false; w * h];
    let mut best: Vec<usize> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for start in 0..w * h {
        if seen[start] || !mask[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let mut pixels = Vec::new();
        while let Some(i) = stack.pop() {
            pixels.push(i);
            let (x, y) = (i % w, i / w);
            for (nx, ny) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if nx < w && ny < h && !seen[ny * w + nx] && mask[ny * w + nx] {
                    seen[ny * w + nx] = true;
                    stack.push(ny * w + nx);
                }
            }
        }
        if pixels.len() > best.len() {
            best = pixels;
        }
    }
    best
}

/// Which way the part in the picture is held. The picture has one part on a plain board; `mm_per_px` is the scale. A picture with no part is `no_part`, a part that touches the edge of
/// the picture `part_cut`, a picture smaller than 5 by 5 pixels `picture_small`.
pub fn analyse(img: &Gray, mm_per_px: f64, s: &OrientationSettings) -> Result<Orientation, String> {
    check(s)?;
    if !(mm_per_px.is_finite() && mm_per_px > 0.0) {
        return Err("the scale must be more than 0 millimetres a pixel".into());
    }
    let (w, h) = (img.w, img.h);
    if w < MIN_SIDE || h < MIN_SIDE || img.data.len() != w * h {
        return Err("picture_small: the picture is smaller than 5 by 5 pixels".into());
    }
    // the board is the grey level the border of the picture has
    let mut border: Vec<u8> = Vec::with_capacity(2 * (w + h));
    for x in 0..w {
        border.push(img.data[x]);
        border.push(img.data[(h - 1) * w + x]);
    }
    for y in 1..h - 1 {
        border.push(img.data[y * w]);
        border.push(img.data[y * w + w - 1]);
    }
    let board = median(&mut border);
    let mask: Vec<bool> = img
        .data
        .iter()
        .map(|&v| (f64::from(v) - board).abs() >= s.body_contrast)
        .collect();
    let body = largest_component(&mask, w, h);
    if body.len() < 9 {
        return Err(
            "no_part: nothing in the picture differs from the board enough to be a part".into(),
        );
    }
    let (mut x0, mut x1, mut y0, mut y1) = (usize::MAX, 0usize, usize::MAX, 0usize);
    for &i in &body {
        let (x, y) = (i % w, i / w);
        (x0, x1, y0, y1) = (x0.min(x), x1.max(x), y0.min(y), y1.max(y));
    }
    if x0 == 0 || y0 == 0 || x1 + 1 == w || y1 + 1 == h {
        return Err("part_cut: the part touches the edge of the picture".into());
    }
    // the part is what lies between its first and its last pixel in every row and in every column, so a mark that looks like the board is inside it
    let (mut row_lo, mut row_hi) = (vec![usize::MAX; h], vec![0usize; h]);
    let (mut col_lo, mut col_hi) = (vec![usize::MAX; w], vec![0usize; w]);
    for &i in &body {
        let (x, y) = (i % w, i / w);
        row_lo[y] = row_lo[y].min(x);
        row_hi[y] = row_hi[y].max(x);
        col_lo[x] = col_lo[x].min(y);
        col_hi[x] = col_hi[x].max(y);
    }
    let mut level: Vec<u8> = body.iter().map(|&i| img.data[i]).collect();
    let body_level = median(&mut level);
    let (mut n_in, mut sx, mut sy) = (0u64, 0f64, 0f64);
    let (mut n_mark, mut mx, mut my) = (0u64, 0f64, 0f64);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let inside = row_lo[y] != usize::MAX
                && col_lo[x] != usize::MAX
                && (row_lo[y]..=row_hi[y]).contains(&x)
                && (col_lo[x]..=col_hi[x]).contains(&y);
            if !inside {
                continue;
            }
            n_in += 1;
            sx += x as f64;
            sy += y as f64;
            if (f64::from(img.data[y * w + x]) - body_level).abs() >= s.mark_contrast {
                n_mark += 1;
                mx += x as f64;
                my += y as f64;
            }
        }
    }
    let (cx, cy) = (sx / n_in as f64, sy / n_in as f64);
    let mark_share = n_mark as f64 / n_in as f64;
    let (box_w, box_h) = (
        (x1 - x0 + 1) as f64 * mm_per_px,
        (y1 - y0 + 1) as f64 * mm_per_px,
    );
    let (body_long_mm, body_short_mm) = (box_w.max(box_h), box_w.min(box_h));
    let mut reasons: Vec<&'static str> = Vec::new();
    let (mut x_mm, mut y_mm, mut direction_deg) = (0.0, 0.0, 0.0);
    if n_mark < s.min_mark_px {
        reasons.push(REASON_NO_MARK);
    } else {
        x_mm = (mx / n_mark as f64 - cx) * mm_per_px;
        // the picture goes down, the machine goes up
        y_mm = -(my / n_mark as f64 - cy) * mm_per_px;
        direction_deg = wrap_deg(y_mm.atan2(x_mm).to_degrees());
        if mark_share > BIG_SHARE {
            reasons.push(REASON_BIG);
        }
        if x_mm.hypot(y_mm) < CENTRED_SHARE * body_short_mm {
            reasons.push(REASON_CENTRED);
        }
    }
    let quarter_deg = wrap_deg((direction_deg / 90.0).round() * 90.0);
    let skew_deg = wrap_deg(direction_deg - quarter_deg);
    let error_deg = s.expected_deg.map(|want| wrap_deg(direction_deg - want));
    if !reasons.contains(&REASON_NO_MARK) && !reasons.contains(&REASON_CENTRED) {
        match error_deg {
            Some(e) if e.abs() > s.tolerance_deg => reasons.push(REASON_WRONG_WAY),
            None if skew_deg.abs() > s.tolerance_deg => reasons.push(REASON_SKEWED),
            _ => {}
        }
    }
    Ok(Orientation {
        mark_px: n_mark,
        mark_share,
        x_mm,
        y_mm,
        distance_mm: x_mm.hypot(y_mm),
        direction_deg,
        quarter_deg,
        skew_deg,
        error_deg,
        body_long_mm,
        body_short_mm,
        reasons,
    })
}

fn verdict(o: &Orientation) -> &'static str {
    if o.passes() {
        "pass"
    } else {
        "fail"
    }
}

fn error_text(o: &Orientation) -> String {
    o.error_deg.map_or_else(String::new, |e| fixed(e, 2))
}

/// The result as CSV: the line `CSV_HEADER` and one line with the numbers, the verdict (`pass` or `fail`) and the reasons joined with `+`.
pub fn to_csv(o: &Orientation) -> String {
    format!(
        "{CSV_HEADER}\n{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
        o.mark_px,
        fixed(o.mark_share, 4),
        fixed(o.x_mm, 4),
        fixed(o.y_mm, 4),
        fixed(o.distance_mm, 4),
        fixed(o.direction_deg, 2),
        fixed(o.quarter_deg, 0),
        fixed(o.skew_deg, 2),
        error_text(o),
        fixed(o.body_long_mm, 4),
        fixed(o.body_short_mm, 4),
        verdict(o),
        o.reasons.join("+")
    )
}

/// The result as JSON: an object with `format`, `version`, the fields of the columns of the CSV file (`error_deg` is `null` when no direction was asked for), the `verdict` and the list of
/// `reasons`.
pub fn to_json(o: &Orientation) -> String {
    let reasons: Vec<String> = o.reasons.iter().map(|r| json_string(r)).collect();
    let error = o
        .error_deg
        .map_or_else(|| "null".to_string(), |e| fixed(e, 2));
    format!(
        "{{\n  \"format\": \"{FORMAT}\",\n  \"version\": {FORMAT_VERSION},\n  \"mark_px\": {},\n  \"mark_share\": {},\n  \"x_mm\": {},\n  \"y_mm\": {},\n  \"distance_mm\": {},\n  \"direction_deg\": {},\n  \"quarter_deg\": {},\n  \"skew_deg\": {},\n  \"error_deg\": {error},\n  \"body_long_mm\": {},\n  \"body_short_mm\": {},\n  \"verdict\": \"{}\",\n  \"reasons\": [{}]\n}}\n",
        o.mark_px,
        fixed(o.mark_share, 4),
        fixed(o.x_mm, 4),
        fixed(o.y_mm, 4),
        fixed(o.distance_mm, 4),
        fixed(o.direction_deg, 2),
        fixed(o.quarter_deg, 0),
        fixed(o.skew_deg, 2),
        fixed(o.body_long_mm, 4),
        fixed(o.body_short_mm, 4),
        verdict(o),
        reasons.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOARD: u8 = 210;
    const BODY: u8 = 60;
    const DOT: u8 = 230;

    /// A picture of a body `long` by `short` pixels turned by `turn` degrees (counter-clockwise as the machine sees it) with a dot of radius `r` at `dot` pixels from the middle along the
    /// long side (positive towards the direction of the turn).
    fn part(turn: f64, long: f64, short: f64, dot: f64, r: f64) -> Gray {
        let (w, h) = (120usize, 120usize);
        let (cx, cy) = (60.0, 60.0);
        let t = turn.to_radians();
        let (ux, uy) = (t.cos(), -t.sin());
        let mut data = vec![BOARD; w * h];
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (x as f64 - cx, y as f64 - cy);
                let a = dx * ux + dy * uy;
                let b = -dx * uy + dy * ux;
                if a.abs() <= long / 2.0 && b.abs() <= short / 2.0 {
                    data[y * w + x] = BODY;
                    if (a - dot).hypot(b) <= r {
                        data[y * w + x] = DOT;
                    }
                }
            }
        }
        Gray::new(w, h, data).unwrap()
    }

    fn settings(expected: Option<f64>) -> OrientationSettings {
        OrientationSettings {
            expected_deg: expected,
            ..OrientationSettings::default()
        }
    }

    #[test]
    fn the_mark_tells_which_way_the_part_points_in_each_quarter() {
        for turn in [0.0, 90.0, 180.0, -90.0] {
            let o = analyse(&part(turn, 60.0, 30.0, 20.0, 4.0), 0.1, &settings(None)).unwrap();
            assert!(o.passes(), "{turn}: {:?}", o.reasons);
            assert!(
                wrap_deg(o.direction_deg - turn).abs() < 3.0,
                "{turn}: {}",
                o.direction_deg
            );
            assert_eq!(o.quarter_deg, turn);
            assert!(o.skew_deg.abs() < 3.0);
            assert!((o.distance_mm - 2.0).abs() < 0.15, "{}", o.distance_mm);
        }
    }

    #[test]
    fn the_direction_is_compared_with_the_wanted_one() {
        let img = part(0.0, 60.0, 30.0, 20.0, 4.0);
        let right = analyse(&img, 0.1, &settings(Some(0.0))).unwrap();
        assert!(right.passes());
        assert!(right.error_deg.unwrap().abs() < 3.0);
        let wrong = analyse(&img, 0.1, &settings(Some(180.0))).unwrap();
        assert_eq!(wrong.reasons, [REASON_WRONG_WAY]);
        assert!(wrong.error_deg.unwrap().abs() > 170.0);
        // a quarter of a turn off is also wrong, and a wanted direction is read round the circle
        assert!(!analyse(&img, 0.1, &settings(Some(90.0))).unwrap().passes());
        assert!(analyse(&img, 0.1, &settings(Some(360.0))).unwrap().passes());
        assert!(analyse(&img, 0.1, &settings(Some(-360.0)))
            .unwrap()
            .passes());
    }

    #[test]
    fn a_part_turned_off_the_quarters_is_skewed_unless_a_direction_is_asked_for() {
        let img = part(30.0, 60.0, 30.0, 20.0, 4.0);
        let o = analyse(&img, 0.1, &settings(None)).unwrap();
        assert_eq!(o.reasons, [REASON_SKEWED]);
        assert!((o.skew_deg - 30.0).abs() < 4.0, "{}", o.skew_deg);
        let a = analyse(&img, 0.1, &settings(Some(30.0))).unwrap();
        assert!(a.passes(), "{:?}", a.reasons);
        let loose = OrientationSettings {
            tolerance_deg: 40.0,
            ..OrientationSettings::default()
        };
        assert!(analyse(&img, 0.1, &loose).unwrap().passes());
    }

    #[test]
    fn a_part_with_no_mark_or_a_mark_in_the_middle_is_refused() {
        let plain = analyse(&part(0.0, 60.0, 30.0, 0.0, -1.0), 0.1, &settings(None)).unwrap();
        assert_eq!(plain.reasons, [REASON_NO_MARK]);
        assert_eq!(plain.mark_px, 0);
        assert_eq!(plain.direction_deg, 0.0);
        let middle = analyse(&part(0.0, 60.0, 30.0, 0.0, 4.0), 0.1, &settings(None)).unwrap();
        assert_eq!(middle.reasons, [REASON_CENTRED]);
        let mut covered = part(0.0, 40.0, 20.0, 5.0, 10.0);
        for v in &mut covered.data {
            if *v == DOT {
                // a mark that is not like the board, so the body stays in one piece
                *v = 140;
            }
        }
        let huge = analyse(&covered, 0.1, &settings(None)).unwrap();
        assert!(huge.reasons.contains(&REASON_BIG), "{:?}", huge.reasons);
    }

    #[test]
    fn what_cannot_be_looked_at_is_refused_with_its_code() {
        let ok = OrientationSettings::default();
        let blank = Gray::blank(40, 40, 100).unwrap();
        assert!(analyse(&blank, 0.1, &ok)
            .unwrap_err()
            .starts_with("no_part"));
        let tiny = Gray::blank(4, 4, 100).unwrap();
        assert!(analyse(&tiny, 0.1, &ok)
            .unwrap_err()
            .starts_with("picture_small"));
        // a spot that reaches the edge
        let mut wide = Gray::blank(60, 60, BOARD).unwrap();
        for y in 20..40 {
            for x in 0..60 {
                wide.data[y * 60 + x] = BODY;
            }
        }
        assert!(analyse(&wide, 0.1, &ok)
            .unwrap_err()
            .starts_with("part_cut"));
        assert!(analyse(&part(0.0, 60.0, 30.0, 20.0, 4.0), 0.0, &ok).is_err());
        assert!(analyse(&part(0.0, 60.0, 30.0, 20.0, 4.0), f64::NAN, &ok).is_err());
        for bad in [
            OrientationSettings {
                body_contrast: 0.0,
                ..ok
            },
            OrientationSettings {
                mark_contrast: 256.0,
                ..ok
            },
            OrientationSettings {
                min_mark_px: 0,
                ..ok
            },
            OrientationSettings {
                tolerance_deg: 91.0,
                ..ok
            },
            OrientationSettings {
                tolerance_deg: f64::NAN,
                ..ok
            },
            OrientationSettings {
                expected_deg: Some(f64::INFINITY),
                ..ok
            },
        ] {
            assert!(check(&bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_scale_and_the_mirror_of_the_picture_change_the_numbers_as_they_should() {
        let img = part(0.0, 60.0, 30.0, 20.0, 4.0);
        let a = analyse(&img, 0.1, &settings(None)).unwrap();
        let b = analyse(&img, 0.2, &settings(None)).unwrap();
        assert!((b.distance_mm - 2.0 * a.distance_mm).abs() < 1e-9);
        assert!((b.body_long_mm - 2.0 * a.body_long_mm).abs() < 1e-9);
        // the same part seen the other way up (the picture turned upside down) points the opposite way
        let mut flipped = img.clone();
        for y in 0..120 {
            for x in 0..120 {
                flipped.data[y * 120 + x] = img.data[(119 - y) * 120 + (119 - x)];
            }
        }
        let f = analyse(&flipped, 0.1, &settings(None)).unwrap();
        assert!(wrap_deg(f.direction_deg - a.direction_deg - 180.0).abs() < 3.0);
    }

    #[test]
    fn the_files_have_the_columns_and_the_reasons() {
        let img = part(0.0, 60.0, 30.0, 20.0, 4.0);
        let good = analyse(&img, 0.1, &settings(Some(0.0))).unwrap();
        let csv = to_csv(&good);
        let mut lines = csv.lines();
        assert_eq!(lines.next().unwrap(), CSV_HEADER);
        let row: Vec<&str> = lines.next().unwrap().split(',').collect();
        assert_eq!(row.len(), CSV_HEADER.split(',').count());
        assert_eq!(row[11], "pass");
        assert_eq!(row[12], "");
        let bad = analyse(&img, 0.1, &settings(Some(180.0))).unwrap();
        assert!(to_csv(&bad).trim_end().ends_with("fail,wrong_way"));
        let json = to_json(&bad);
        assert!(json.contains("\"format\": \"eh3d-vision-orientation\""));
        assert!(json.contains("\"reasons\": [\"wrong_way\"]"));
        assert!(json.contains("\"verdict\": \"fail\""));
        let none = analyse(&img, 0.1, &settings(None)).unwrap();
        assert!(to_json(&none).contains("\"error_deg\": null"));
        assert!(to_csv(&none)
            .lines()
            .nth(1)
            .unwrap()
            .split(',')
            .nth(8)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn angles_are_brought_into_half_open_range() {
        assert_eq!(wrap_deg(0.0), 0.0);
        assert_eq!(wrap_deg(180.0), 180.0);
        assert_eq!(wrap_deg(-180.0), 180.0);
        assert_eq!(wrap_deg(190.0), -170.0);
        assert_eq!(wrap_deg(-190.0), 170.0);
        assert_eq!(wrap_deg(720.0), 0.0);
        assert_eq!(wrap_deg(-540.0), 180.0);
    }
}
