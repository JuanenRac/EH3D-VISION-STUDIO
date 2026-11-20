//! The scale of the camera from a board of squares (a chessboard-like target, or a grid of square pads) with a report. From the squares that `squares` finds the turn of the board is worked out,
//! the squares are put in columns and rows, and the distance between neighbours along each axis and the side of the squares give the millimetres of a pixel along x, along y and from the
//! side. When the pitch (centre to centre) is known the scales along the two axes tell whether the pixels are square (`aspect`), when the side is known it is a second opinion; the numbers
//! that do not agree are refused instead of averaged.

use crate::inspect::json_string;
use crate::measure::fixed;
use crate::square::Square;

/// The name of the format and its version, in the JSON file.
pub const FORMAT: &str = "eh3d-vision-board";
pub const FORMAT_VERSION: u32 = 1;
/// The line of column names of the CSV file.
pub const CSV_HEADER: &str = "squares,columns,rows,angle_deg,side_px,pitch_x_px,pitch_y_px,spread_x,spread_y,mm_per_px_side,mm_per_px_x,mm_per_px_y,aspect,mm_per_px,verdict,reason";

/// The reasons that a board is refused.
pub const REASON_ASPECT: &str = "aspect";
pub const REASON_SPREAD: &str = "spread";
pub const REASON_DISAGREE: &str = "disagree";

/// The fewest squares a board needs, and the most that are looked at.
pub const MIN_SQUARES: usize = 4;
pub const MAX_SQUARES: usize = 4000;

/// What is known of the board and what is allowed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardSettings {
    /// The side of a square in millimetres, or 0 when it is not known.
    pub square_mm: f64,
    /// The distance between the centres of neighbouring squares in millimetres, or 0 when it is not known.
    pub pitch_mm: f64,
    /// How far the scales along x and along y may differ (0.02 is two percent).
    pub max_aspect: f64,
    /// How much the distances between neighbours may differ from each other, as a share of the median one; also how far the scale from the side may be from the one from the pitch.
    pub max_spread: f64,
}

impl Default for BoardSettings {
    fn default() -> Self {
        BoardSettings {
            square_mm: 0.0,
            pitch_mm: 0.0,
            max_aspect: 0.02,
            max_spread: 0.05,
        }
    }
}

/// Whether the settings make sense; the reason when they do not.
pub fn check(s: &BoardSettings) -> Result<(), String> {
    for (v, name) in [(s.square_mm, "board_square_mm"), (s.pitch_mm, "pitch_mm")] {
        if !(v.is_finite() && v >= 0.0) {
            return Err(format!("{name} must be 0 (not known) or more millimetres"));
        }
    }
    if s.square_mm == 0.0 && s.pitch_mm == 0.0 {
        return Err(
            "need_size: the board needs the side of a square (board_square_mm) or the distance between the centres of neighbours (pitch_mm)"
                .into(),
        );
    }
    if s.square_mm > 0.0 && s.pitch_mm > 0.0 && s.square_mm >= s.pitch_mm {
        return Err("the side of a square must be less than the distance between centres".into());
    }
    for (v, name) in [
        (s.max_aspect, "board_max_aspect"),
        (s.max_spread, "board_max_spread"),
    ] {
        if !(v.is_finite() && v > 0.0 && v <= 1.0) {
            return Err(format!("{name} must be more than 0 and at most 1"));
        }
    }
    Ok(())
}

/// What the board says.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardScale {
    pub squares: usize,
    pub columns: usize,
    pub rows: usize,
    /// How much the board is turned, from -45 (not included) to 45 degrees, as the machine sees it.
    pub angle_deg: f64,
    /// The median side of the squares in pixels.
    pub side_px: f64,
    /// The median distance between neighbours along each axis of the board, in pixels, and how much they differ (largest minus smallest over the median).
    pub pitch_x_px: f64,
    pub pitch_y_px: f64,
    pub spread_x: f64,
    pub spread_y: f64,
    /// The millimetres of a pixel from the side of the squares, along x and along y from the pitch, when those were given.
    pub mm_per_px_side: Option<f64>,
    pub mm_per_px_x: Option<f64>,
    pub mm_per_px_y: Option<f64>,
    /// The scale along x over the scale along y (1 is a square pixel), when the pitch was given.
    pub aspect: Option<f64>,
    /// The scale to use: the mean of the two axes when the pitch was given, else the one from the side.
    pub mm_per_px: f64,
    /// Why the board is refused; empty when it is good.
    pub reasons: Vec<&'static str>,
}

impl BoardScale {
    pub fn passes(&self) -> bool {
        self.reasons.is_empty()
    }
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.total_cmp(b));
    let n = values.len();
    if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    }
}

/// Groups the squares by the coordinate `v` (a new group starts where the gap to the last one is more than `gap`), each group sorted by `u`.
fn lines(items: &[(f64, f64)], gap: f64) -> Vec<Vec<f64>> {
    let mut sorted: Vec<(f64, f64)> = items.to_vec();
    sorted.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut groups: Vec<Vec<(f64, f64)>> = Vec::new();
    for p in sorted {
        match groups.last_mut() {
            Some(g) if p.1 - g[g.len() - 1].1 <= gap => g.push(p),
            _ => groups.push(vec![p]),
        }
    }
    groups
        .into_iter()
        .map(|mut g| {
            g.sort_by(|a, b| a.0.total_cmp(&b.0));
            g.into_iter().map(|p| p.0).collect()
        })
        .collect()
}

/// The distances between neighbours in the lines, with those that skip a square (about twice the usual one) or are not between neighbours dropped: the median and the spread.
fn pitch_of(groups: &[Vec<f64>]) -> Option<(f64, f64)> {
    let mut all: Vec<f64> = groups
        .iter()
        .flat_map(|g| g.windows(2).map(|w| w[1] - w[0]))
        .filter(|d| *d > 0.0)
        .collect();
    if all.is_empty() {
        return None;
    }
    let guess = median(&mut all.clone());
    all.retain(|d| (0.7 * guess..=1.3 * guess).contains(d));
    if all.is_empty() {
        return None;
    }
    let m = median(&mut all.clone());
    let (lo, hi) = all
        .iter()
        .fold((f64::INFINITY, 0f64), |(l, h), &d| (l.min(d), h.max(d)));
    Some((m, (hi - lo) / m))
}

/// The scale of the camera from the squares of a board. Fewer than 4 squares is `few_squares`, more than `MAX_SQUARES` `too_many_squares`, a board that is a single line of squares
/// `no_lattice` (the pitch along the other axis cannot be told), and a square or a place that is not a number is refused.
pub fn scale_from_squares(squares: &[Square], s: &BoardSettings) -> Result<BoardScale, String> {
    check(s)?;
    if squares.len() < MIN_SQUARES {
        return Err(format!(
            "few_squares: {} squares, the board needs at least {MIN_SQUARES}",
            squares.len()
        ));
    }
    if squares.len() > MAX_SQUARES {
        return Err(format!(
            "too_many_squares: {} squares, more than the {MAX_SQUARES} that are looked at",
            squares.len()
        ));
    }
    if squares.iter().any(|q| {
        !(q.x.is_finite()
            && q.y.is_finite()
            && q.side.is_finite()
            && q.side > 0.0
            && q.angle_deg.is_finite())
    }) {
        return Err("a square has a place or a size that is not a number".into());
    }
    // the turn of the board: the mean of the turns of the squares, taken a quarter of a turn at a time
    let (sin4, cos4) = squares.iter().fold((0f64, 0f64), |(s, c), q| {
        let a = (4.0 * q.angle_deg).to_radians();
        (s + a.sin(), c + a.cos())
    });
    let angle_deg = sin4.atan2(cos4).to_degrees() / 4.0;
    // the picture goes down, the machine goes up: the axes of the board in the picture are turned the other way
    let phi = (-angle_deg).to_radians();
    let (c, sn) = (phi.cos(), phi.sin());
    let along: Vec<(f64, f64)> = squares
        .iter()
        .map(|q| (q.x * c + q.y * sn, -q.x * sn + q.y * c))
        .collect();
    let mut sides: Vec<f64> = squares.iter().map(|q| q.side).collect();
    let side_px = median(&mut sides);
    let gap = 0.5 * side_px;
    // rows: squares with nearly the same v, ordered by u; columns: the other way round
    let rows = lines(&along, gap);
    let swapped: Vec<(f64, f64)> = along.iter().map(|p| (p.1, p.0)).collect();
    let columns = lines(&swapped, gap);
    let (Some((pitch_x_px, spread_x)), Some((pitch_y_px, spread_y))) =
        (pitch_of(&rows), pitch_of(&columns))
    else {
        return Err(
            "no_lattice: the squares do not make rows and columns with neighbours on both axes (a single line of squares cannot tell the scale along the other axis)"
                .into(),
        );
    };
    let mm_per_px_side = (s.square_mm > 0.0).then(|| s.square_mm / side_px);
    let (mm_per_px_x, mm_per_px_y) = if s.pitch_mm > 0.0 {
        (Some(s.pitch_mm / pitch_x_px), Some(s.pitch_mm / pitch_y_px))
    } else {
        (None, None)
    };
    let aspect = mm_per_px_x.zip(mm_per_px_y).map(|(x, y)| x / y);
    let mm_per_px = match (mm_per_px_x, mm_per_px_y, mm_per_px_side) {
        (Some(x), Some(y), _) => (x + y) / 2.0,
        (_, _, Some(side)) => side,
        _ => unreachable!("check() needs a side or a pitch"),
    };
    let mut reasons: Vec<&'static str> = Vec::new();
    if aspect.is_some_and(|a| (a - 1.0).abs() > s.max_aspect) {
        reasons.push(REASON_ASPECT);
    }
    let pitch_known = s.pitch_mm > 0.0;
    if pitch_known && (spread_x > s.max_spread || spread_y > s.max_spread) {
        reasons.push(REASON_SPREAD);
    }
    if let (Some(side), true) = (mm_per_px_side, pitch_known) {
        if (side / mm_per_px - 1.0).abs() > s.max_spread {
            reasons.push(REASON_DISAGREE);
        }
    }
    let sides_spread = sides
        .last()
        .zip(sides.first())
        .map_or(0.0, |(hi, lo)| (hi - lo) / side_px);
    if !pitch_known && sides_spread > s.max_spread {
        reasons.push(REASON_SPREAD);
    }
    Ok(BoardScale {
        squares: squares.len(),
        columns: columns.len(),
        rows: rows.len(),
        angle_deg,
        side_px,
        pitch_x_px,
        pitch_y_px,
        spread_x,
        spread_y,
        mm_per_px_side,
        mm_per_px_x,
        mm_per_px_y,
        aspect,
        mm_per_px,
        reasons,
    })
}

fn verdict(b: &BoardScale) -> &'static str {
    if b.passes() {
        "pass"
    } else {
        "fail"
    }
}

fn opt(v: Option<f64>, decimals: usize) -> String {
    v.map_or_else(String::new, |x| fixed(x, decimals))
}

/// The result as CSV: the line `CSV_HEADER` and one line with the numbers (a scale that was not asked for is empty), the verdict (`pass` or `fail`) and the reasons joined with `+`.
pub fn to_csv(b: &BoardScale) -> String {
    format!(
        "{CSV_HEADER}\n{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
        b.squares,
        b.columns,
        b.rows,
        fixed(b.angle_deg, 3),
        fixed(b.side_px, 3),
        fixed(b.pitch_x_px, 3),
        fixed(b.pitch_y_px, 3),
        fixed(b.spread_x, 4),
        fixed(b.spread_y, 4),
        opt(b.mm_per_px_side, 6),
        opt(b.mm_per_px_x, 6),
        opt(b.mm_per_px_y, 6),
        opt(b.aspect, 4),
        fixed(b.mm_per_px, 6),
        verdict(b),
        b.reasons.join("+")
    )
}

/// The result as JSON: an object with `format`, `version`, the fields of the columns of the CSV file (`null` for a scale that was not asked for), the `verdict` and the list of `reasons`.
pub fn to_json(b: &BoardScale) -> String {
    let reasons: Vec<String> = b.reasons.iter().map(|r| json_string(r)).collect();
    let n = |v: Option<f64>, d: usize| v.map_or_else(|| "null".to_string(), |x| fixed(x, d));
    format!(
        "{{\n  \"format\": \"{FORMAT}\",\n  \"version\": {FORMAT_VERSION},\n  \"squares\": {},\n  \"columns\": {},\n  \"rows\": {},\n  \"angle_deg\": {},\n  \"side_px\": {},\n  \"pitch_x_px\": {},\n  \"pitch_y_px\": {},\n  \"spread_x\": {},\n  \"spread_y\": {},\n  \"mm_per_px_side\": {},\n  \"mm_per_px_x\": {},\n  \"mm_per_px_y\": {},\n  \"aspect\": {},\n  \"mm_per_px\": {},\n  \"verdict\": \"{}\",\n  \"reasons\": [{}]\n}}\n",
        b.squares,
        b.columns,
        b.rows,
        fixed(b.angle_deg, 3),
        fixed(b.side_px, 3),
        fixed(b.pitch_x_px, 3),
        fixed(b.pitch_y_px, 3),
        fixed(b.spread_x, 4),
        fixed(b.spread_y, 4),
        n(b.mm_per_px_side, 6),
        n(b.mm_per_px_x, 6),
        n(b.mm_per_px_y, 6),
        n(b.aspect, 4),
        fixed(b.mm_per_px, 6),
        verdict(b),
        reasons.join(", ")
    )
}

/// The report for a person: what was found, the scale along each axis, how far the pixels are from square, how much the squares differ and the verdict, and the line to put in `vision.cfg`.
pub fn report(b: &BoardScale, s: &BoardSettings) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Board: {} squares in {} columns and {} rows, turned {:+.2} degrees\n",
        b.squares, b.columns, b.rows, b.angle_deg
    ));
    out.push_str(&format!(
        "Squares: side {:.2} px (median), neighbours {:.2} px apart along x and {:.2} px along y\n",
        b.side_px, b.pitch_x_px, b.pitch_y_px
    ));
    out.push_str(&format!(
        "Spread of the distances between neighbours: {:.2} % along x, {:.2} % along y (allowed {:.2} %)\n",
        b.spread_x * 100.0,
        b.spread_y * 100.0,
        s.max_spread * 100.0
    ));
    if let Some(side) = b.mm_per_px_side {
        out.push_str(&format!(
            "From the side of the squares ({} mm): {side:.6} mm per pixel\n",
            s.square_mm
        ));
    }
    if let (Some(x), Some(y), Some(a)) = (b.mm_per_px_x, b.mm_per_px_y, b.aspect) {
        out.push_str(&format!(
            "From the pitch ({} mm): {x:.6} mm per pixel along x, {y:.6} along y, x over y {a:.4} (allowed {:.2} % off 1)\n",
            s.pitch_mm,
            s.max_aspect * 100.0
        ));
    }
    out.push_str(&format!(
        "Verdict: {}\n",
        if b.passes() {
            "good".to_string()
        } else {
            format!("refused ({})", b.reasons.join(", "))
        }
    ));
    out.push_str(&format!("mm_per_px = {:.6}\n", b.mm_per_px));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A board of `cols` by `rows` squares of `side` pixels, `pitch_x` and `pitch_y` pixels apart, turned by `turn` degrees (as the machine sees it) about the first one.
    fn board(
        cols: usize,
        rows: usize,
        side: f64,
        pitch_x: f64,
        pitch_y: f64,
        turn: f64,
    ) -> Vec<Square> {
        let t = (-turn).to_radians();
        let mut out = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                let (u, v) = (c as f64 * pitch_x, r as f64 * pitch_y);
                out.push(Square {
                    x: 100.0 + u * t.cos() - v * t.sin(),
                    y: 100.0 + u * t.sin() + v * t.cos(),
                    side,
                    angle_deg: turn,
                    score: 1.0,
                });
            }
        }
        out
    }

    fn known(square_mm: f64, pitch_mm: f64) -> BoardSettings {
        BoardSettings {
            square_mm,
            pitch_mm,
            ..BoardSettings::default()
        }
    }

    #[test]
    fn a_square_on_board_gives_the_scale_from_the_pitch_and_from_the_side() {
        let b = scale_from_squares(&board(5, 4, 40.0, 50.0, 50.0, 0.0), &known(4.0, 5.0)).unwrap();
        assert!(b.passes(), "{:?}", b.reasons);
        assert_eq!((b.squares, b.columns, b.rows), (20, 5, 4));
        assert!((b.pitch_x_px - 50.0).abs() < 1e-9 && (b.pitch_y_px - 50.0).abs() < 1e-9);
        assert!((b.mm_per_px - 0.1).abs() < 1e-9);
        assert!((b.mm_per_px_side.unwrap() - 0.1).abs() < 1e-9);
        assert!((b.aspect.unwrap() - 1.0).abs() < 1e-9);
        assert!(b.angle_deg.abs() < 1e-9);
        assert!(b.spread_x < 1e-9 && b.spread_y < 1e-9);
    }

    #[test]
    fn the_turn_of_the_board_does_not_change_the_scale() {
        for turn in [-30.0, -12.0, 5.0, 20.0, 40.0] {
            let b =
                scale_from_squares(&board(4, 4, 40.0, 50.0, 50.0, turn), &known(0.0, 5.0)).unwrap();
            assert!(b.passes(), "{turn}: {:?}", b.reasons);
            assert!((b.angle_deg - turn).abs() < 1e-6, "{turn}: {}", b.angle_deg);
            assert!((b.mm_per_px - 0.1).abs() < 1e-6, "{turn}: {}", b.mm_per_px);
            assert!((b.aspect.unwrap() - 1.0).abs() < 1e-6, "{turn}");
        }
    }

    #[test]
    fn pixels_that_are_not_square_are_told_by_the_aspect() {
        let b = scale_from_squares(&board(5, 4, 40.0, 50.0, 55.0, 0.0), &known(0.0, 5.0)).unwrap();
        assert_eq!(b.reasons, [REASON_ASPECT]);
        assert!((b.aspect.unwrap() - 55.0 / 50.0).abs() < 1e-9);
        assert!((b.mm_per_px_x.unwrap() - 0.1).abs() < 1e-9);
        assert!((b.mm_per_px_y.unwrap() - 5.0 / 55.0).abs() < 1e-9);
        // a looser aspect lets it pass
        let loose = BoardSettings {
            max_aspect: 0.2,
            ..known(0.0, 5.0)
        };
        assert!(
            scale_from_squares(&board(5, 4, 40.0, 50.0, 55.0, 0.0), &loose)
                .unwrap()
                .passes()
        );
    }

    #[test]
    fn a_side_that_does_not_agree_with_the_pitch_is_refused() {
        // the squares are 40 px of a pitch of 50, so a side of 3 mm for a pitch of 5 mm is not the same board
        let b = scale_from_squares(&board(4, 4, 40.0, 50.0, 50.0, 0.0), &known(3.0, 5.0)).unwrap();
        assert_eq!(b.reasons, [REASON_DISAGREE]);
        // with the side only, the scale is the one of the side
        let only =
            scale_from_squares(&board(4, 4, 40.0, 50.0, 50.0, 0.0), &known(3.0, 0.0)).unwrap();
        assert!(only.passes());
        assert!((only.mm_per_px - 0.075).abs() < 1e-9);
        assert!(only.aspect.is_none() && only.mm_per_px_x.is_none());
    }

    #[test]
    fn a_missing_square_does_not_move_the_pitch_but_an_uneven_board_is_refused() {
        let mut squares = board(6, 4, 40.0, 50.0, 50.0, 0.0);
        squares.remove(8);
        let b = scale_from_squares(&squares, &known(0.0, 5.0)).unwrap();
        assert!(b.passes(), "{:?}", b.reasons);
        assert!((b.pitch_x_px - 50.0).abs() < 1e-9);
        // a lens that bends the board: the squares of the outer column are pushed in
        let mut bent = board(6, 4, 40.0, 50.0, 50.0, 0.0);
        for q in &mut bent {
            if q.x > 330.0 {
                q.x -= 8.0;
            }
        }
        let b = scale_from_squares(&bent, &known(0.0, 5.0)).unwrap();
        assert_eq!(b.reasons, [REASON_SPREAD]);
        assert!(b.spread_x > 0.05);
    }

    #[test]
    fn what_is_not_a_board_is_refused_with_its_code() {
        let ok = known(0.0, 5.0);
        let three = board(3, 1, 40.0, 50.0, 50.0, 0.0);
        assert!(scale_from_squares(&three, &ok)
            .unwrap_err()
            .starts_with("few_squares"));
        let line = board(6, 1, 40.0, 50.0, 50.0, 0.0);
        assert!(scale_from_squares(&line, &ok)
            .unwrap_err()
            .starts_with("no_lattice"));
        let many = vec![board(1, 1, 40.0, 50.0, 50.0, 0.0)[0]; MAX_SQUARES + 1];
        assert!(scale_from_squares(&many, &ok)
            .unwrap_err()
            .starts_with("too_many_squares"));
        let mut nan = board(3, 3, 40.0, 50.0, 50.0, 0.0);
        nan[2].x = f64::NAN;
        assert!(scale_from_squares(&nan, &ok).is_err());
        let mut flat = board(3, 3, 40.0, 50.0, 50.0, 0.0);
        flat[0].side = 0.0;
        assert!(scale_from_squares(&flat, &ok).is_err());
        let fine = board(3, 3, 40.0, 50.0, 50.0, 0.0);
        assert!(scale_from_squares(&fine, &BoardSettings::default())
            .unwrap_err()
            .starts_with("need_size"));
        for bad in [
            known(-1.0, 5.0),
            known(f64::NAN, 5.0),
            known(5.0, 5.0),
            known(6.0, 5.0),
            BoardSettings {
                max_aspect: 0.0,
                ..ok
            },
            BoardSettings {
                max_spread: 1.5,
                ..ok
            },
            BoardSettings {
                max_spread: f64::NAN,
                ..ok
            },
        ] {
            assert!(check(&bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_files_and_the_report_say_the_same_things() {
        let s = known(4.0, 5.0);
        let b = scale_from_squares(&board(5, 4, 40.0, 50.0, 55.0, 0.0), &s).unwrap();
        let csv = to_csv(&b);
        let mut lines = csv.lines();
        assert_eq!(lines.next().unwrap(), CSV_HEADER);
        let row: Vec<&str> = lines.next().unwrap().split(',').collect();
        assert_eq!(row.len(), CSV_HEADER.split(',').count());
        assert_eq!(&row[..3], ["20", "5", "4"]);
        assert_eq!(&row[14..], ["fail", "aspect"]);
        let json = to_json(&b);
        assert!(json.contains("\"format\": \"eh3d-vision-board\""));
        assert!(json.contains("\"verdict\": \"fail\""));
        let side_only =
            scale_from_squares(&board(5, 4, 40.0, 50.0, 50.0, 0.0), &known(4.0, 0.0)).unwrap();
        assert!(to_json(&side_only).contains("\"aspect\": null"));
        assert!(to_csv(&side_only)
            .lines()
            .nth(1)
            .unwrap()
            .split(',')
            .nth(12)
            .unwrap()
            .is_empty());
        let text = report(&b, &s);
        assert!(text.contains("20 squares in 5 columns and 4 rows"));
        assert!(text.contains("refused (aspect)"));
        assert!(text
            .trim_end()
            .ends_with(&format!("mm_per_px = {:.6}", b.mm_per_px)));
        let good = scale_from_squares(&board(5, 4, 40.0, 50.0, 50.0, 0.0), &s).unwrap();
        assert!(report(&good, &s).contains("Verdict: good"));
    }
}
