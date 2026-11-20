//! A map of where the picture is sharp: the picture is cut into a grid of cells and the sharpness of each one (the variance of the Laplacian, as `quality` measures the whole picture) is
//! worked out, so that a camera that is out of focus at one side, a lens that is tilted or a dirty window show as a cold corner. A cell with almost no contrast has nothing to focus on, so
//! it is `empty` and is not judged; the others are `weak` when they are much less sharp than the median cell.

use crate::image::Gray;
use crate::inspect::json_string;
use crate::measure::fixed;

/// The name of the format and its version, in the JSON file.
pub const FORMAT: &str = "eh3d-vision-sharpmap";
pub const FORMAT_VERSION: u32 = 1;
/// The line of column names of the CSV file.
pub const CSV_HEADER: &str = "col,row,x0,y0,x1,y1,sharpness,contrast,state";

/// The reason that a picture is refused.
pub const REASON_UNEVEN: &str = "uneven";

/// The fewest and the most cells on a side, and the fewest pixels a cell has on a side (the Laplacian needs a pixel and its four neighbours).
pub const MIN_CELLS: usize = 2;
pub const MAX_CELLS: usize = 64;
pub const MIN_CELL_PX: usize = 3;

/// What is asked of the map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SharpSettings {
    /// The cells on each side (the picture is cut into this many columns and this many rows).
    pub cells: usize,
    /// A cell is weak when its sharpness is below this share of the median one (more than 0, at most 1).
    pub min_share: f64,
    /// A cell whose grey levels have a standard deviation below this has nothing to focus on and is empty.
    pub min_contrast: f64,
}

impl Default for SharpSettings {
    fn default() -> Self {
        SharpSettings {
            cells: 8,
            min_share: 0.5,
            min_contrast: 3.0,
        }
    }
}

/// Whether the settings make sense; the reason when they do not.
pub fn check(s: &SharpSettings) -> Result<(), String> {
    if !(MIN_CELLS..=MAX_CELLS).contains(&s.cells) {
        return Err(format!(
            "sharpmap_cells must be from {MIN_CELLS} to {MAX_CELLS}"
        ));
    }
    if !(s.min_share.is_finite() && s.min_share > 0.0 && s.min_share <= 1.0) {
        return Err("sharpmap_min_share must be more than 0 and at most 1".into());
    }
    if !(s.min_contrast.is_finite() && (0.0..=255.0).contains(&s.min_contrast)) {
        return Err("sharpmap_min_contrast must be from 0 to 255 grey levels".into());
    }
    Ok(())
}

/// What a cell is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Ok,
    Weak,
    Empty,
}

impl State {
    pub fn word(self) -> &'static str {
        match self {
            State::Ok => "ok",
            State::Weak => "weak",
            State::Empty => "empty",
        }
    }
}

/// One cell of the map: its place in the grid, the pixels it covers (`x1` and `y1` are the first pixel after it) and what was measured in it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    pub col: usize,
    pub row: usize,
    pub x0: usize,
    pub y0: usize,
    pub x1: usize,
    pub y1: usize,
    pub sharpness: f64,
    pub contrast: f64,
    pub state: State,
}

/// The map.
#[derive(Debug, Clone, PartialEq)]
pub struct SharpMap {
    pub cells: Vec<Cell>,
    pub columns: usize,
    pub rows: usize,
    /// The median, the largest and the smallest sharpness of the cells that are not empty.
    pub median: f64,
    pub max: f64,
    pub min: f64,
    /// The smallest over the median: 1 is an even picture.
    pub uniformity: f64,
    /// How much the sharpness changes from the left of the picture to the right and from the top to the bottom, as a share of the median (a positive number: sharper at the right or at the
    /// bottom). A tilted lens or a camera not square to the work gives a big number on one axis.
    pub tilt_x: f64,
    pub tilt_y: f64,
    pub weak: usize,
    pub empty: usize,
    /// Why the picture is refused (`uneven`); empty when it is good.
    pub reasons: Vec<&'static str>,
}

impl SharpMap {
    pub fn passes(&self) -> bool {
        self.reasons.is_empty()
    }
}

fn median_of(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

/// The slope of the least squares line through `(position, value)` pairs, or 0 when the positions are all the same.
fn slope(points: &[(f64, f64)]) -> f64 {
    let n = points.len() as f64;
    let (mx, my) = (
        points.iter().map(|p| p.0).sum::<f64>() / n,
        points.iter().map(|p| p.1).sum::<f64>() / n,
    );
    let (mut sxx, mut sxy) = (0.0, 0.0);
    for p in points {
        sxx += (p.0 - mx) * (p.0 - mx);
        sxy += (p.0 - mx) * (p.1 - my);
    }
    if sxx > 0.0 {
        sxy / sxx
    } else {
        0.0
    }
}

/// The map of a picture. A picture whose cells would be below 3 pixels on a side is `picture_small`; one where every cell is empty is `flat`.
pub fn sharp_map(img: &Gray, s: &SharpSettings) -> Result<SharpMap, String> {
    check(s)?;
    let (w, h) = (img.w, img.h);
    if img.data.len() != w * h || w / s.cells < MIN_CELL_PX || h / s.cells < MIN_CELL_PX {
        return Err(format!(
            "picture_small: {} cells on a side need a picture of at least {} by {} pixels",
            s.cells,
            s.cells * MIN_CELL_PX,
            s.cells * MIN_CELL_PX
        ));
    }
    let n = s.cells;
    // the edges of the cells: the pixels are shared out as evenly as they go
    let edge = |len: usize, k: usize| k * len / n;
    let at = |x: usize, y: usize| i32::from(img.data[y * w + x]);
    let mut cells = Vec::with_capacity(n * n);
    for row in 0..n {
        for col in 0..n {
            let (x0, x1, y0, y1) = (
                edge(w, col),
                edge(w, col + 1),
                edge(h, row),
                edge(h, row + 1),
            );
            let (mut sum, mut sum_sq, mut count) = (0f64, 0f64, 0f64);
            let (mut lap_sum, mut lap_sq, mut lap_n) = (0f64, 0f64, 0f64);
            for y in y0..y1 {
                for x in x0..x1 {
                    let v = f64::from(img.data[y * w + x]);
                    sum += v;
                    sum_sq += v * v;
                    count += 1.0;
                    if x >= 1 && y >= 1 && x + 1 < w && y + 1 < h {
                        let l = f64::from(
                            at(x - 1, y) + at(x + 1, y) + at(x, y - 1) + at(x, y + 1)
                                - 4 * at(x, y),
                        );
                        lap_sum += l;
                        lap_sq += l * l;
                        lap_n += 1.0;
                    }
                }
            }
            let mean = sum / count;
            let contrast = (sum_sq / count - mean * mean).max(0.0).sqrt();
            let lap_mean = lap_sum / lap_n;
            let sharpness = (lap_sq / lap_n - lap_mean * lap_mean).max(0.0);
            let state = if contrast < s.min_contrast {
                State::Empty
            } else {
                State::Ok
            };
            cells.push(Cell {
                col,
                row,
                x0,
                y0,
                x1,
                y1,
                sharpness,
                contrast,
                state,
            });
        }
    }
    let mut live: Vec<f64> = cells
        .iter()
        .filter(|c| c.state != State::Empty)
        .map(|c| c.sharpness)
        .collect();
    if live.is_empty() {
        return Err("flat: no cell of the picture has enough contrast to be judged".into());
    }
    live.sort_by(|a, b| a.total_cmp(b));
    let median = median_of(&live);
    let (min, max) = (live[0], live[live.len() - 1]);
    let mut weak = 0;
    for c in &mut cells {
        if c.state == State::Ok && c.sharpness < s.min_share * median {
            c.state = State::Weak;
            weak += 1;
        }
    }
    let empty = cells.iter().filter(|c| c.state == State::Empty).count();
    let along = |key: &dyn Fn(&Cell) -> f64| -> f64 {
        let pts: Vec<(f64, f64)> = cells
            .iter()
            .filter(|c| c.state != State::Empty)
            .map(|c| (key(c), c.sharpness))
            .collect();
        if median > 0.0 {
            slope(&pts) * (n - 1) as f64 / median
        } else {
            0.0
        }
    };
    let (tilt_x, tilt_y) = (along(&|c| c.col as f64), along(&|c| c.row as f64));
    Ok(SharpMap {
        cells,
        columns: n,
        rows: n,
        median,
        max,
        min,
        uniformity: if median > 0.0 { min / median } else { 0.0 },
        tilt_x,
        tilt_y,
        weak,
        empty,
        reasons: if weak > 0 {
            vec![REASON_UNEVEN]
        } else {
            Vec::new()
        },
    })
}

/// The picture of the map, as large as the picture it was made from: every pixel has the grey level of its cell, 255 for the sharpest cell and 0 for an empty one, so that a cold corner is
/// dark.
pub fn heat(map: &SharpMap, w: usize, h: usize) -> Result<Gray, String> {
    let mut data = vec![0u8; w * h];
    for c in &map.cells {
        let level = if c.state == State::Empty || map.max <= 0.0 {
            0
        } else {
            (255.0 * c.sharpness / map.max).round().clamp(0.0, 255.0) as u8
        };
        for y in c.y0.min(h)..c.y1.min(h) {
            for x in c.x0.min(w)..c.x1.min(w) {
                data[y * w + x] = level;
            }
        }
    }
    Gray::new(w, h, data)
}

fn verdict(m: &SharpMap) -> &'static str {
    if m.passes() {
        "pass"
    } else {
        "fail"
    }
}

/// The result as CSV: the line `CSV_HEADER` and one line for each cell, from the top row down, each from left to right, with the state `ok`, `weak` or `empty`.
pub fn to_csv(m: &SharpMap) -> String {
    let mut out = format!("{CSV_HEADER}\n");
    for c in &m.cells {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            c.col + 1,
            c.row + 1,
            c.x0,
            c.y0,
            c.x1,
            c.y1,
            fixed(c.sharpness, 3),
            fixed(c.contrast, 3),
            c.state.word()
        ));
    }
    out
}

/// The result as JSON: an object with `format`, `version`, the size of the grid, the median, the largest and the smallest sharpness, the uniformity, the tilt along x and y, the number of
/// weak and of empty cells, the `verdict`, the list of `reasons` and the list of `cells` with the fields of the columns of the CSV file.
pub fn to_json(m: &SharpMap) -> String {
    let reasons: Vec<String> = m.reasons.iter().map(|r| json_string(r)).collect();
    let cells: Vec<String> = m
        .cells
        .iter()
        .map(|c| {
            format!(
                "    {{\"col\": {}, \"row\": {}, \"x0\": {}, \"y0\": {}, \"x1\": {}, \"y1\": {}, \"sharpness\": {}, \"contrast\": {}, \"state\": \"{}\"}}",
                c.col + 1,
                c.row + 1,
                c.x0,
                c.y0,
                c.x1,
                c.y1,
                fixed(c.sharpness, 3),
                fixed(c.contrast, 3),
                c.state.word()
            )
        })
        .collect();
    format!(
        "{{\n  \"format\": \"{FORMAT}\",\n  \"version\": {FORMAT_VERSION},\n  \"columns\": {},\n  \"rows\": {},\n  \"median\": {},\n  \"max\": {},\n  \"min\": {},\n  \"uniformity\": {},\n  \"tilt_x\": {},\n  \"tilt_y\": {},\n  \"weak\": {},\n  \"empty\": {},\n  \"verdict\": \"{}\",\n  \"reasons\": [{}],\n  \"cells\": [\n{}\n  ]\n}}\n",
        m.columns,
        m.rows,
        fixed(m.median, 3),
        fixed(m.max, 3),
        fixed(m.min, 3),
        fixed(m.uniformity, 4),
        fixed(m.tilt_x, 4),
        fixed(m.tilt_y, 4),
        m.weak,
        m.empty,
        verdict(m),
        reasons.join(", "),
        cells.join(",\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture of a fine checker pattern whose contrast is `contrast(x, y)` grey levels (the blur is a lower contrast of the pattern).
    fn pattern(w: usize, h: usize, contrast: &dyn Fn(usize, usize) -> f64) -> Gray {
        let mut data = vec![0u8; w * h];
        for y in 0..h {
            for x in 0..w {
                let sign = if (x + y) % 2 == 0 { 1.0 } else { -1.0 };
                data[y * w + x] = (128.0 + sign * contrast(x, y) / 2.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
        }
        Gray::new(w, h, data).unwrap()
    }

    #[test]
    fn an_even_picture_has_the_same_sharpness_everywhere() {
        let img = pattern(64, 64, &|_, _| 100.0);
        let m = sharp_map(&img, &SharpSettings::default()).unwrap();
        assert!(m.passes());
        assert_eq!((m.columns, m.rows, m.cells.len()), (8, 8, 64));
        assert_eq!((m.weak, m.empty), (0, 0));
        assert!(m.uniformity > 0.9, "{}", m.uniformity);
        assert!(m.tilt_x.abs() < 0.2 && m.tilt_y.abs() < 0.2);
        assert!(m.cells.iter().all(|c| c.state == State::Ok));
    }

    #[test]
    fn a_blurred_corner_is_a_weak_cell_and_a_tilt() {
        // the contrast of the pattern fades towards the right
        let img = pattern(64, 64, &|x, _| 120.0 - x as f64 * 1.6);
        let m = sharp_map(&img, &SharpSettings::default()).unwrap();
        assert!(!m.passes());
        assert_eq!(m.reasons, [REASON_UNEVEN]);
        assert!(m.weak > 0);
        assert!(m.tilt_x < -0.5, "{}", m.tilt_x);
        assert!(m.tilt_y.abs() < 0.1, "{}", m.tilt_y);
        // the weak cells are at the right
        assert!(m
            .cells
            .iter()
            .filter(|c| c.state == State::Weak)
            .all(|c| c.col >= 4));
        assert!(m.uniformity < 0.5);
        // a looser share lets the picture pass
        let loose = SharpSettings {
            min_share: 0.01,
            ..SharpSettings::default()
        };
        assert!(sharp_map(&img, &loose).unwrap().passes());
    }

    #[test]
    fn a_cell_with_nothing_in_it_is_empty_and_not_judged() {
        // the left half is a flat grey
        let img = pattern(64, 64, &|x, _| if x < 32 { 0.0 } else { 100.0 });
        let m = sharp_map(&img, &SharpSettings::default()).unwrap();
        assert_eq!(m.empty, 32);
        assert_eq!(m.weak, 0);
        assert!(m.passes());
        assert!(m
            .cells
            .iter()
            .filter(|c| c.state == State::Empty)
            .all(|c| c.col < 4));
        let flat = pattern(64, 64, &|_, _| 0.0);
        assert!(sharp_map(&flat, &SharpSettings::default())
            .unwrap_err()
            .starts_with("flat"));
    }

    #[test]
    fn the_cells_cover_the_picture_without_a_gap_whatever_its_size() {
        for (w, h) in [(24, 24), (25, 31), (70, 40), (131, 67)] {
            let img = pattern(w, h, &|_, _| 90.0);
            let s = SharpSettings {
                cells: 8,
                ..SharpSettings::default()
            };
            if w / 8 < MIN_CELL_PX || h / 8 < MIN_CELL_PX {
                assert!(sharp_map(&img, &s).is_err());
                continue;
            }
            let m = sharp_map(&img, &s).unwrap();
            let mut covered = vec![0u8; w * h];
            for c in &m.cells {
                assert!(c.x1 > c.x0 && c.y1 > c.y0);
                for y in c.y0..c.y1 {
                    for x in c.x0..c.x1 {
                        covered[y * w + x] += 1;
                    }
                }
            }
            assert!(covered.iter().all(|&n| n == 1), "{w}x{h}");
        }
    }

    #[test]
    fn the_heat_picture_is_the_size_of_the_picture_and_cold_where_it_is_blurred() {
        let img = pattern(64, 64, &|x, _| 120.0 - x as f64 * 1.6);
        let m = sharp_map(&img, &SharpSettings::default()).unwrap();
        let hot = heat(&m, 64, 64).unwrap();
        assert_eq!((hot.w, hot.h), (64, 64));
        assert!(
            hot.data[4] > hot.data[60],
            "{} {}",
            hot.data[4],
            hot.data[60]
        );
        assert!(hot.data.contains(&255));
        // an empty cell is black
        let half = pattern(64, 64, &|x, _| if x < 32 { 0.0 } else { 100.0 });
        let m = sharp_map(&half, &SharpSettings::default()).unwrap();
        assert_eq!(heat(&m, 64, 64).unwrap().data[0], 0);
    }

    #[test]
    fn what_cannot_be_mapped_is_refused() {
        let img = pattern(64, 64, &|_, _| 100.0);
        let ok = SharpSettings::default();
        assert!(sharp_map(&Gray::blank(10, 10, 5).unwrap(), &ok)
            .unwrap_err()
            .starts_with("picture_small"));
        for bad in [
            SharpSettings { cells: 1, ..ok },
            SharpSettings { cells: 65, ..ok },
            SharpSettings {
                min_share: 0.0,
                ..ok
            },
            SharpSettings {
                min_share: 1.5,
                ..ok
            },
            SharpSettings {
                min_share: f64::NAN,
                ..ok
            },
            SharpSettings {
                min_contrast: -1.0,
                ..ok
            },
            SharpSettings {
                min_contrast: 300.0,
                ..ok
            },
        ] {
            assert!(sharp_map(&img, &bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_files_have_a_row_for_every_cell() {
        let img = pattern(64, 64, &|x, _| 120.0 - x as f64 * 1.6);
        let m = sharp_map(&img, &SharpSettings::default()).unwrap();
        let csv = to_csv(&m);
        let mut lines = csv.lines();
        assert_eq!(lines.next().unwrap(), CSV_HEADER);
        assert_eq!(lines.clone().count(), 64);
        let first: Vec<&str> = lines.next().unwrap().split(',').collect();
        assert_eq!(first.len(), CSV_HEADER.split(',').count());
        assert_eq!(&first[..6], ["1", "1", "0", "0", "8", "8"]);
        assert!(csv.contains(",weak\n") && csv.contains(",ok\n"));
        let json = to_json(&m);
        assert!(json.contains("\"format\": \"eh3d-vision-sharpmap\""));
        assert!(json.contains("\"verdict\": \"fail\""));
        assert!(json.contains("\"reasons\": [\"uneven\"]"));
        assert_eq!(json.matches("\"state\"").count(), 64);
    }
}
