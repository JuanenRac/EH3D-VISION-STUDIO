//! A grid of marks (a printed target with the marks in rows and columns) tells how much the lens bends the lines: the marks of a straight grid lie on a straight lattice, so the radial
//! factor `k1` that makes the marks most like a lattice is the distortion of the lens.

use crate::scale::undistort;

/// What the grid says about the lens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Distortion {
    /// The radial factor to give to `scale::undistort`.
    pub k1: f64,
    /// How far the marks are from the nearest lattice before the correction, in pixels (root mean square).
    pub rms_before: f64,
    /// ... and after it.
    pub rms_after: f64,
}

/// The largest `|k1|` that is searched, and the step of the first sweep.
const K1_LIMIT: f64 = 0.5;
const K1_STEP: f64 = 0.0025;

fn by_x(a: &(f64, f64), b: &(f64, f64)) -> std::cmp::Ordering {
    a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)
}

fn by_y(a: &(f64, f64), b: &(f64, f64)) -> std::cmp::Ordering {
    a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
}

/// The marks in the order of the grid, row after row (the rows from the top of the picture down and each row from the left to the right). The marks must be exactly `columns` x `rows`
/// and form a grid that is not turned by more than a fraction of the distance between rows.
pub fn order(
    points: &[(f64, f64)],
    columns: usize,
    rows: usize,
) -> Result<Vec<(f64, f64)>, String> {
    if columns < 2 || rows < 2 {
        return Err("a grid has at least 2 columns and 2 rows".into());
    }
    if points.len() != columns * rows {
        return Err(format!(
            "a grid of {columns} x {rows} needs {} marks and {} were given",
            columns * rows,
            points.len()
        ));
    }
    if points.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
        return Err("a point is not a number".into());
    }
    let mut sorted = points.to_vec();
    sorted.sort_by(by_y);
    let mut out = Vec::with_capacity(sorted.len());
    for row in sorted.chunks(columns) {
        let mut row = row.to_vec();
        row.sort_by(by_x);
        out.extend(row);
    }
    Ok(out)
}

/// The lattice that is closest to the points (given in the order of `order`), by least squares: the same number of points, in the places a perfect grid would have them.
pub fn lattice(
    ordered: &[(f64, f64)],
    columns: usize,
    rows: usize,
) -> Result<Vec<(f64, f64)>, String> {
    if columns < 2 || rows < 2 || ordered.len() != columns * rows {
        return Err(format!(
            "a grid of {columns} x {rows} needs {} marks and {} were given",
            columns * rows,
            ordered.len()
        ));
    }
    let n = ordered.len() as f64;
    // the column and the row of each mark counted from the middle of the grid, so that the three terms of the fit do not depend on each other
    let index = |k: usize| {
        let (c, r) = (k % columns, k / columns);
        (
            c as f64 - (columns as f64 - 1.0) / 2.0,
            r as f64 - (rows as f64 - 1.0) / 2.0,
        )
    };
    let (mut cc, mut rr) = (0.0, 0.0);
    let (mut sx, mut sy) = ((0.0, 0.0, 0.0), (0.0, 0.0, 0.0));
    for (k, p) in ordered.iter().enumerate() {
        let (c, r) = index(k);
        cc += c * c;
        rr += r * r;
        sx = (sx.0 + p.0, sx.1 + c * p.0, sx.2 + r * p.0);
        sy = (sy.0 + p.1, sy.1 + c * p.1, sy.2 + r * p.1);
    }
    let (ox, oy) = (sx.0 / n, sy.0 / n);
    let (ux, uy) = (sx.1 / cc, sy.1 / cc);
    let (vx, vy) = (sx.2 / rr, sy.2 / rr);
    Ok((0..ordered.len())
        .map(|k| {
            let (c, r) = index(k);
            (ox + c * ux + r * vx, oy + c * uy + r * vy)
        })
        .collect())
}

/// How far the points are from the lattice that fits them, in the units of the points (root mean square).
fn residual(ordered: &[(f64, f64)], columns: usize, rows: usize) -> Result<f64, String> {
    let ideal = lattice(ordered, columns, rows)?;
    let sum: f64 = ordered
        .iter()
        .zip(&ideal)
        .map(|(p, q)| (p.0 - q.0).powi(2) + (p.1 - q.1).powi(2))
        .sum();
    Ok((sum / ordered.len() as f64).sqrt())
}

/// The grid and the picture a `k1` is tried on.
struct Trial<'a> {
    ordered: &'a [(f64, f64)],
    columns: usize,
    rows: usize,
    centre: (f64, f64),
    norm: f64,
}

impl Trial<'_> {
    /// The residual of the lattice once the points are corrected with `k1`.
    fn residual(&self, k1: f64) -> Result<f64, String> {
        let corrected: Vec<(f64, f64)> = self
            .ordered
            .iter()
            .map(|p| undistort(*p, self.centre, k1, self.norm))
            .collect();
        residual(&corrected, self.columns, self.rows)
    }
}

/// The `k1` that makes the marks of a grid most like a lattice. `points` are the marks as the camera saw them (any order), `centre` is the centre of the picture and `norm` the distance from
/// it to a corner, as for `scale::undistort`.
pub fn fit_distortion(
    points: &[(f64, f64)],
    columns: usize,
    rows: usize,
    centre: (f64, f64),
    norm: f64,
) -> Result<Distortion, String> {
    if !(norm.is_finite() && norm > 0.0) || !centre.0.is_finite() || !centre.1.is_finite() {
        return Err(
            "the centre and the size of the picture must be numbers, the size more than 0".into(),
        );
    }
    let ordered = order(points, columns, rows)?;
    let rms_before = residual(&ordered, columns, rows)?;
    let trial = Trial {
        ordered: &ordered,
        columns,
        rows,
        centre,
        norm,
    };
    let steps = (K1_LIMIT / K1_STEP).round() as i32;
    let mut best = (f64::INFINITY, 0.0);
    for s in -steps..=steps {
        let k = f64::from(s) * K1_STEP;
        let r = trial.residual(k)?;
        if r < best.0 {
            best = (r, k);
        }
    }
    // the minimum is between the two neighbours of the best step: narrow it down
    let (mut lo, mut hi) = (best.1 - K1_STEP, best.1 + K1_STEP);
    for _ in 0..60 {
        let a = lo + (hi - lo) / 3.0;
        let b = hi - (hi - lo) / 3.0;
        if trial.residual(a)? < trial.residual(b)? {
            hi = b;
        } else {
            lo = a;
        }
    }
    let k1 = (lo + hi) / 2.0;
    Ok(Distortion {
        k1,
        rms_before,
        rms_after: trial.residual(k1)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CENTRE: (f64, f64) = (320.0, 240.0);

    fn norm() -> f64 {
        CENTRE.0.hypot(CENTRE.1)
    }

    /// A grid of marks, `pitch` pixels apart and centred on the picture.
    fn grid(columns: usize, rows: usize, pitch: f64) -> Vec<(f64, f64)> {
        let mut out = Vec::new();
        for r in 0..rows {
            for c in 0..columns {
                out.push((
                    CENTRE.0 + (c as f64 - (columns as f64 - 1.0) / 2.0) * pitch,
                    CENTRE.1 + (r as f64 - (rows as f64 - 1.0) / 2.0) * pitch,
                ));
            }
        }
        out
    }

    /// Where a lens with the factor `k1` puts a point: the point that `undistort` takes back to `ideal`.
    fn bend(ideal: (f64, f64), k1: f64) -> (f64, f64) {
        let mut m = ideal;
        for _ in 0..60 {
            let r2 = ((m.0 - CENTRE.0).powi(2) + (m.1 - CENTRE.1).powi(2)) / (norm() * norm());
            m = (
                CENTRE.0 + (ideal.0 - CENTRE.0) / (1.0 + k1 * r2),
                CENTRE.1 + (ideal.1 - CENTRE.1) / (1.0 + k1 * r2),
            );
        }
        m
    }

    #[test]
    fn marks_are_put_in_the_order_of_the_grid() {
        let g = grid(3, 2, 50.0);
        let shuffled = [g[4], g[0], g[5], g[2], g[3], g[1]];
        assert_eq!(order(&shuffled, 3, 2).unwrap(), g);
    }

    #[test]
    fn a_wrong_number_of_marks_or_a_grid_too_small_is_an_error() {
        let g = grid(3, 2, 50.0);
        assert!(order(&g[..5], 3, 2).unwrap_err().contains("needs 6 marks"));
        assert!(order(&g, 6, 1).is_err());
        assert!(order(&[(f64::NAN, 0.0); 6], 3, 2).is_err());
        assert!(lattice(&g[..5], 3, 2).is_err());
    }

    #[test]
    fn a_perfect_grid_is_its_own_lattice() {
        let g = grid(5, 4, 60.0);
        let l = lattice(&g, 5, 4).unwrap();
        for (a, b) in g.iter().zip(&l) {
            assert!((a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
        }
    }

    #[test]
    fn a_straight_grid_has_no_distortion() {
        let g = grid(5, 4, 100.0);
        let d = fit_distortion(&g, 5, 4, CENTRE, norm()).unwrap();
        assert!(d.k1.abs() < 1e-6, "k1 {}", d.k1);
        assert!(d.rms_before < 1e-6);
    }

    #[test]
    fn the_distortion_of_a_lens_is_found_from_the_grid() {
        for k1 in [-0.1, -0.05, 0.05, 0.1] {
            let bent: Vec<(f64, f64)> =
                grid(5, 4, 100.0).into_iter().map(|p| bend(p, k1)).collect();
            let d = fit_distortion(&bent, 5, 4, CENTRE, norm()).unwrap();
            assert!((d.k1 - k1).abs() < 1e-3, "k1 {k1} found as {}", d.k1);
            assert!(d.rms_after < d.rms_before / 10.0);
        }
    }

    #[test]
    fn the_norm_must_be_more_than_zero() {
        let g = grid(3, 3, 50.0);
        assert!(fit_distortion(&g, 3, 3, CENTRE, 0.0).is_err());
        assert!(fit_distortion(&g, 3, 3, (f64::NAN, 0.0), 10.0).is_err());
    }
}
