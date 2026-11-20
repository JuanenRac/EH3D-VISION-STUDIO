//! The radial distortion of a lens with two terms, from a grid of marks. The marks of a straight grid lie on a straight lattice, so the factors `k1` and `k2` of
//! `p' = c + (p - c) * (1 + k1 * (r / norm)^2 + k2 * (r / norm)^4)` that make the marks most like a lattice are the distortion of the lens. The fit alternates between the
//! lattice that fits the corrected marks and the two factors that best bring the marks onto that lattice (a linear least squares of two unknowns), and it is refused when the
//! marks still lie farther from the lattice than the tolerance, when a factor is out of the range that a real lens has, and when the marks cannot tell the two factors apart.

use crate::grid::{lattice, order};

/// The largest `|k1|` and `|k2|` that are accepted.
pub const K_LIMIT: f64 = 0.5;
/// The code of a fit that leaves the marks too far from the lattice.
pub const RESIDUAL_TOO_LARGE: &str = "lens_residual_too_large";
/// The code of a factor outside the range of a real lens.
pub const OUT_OF_RANGE: &str = "lens_out_of_range";

/// What the grid says about the lens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lens {
    pub k1: f64,
    pub k2: f64,
    /// How far the marks are from the nearest lattice before the correction, in pixels (root mean square).
    pub rms_before: f64,
    /// ... and after it.
    pub rms_after: f64,
    /// How many marks were used.
    pub marks: usize,
}

/// The radial correction with two factors; `norm` is the distance from the centre to a corner. With `k2` at 0 it is `scale::undistort`.
pub fn undistort2(p: (f64, f64), centre: (f64, f64), k1: f64, k2: f64, norm: f64) -> (f64, f64) {
    let (dx, dy) = (p.0 - centre.0, p.1 - centre.1);
    let r2 = (dx * dx + dy * dy) / (norm * norm);
    let factor = 1.0 + k1 * r2 + k2 * r2 * r2;
    (centre.0 + dx * factor, centre.1 + dy * factor)
}

/// Where a lens with the factors puts a point that should be at `ideal`: the point that `undistort2` takes back to `ideal`, found by iterating. It is the inverse of `undistort2` for
/// the factors of a real lens (below `K_LIMIT`); a point that does not settle, or a factor that would flip the picture over, is left where it is.
pub fn distort2(ideal: (f64, f64), centre: (f64, f64), k1: f64, k2: f64, norm: f64) -> (f64, f64) {
    let mut m = ideal;
    for _ in 0..100 {
        let (dx, dy) = (m.0 - centre.0, m.1 - centre.1);
        let r2 = (dx * dx + dy * dy) / (norm * norm);
        let factor = 1.0 + k1 * r2 + k2 * r2 * r2;
        if !factor.is_finite() || factor < 0.05 {
            return ideal;
        }
        let next = (
            centre.0 + (ideal.0 - centre.0) / factor,
            centre.1 + (ideal.1 - centre.1) / factor,
        );
        let moved = (next.0 - m.0).abs().max((next.1 - m.1).abs());
        m = next;
        if moved < 1e-9 {
            break;
        }
    }
    m
}

/// The places in the picture where the lens with the factors puts the lattice of a grid of `columns` x `rows` marks: the lattice that fits the marks once they are corrected, bent back
/// through the lens. The marks of a picture that was fitted lie on these places; one that does not is a mark that is wrong. The places come in the order of the grid, row after row from
/// the top of the picture, each row from the left.
pub fn predicted_grid(
    points: &[(f64, f64)],
    columns: usize,
    rows: usize,
    centre: (f64, f64),
    norm: f64,
    k1: f64,
    k2: f64,
) -> Result<Vec<(f64, f64)>, String> {
    if !(norm.is_finite() && norm > 0.0) {
        return Err("the size of the picture must be a number above 0".into());
    }
    let ordered = order(points, columns, rows)?;
    let corrected: Vec<(f64, f64)> = ordered
        .iter()
        .map(|p| undistort2(*p, centre, k1, k2, norm))
        .collect();
    let ideal = lattice(&corrected, columns, rows)?;
    Ok(ideal
        .into_iter()
        .map(|p| distort2(p, centre, k1, k2, norm))
        .collect())
}

/// The lattice that fits the marks once they are corrected with the factors, where the corrected marks are drawn (the window shows the corrected marks when the correction is on): the
/// same lattice as `predicted_grid` before it is bent back through the lens.
pub fn corrected_grid(
    points: &[(f64, f64)],
    columns: usize,
    rows: usize,
    centre: (f64, f64),
    norm: f64,
    k1: f64,
    k2: f64,
) -> Result<Vec<(f64, f64)>, String> {
    if !(norm.is_finite() && norm > 0.0) {
        return Err("the size of the picture must be a number above 0".into());
    }
    let ordered = order(points, columns, rows)?;
    let corrected: Vec<(f64, f64)> = ordered
        .iter()
        .map(|p| undistort2(*p, centre, k1, k2, norm))
        .collect();
    lattice(&corrected, columns, rows)
}

/// How far the points are from their lattice, in pixels (root mean square), and the lattice.
fn residual(
    corrected: &[(f64, f64)],
    columns: usize,
    rows: usize,
) -> Result<(f64, Vec<(f64, f64)>), String> {
    let ideal = lattice(corrected, columns, rows)?;
    let sum: f64 = corrected
        .iter()
        .zip(&ideal)
        .map(|(p, q)| (p.0 - q.0).powi(2) + (p.1 - q.1).powi(2))
        .sum();
    Ok(((sum / corrected.len() as f64).sqrt(), ideal))
}

/// The two factors of the lens from a grid of `columns` x `rows` marks (any order). `centre` is the centre of the picture and `norm` the distance from it to a corner. The fit is
/// refused with the code `lens_residual_too_large` when the marks are farther than `max_rms_px` from the lattice after the correction (a wrong mark, or a target that is not flat),
/// with `lens_out_of_range` when a factor is above `K_LIMIT`, and when fewer than 6 marks are given.
pub fn fit_lens(
    points: &[(f64, f64)],
    columns: usize,
    rows: usize,
    centre: (f64, f64),
    norm: f64,
    max_rms_px: f64,
) -> Result<Lens, String> {
    if !(norm.is_finite() && norm > 0.0) || !centre.0.is_finite() || !centre.1.is_finite() {
        return Err(
            "the centre and the size of the picture must be numbers, the size more than 0".into(),
        );
    }
    if !(max_rms_px.is_finite() && max_rms_px > 0.0) {
        return Err("the tolerance of the residual must be more than 0 pixels".into());
    }
    let ordered = order(points, columns, rows)?;
    if ordered.len() < 6 {
        return Err(format!(
            "{} marks: the two factors of the lens need at least 6",
            ordered.len()
        ));
    }
    let (rms_before, _) = residual(&ordered, columns, rows)?;
    // the powers of the distance of each mark from the centre, which do not change while the factors are searched
    let terms: Vec<(f64, f64, f64, f64)> = ordered
        .iter()
        .map(|p| {
            let (ux, uy) = (p.0 - centre.0, p.1 - centre.1);
            let r2 = (ux * ux + uy * uy) / (norm * norm);
            (ux, uy, r2, r2 * r2)
        })
        .collect();
    let (mut k1, mut k2) = (0.0f64, 0.0f64);
    for _ in 0..200 {
        let corrected: Vec<(f64, f64)> = ordered
            .iter()
            .map(|p| undistort2(*p, centre, k1, k2, norm))
            .collect();
        let (_, ideal) = residual(&corrected, columns, rows)?;
        // the normal equations of: ideal - centre - u = u * (k1 * r2 + k2 * r4)
        let (mut a11, mut a12, mut a22, mut b1, mut b2) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for ((ux, uy, r2, r4), q) in terms.iter().zip(&ideal) {
            let (ex, ey) = (q.0 - centre.0 - ux, q.1 - centre.1 - uy);
            let (g, h) = (ux * ux + uy * uy, ux * ex + uy * ey);
            a11 += g * r2 * r2;
            a12 += g * r2 * r4;
            a22 += g * r4 * r4;
            b1 += h * r2;
            b2 += h * r4;
        }
        let det = a11 * a22 - a12 * a12;
        if det.abs() < 1e-18 * (a11 * a22).abs().max(1e-300) || !det.is_finite() {
            return Err("the marks lie too near the same distance from the centre: the two factors of the lens cannot be told apart".into());
        }
        let (n1, n2) = ((b1 * a22 - b2 * a12) / det, (a11 * b2 - a12 * b1) / det);
        let step = (n1 - k1).abs().max((n2 - k2).abs());
        (k1, k2) = (n1, n2);
        if !(k1.is_finite() && k2.is_finite())
            || k1.abs() > 4.0 * K_LIMIT
            || k2.abs() > 4.0 * K_LIMIT
        {
            return Err(format!(
                "{OUT_OF_RANGE}: k1 and k2 are far above {K_LIMIT} after the fit; the marks are not a grid seen through a lens"
            ));
        }
        if step < 1e-10 {
            break;
        }
    }
    let corrected: Vec<(f64, f64)> = ordered
        .iter()
        .map(|p| undistort2(*p, centre, k1, k2, norm))
        .collect();
    let (rms_after, _) = residual(&corrected, columns, rows)?;
    if k1.abs() > K_LIMIT || k2.abs() > K_LIMIT {
        return Err(format!(
            "{OUT_OF_RANGE}: k1 {k1:.4} and k2 {k2:.4}, a real lens has both between -{K_LIMIT} and {K_LIMIT}"
        ));
    }
    if rms_after > max_rms_px {
        return Err(format!(
            "{RESIDUAL_TOO_LARGE}: the marks are still {rms_after:.3} pixels from the lattice after the correction, more than the {max_rms_px:.3} that is allowed (a wrong mark, or a target that is not flat)"
        ));
    }
    Ok(Lens {
        k1,
        k2,
        rms_before,
        rms_after,
        marks: ordered.len(),
    })
}

/// The name of the format and its version, in the JSON file of the residuals.
pub const RESIDUAL_FORMAT: &str = "eh3d-vision-lens-residuals";
pub const RESIDUAL_FORMAT_VERSION: u32 = 1;
pub const RESIDUAL_CSV_HEADER: &str =
    "column,row,x,y,radius,before_dx,before_dy,after_dx,after_dy,after_distance";

/// How far one mark lies from its place in the lattice, before the correction of the lens and after it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkResidual {
    /// The place in the grid, counted from 0: the column from the left and the row from the top of the picture.
    pub column: usize,
    pub row: usize,
    /// Where the mark was found, in pixels.
    pub x: f64,
    pub y: f64,
    /// The distance of the mark from the centre of the picture as a share of the distance to a corner (0 at the centre, 1 at a corner).
    pub radius: f64,
    /// The mark minus its place in the lattice, in pixels: before the correction (the marks as they were found) and after it (the corrected marks and the lattice that fits them).
    pub before: (f64, f64),
    pub after: (f64, f64),
}

impl MarkResidual {
    /// The distance after the correction, in pixels.
    pub fn after_distance(&self) -> f64 {
        self.after.0.hypot(self.after.1)
    }

    fn before_distance(&self) -> f64 {
        self.before.0.hypot(self.before.1)
    }
}

/// The residuals of the marks in one band of distance from the centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    /// The band covers the radius from `from` up to but not including `to` (the last band includes everything beyond).
    pub from: f64,
    pub to: f64,
    pub marks: usize,
    /// The root mean square of the distances, before and after the correction, in pixels; 0 when the band has no mark.
    pub rms_before: f64,
    pub rms_after: f64,
}

/// What is left over once the lens is corrected, mark by mark: a lens that explains the marks leaves small residuals with no pattern; residuals that grow toward the corners say that two
/// radial terms are not enough for this lens (or that the target is not flat).
#[derive(Debug, Clone, PartialEq)]
pub struct ResidualReport {
    pub marks: Vec<MarkResidual>,
    pub rms_before: f64,
    pub rms_after: f64,
    /// The mark that is farthest from its place after the correction (the first of them when several are as far) and how far.
    pub worst: usize,
    pub max_after: f64,
    /// The marks in three bands of distance from the centre.
    pub bands: [Band; 3],
    /// The rms of the outer band over the rms of the inner one after the correction, when both have marks and the inner one is not 0: well above 1 is a radial pattern left over.
    pub radial_trend: Option<f64>,
}

fn rms(v: impl Iterator<Item = f64>) -> f64 {
    let (n, sum) = v.fold((0usize, 0.0), |(n, s), d| (n + 1, s + d * d));
    if n == 0 {
        0.0
    } else {
        (sum / n as f64).sqrt()
    }
}

/// The residuals of a grid of `columns` x `rows` marks (any order) for a lens with the factors `k1` and `k2`; `centre` and `norm` are as in `fit_lens`.
pub fn residual_report(
    points: &[(f64, f64)],
    columns: usize,
    rows: usize,
    centre: (f64, f64),
    norm: f64,
    k1: f64,
    k2: f64,
) -> Result<ResidualReport, String> {
    if !(norm.is_finite() && norm > 0.0) || !centre.0.is_finite() || !centre.1.is_finite() {
        return Err(
            "the centre and the size of the picture must be numbers, the size more than 0".into(),
        );
    }
    let ordered = order(points, columns, rows)?;
    let (_, ideal_before) = residual(&ordered, columns, rows)?;
    let corrected: Vec<(f64, f64)> = ordered
        .iter()
        .map(|p| undistort2(*p, centre, k1, k2, norm))
        .collect();
    let (_, ideal_after) = residual(&corrected, columns, rows)?;
    let marks: Vec<MarkResidual> = ordered
        .iter()
        .enumerate()
        .map(|(i, p)| MarkResidual {
            column: i % columns,
            row: i / columns,
            x: p.0,
            y: p.1,
            radius: (p.0 - centre.0).hypot(p.1 - centre.1) / norm,
            before: (p.0 - ideal_before[i].0, p.1 - ideal_before[i].1),
            after: (
                corrected[i].0 - ideal_after[i].0,
                corrected[i].1 - ideal_after[i].1,
            ),
        })
        .collect();
    let (mut worst, mut max_after) = (0usize, 0.0f64);
    for (i, m) in marks.iter().enumerate() {
        if m.after_distance() > max_after {
            max_after = m.after_distance();
            worst = i;
        }
    }
    let edges = [0.0, 1.0 / 3.0, 2.0 / 3.0, f64::INFINITY];
    let bands = [0, 1, 2].map(|b| {
        let inside = |m: &&MarkResidual| m.radius >= edges[b] && m.radius < edges[b + 1];
        Band {
            from: edges[b],
            to: if b == 2 { 1.0 } else { edges[b + 1] },
            marks: marks.iter().filter(inside).count(),
            rms_before: rms(marks
                .iter()
                .filter(inside)
                .map(MarkResidual::before_distance)),
            rms_after: rms(marks
                .iter()
                .filter(inside)
                .map(MarkResidual::after_distance)),
        }
    });
    let radial_trend = (bands[0].marks > 0 && bands[2].marks > 0 && bands[0].rms_after > 0.0)
        .then(|| bands[2].rms_after / bands[0].rms_after);
    Ok(ResidualReport {
        rms_before: rms(marks.iter().map(MarkResidual::before_distance)),
        rms_after: rms(marks.iter().map(MarkResidual::after_distance)),
        marks,
        worst,
        max_after,
        bands,
        radial_trend,
    })
}

fn fixed(v: f64, decimals: usize) -> String {
    crate::measure::fixed(v, decimals)
}

/// The residuals as CSV: the line `RESIDUAL_CSV_HEADER` and one line per mark, row after row, with a dot for the decimals.
pub fn residuals_to_csv(r: &ResidualReport) -> String {
    let mut out = format!("{RESIDUAL_CSV_HEADER}\n");
    for m in &r.marks {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            m.column,
            m.row,
            fixed(m.x, 3),
            fixed(m.y, 3),
            fixed(m.radius, 4),
            fixed(m.before.0, 4),
            fixed(m.before.1, 4),
            fixed(m.after.0, 4),
            fixed(m.after.1, 4),
            fixed(m.after_distance(), 4)
        ));
    }
    out
}

/// The residuals as JSON: an object with `format`, `version`, `unit` ("px"), the rms before and after, the `worst` mark (its column and row) and how far it is, the three `bands`, the
/// `radial_trend` (or `null`) and the list `marks`.
pub fn residuals_to_json(r: &ResidualReport) -> String {
    let marks: Vec<String> = r
        .marks
        .iter()
        .map(|m| {
            format!(
                "    {{\"column\": {}, \"row\": {}, \"x\": {}, \"y\": {}, \"radius\": {}, \"before_dx\": {}, \"before_dy\": {}, \"after_dx\": {}, \"after_dy\": {}, \"after_distance\": {}}}",
                m.column,
                m.row,
                fixed(m.x, 3),
                fixed(m.y, 3),
                fixed(m.radius, 4),
                fixed(m.before.0, 4),
                fixed(m.before.1, 4),
                fixed(m.after.0, 4),
                fixed(m.after.1, 4),
                fixed(m.after_distance(), 4)
            )
        })
        .collect();
    let bands: Vec<String> = r
        .bands
        .iter()
        .map(|b| {
            format!(
                "    {{\"from\": {}, \"to\": {}, \"marks\": {}, \"rms_before\": {}, \"rms_after\": {}}}",
                fixed(b.from, 4),
                fixed(b.to, 4),
                b.marks,
                fixed(b.rms_before, 4),
                fixed(b.rms_after, 4)
            )
        })
        .collect();
    let worst = r.marks.get(r.worst).map_or("null".to_string(), |m| {
        format!("{{\"column\": {}, \"row\": {}}}", m.column, m.row)
    });
    let trend = r.radial_trend.map_or("null".to_string(), |t| fixed(t, 3));
    format!(
        "{{\n  \"format\": \"{RESIDUAL_FORMAT}\",\n  \"version\": {RESIDUAL_FORMAT_VERSION},\n  \"unit\": \"px\",\n  \"rms_before\": {},\n  \"rms_after\": {},\n  \"worst\": {worst},\n  \"max_after\": {},\n  \"radial_trend\": {trend},\n  \"bands\": [\n{}\n  ],\n  \"marks\": [\n{}\n  ]\n}}\n",
        fixed(r.rms_before, 4),
        fixed(r.rms_after, 4),
        fixed(r.max_after, 4),
        bands.join(",\n"),
        marks.join(",\n")
    )
}

/// A few lines of text for the command line.
pub fn residuals_summary(r: &ResidualReport) -> String {
    let worst = r.marks.get(r.worst).map_or(String::new(), |m| {
        format!(
            "; the farthest is the mark of column {} and row {}, {:.3} px from its place",
            m.column + 1,
            m.row + 1,
            r.max_after
        )
    });
    let trend = r.radial_trend.map_or(String::new(), |t| {
        format!(
            "; the outer third is {t:.2} times the inner third after the correction{}",
            if t > 2.0 {
                " (a pattern that grows outwards is left: two terms may not be enough)"
            } else {
                ""
            }
        )
    });
    format!(
        "{} marks, {:.3} px from their lattice before the correction and {:.3} px after it{worst}{trend}",
        r.marks.len(),
        r.rms_before,
        r.rms_after
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::undistort;

    const CENTRE: (f64, f64) = (320.0, 240.0);

    fn norm() -> f64 {
        CENTRE.0.hypot(CENTRE.1)
    }

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

    /// Where a lens with the factors puts a point: the point that `undistort2` takes back to `ideal`.
    fn bend(ideal: (f64, f64), k1: f64, k2: f64) -> (f64, f64) {
        let mut m = ideal;
        for _ in 0..200 {
            let r2 = ((m.0 - CENTRE.0).powi(2) + (m.1 - CENTRE.1).powi(2)) / (norm() * norm());
            let f = 1.0 + k1 * r2 + k2 * r2 * r2;
            m = (
                CENTRE.0 + (ideal.0 - CENTRE.0) / f,
                CENTRE.1 + (ideal.1 - CENTRE.1) / f,
            );
        }
        m
    }

    #[test]
    fn bending_a_point_and_correcting_it_are_the_inverse_of_each_other() {
        for (k1, k2) in [(-0.1, 0.03), (0.08, -0.02), (0.0, 0.0), (0.2, 0.1)] {
            for p in [
                (500.0, 100.0),
                (10.0, 470.0),
                (320.0, 240.0),
                (330.0, 250.0),
            ] {
                let bent = distort2(p, CENTRE, k1, k2, norm());
                let back = undistort2(bent, CENTRE, k1, k2, norm());
                assert!(
                    (back.0 - p.0).abs() < 1e-6 && (back.1 - p.1).abs() < 1e-6,
                    "{p:?} {k1} {k2}"
                );
            }
        }
        // a factor that would flip the picture over leaves the point where it is
        assert_eq!(
            distort2((600.0, 20.0), CENTRE, -3.0, 0.0, norm()),
            (600.0, 20.0)
        );
    }

    #[test]
    fn the_lattice_bent_through_the_fitted_lens_lies_on_the_marks() {
        let bent: Vec<(f64, f64)> = grid(7, 5, 70.0)
            .into_iter()
            .map(|p| bend(p, -0.07, 0.02))
            .collect();
        let l = fit_lens(&bent, 7, 5, CENTRE, norm(), 0.5).unwrap();
        let mut shuffled = bent.clone();
        shuffled.reverse();
        let places = predicted_grid(&shuffled, 7, 5, CENTRE, norm(), l.k1, l.k2).unwrap();
        assert_eq!(places.len(), 35);
        // the marks come back in the order of the grid whatever order they were given in, and each lies on its place
        let ordered = order(&bent, 7, 5).unwrap();
        for (m, p) in ordered.iter().zip(&places) {
            assert!(
                (m.0 - p.0).abs() < 0.05 && (m.1 - p.1).abs() < 0.05,
                "{m:?} {p:?}"
            );
        }
        // with the correction on, the lattice is the one of the corrected marks, which lie on it as well
        let straight = corrected_grid(&bent, 7, 5, CENTRE, norm(), l.k1, l.k2).unwrap();
        for (m, p) in ordered.iter().zip(&straight) {
            let c = undistort2(*m, CENTRE, l.k1, l.k2, norm());
            assert!(
                (c.0 - p.0).abs() < 0.05 && (c.1 - p.1).abs() < 0.05,
                "{c:?} {p:?}"
            );
        }
        assert!(corrected_grid(&bent, 7, 5, CENTRE, 0.0, 0.0, 0.0).is_err());
        // a wrong number of marks or a size of 0 is refused
        assert!(predicted_grid(&bent[..30], 7, 5, CENTRE, norm(), 0.0, 0.0).is_err());
        assert!(predicted_grid(&bent, 7, 5, CENTRE, 0.0, 0.0, 0.0).is_err());
    }

    #[test]
    fn with_no_second_factor_it_is_the_correction_of_one_factor() {
        let p = (500.0, 100.0);
        assert_eq!(
            undistort2(p, CENTRE, -0.1, 0.0, norm()),
            undistort(p, CENTRE, -0.1, norm())
        );
        assert_eq!(undistort2(CENTRE, CENTRE, -0.1, 0.2, norm()), CENTRE);
    }

    #[test]
    fn the_two_factors_of_a_lens_come_back_from_its_grid() {
        for (k1, k2) in [
            (-0.08, 0.03),
            (0.06, -0.02),
            (-0.12, 0.0),
            (0.0, 0.05),
            (0.1, 0.04),
        ] {
            let bent: Vec<(f64, f64)> = grid(9, 7, 60.0)
                .into_iter()
                .map(|p| bend(p, k1, k2))
                .collect();
            let l = fit_lens(&bent, 9, 7, CENTRE, norm(), 0.5).unwrap();
            assert!(
                (l.k1 - k1).abs() < 5e-3 && (l.k2 - k2).abs() < 5e-3,
                "k1 {k1} k2 {k2} found as {} {}",
                l.k1,
                l.k2
            );
            assert!(l.rms_after < 0.05, "{l:?}");
            assert!(l.rms_after <= l.rms_before + 1e-9, "{l:?}");
            assert_eq!(l.marks, 63);
        }
    }

    #[test]
    fn the_marks_may_come_in_any_order_and_the_fit_does_not_depend_on_it() {
        let bent: Vec<(f64, f64)> = grid(7, 5, 70.0)
            .into_iter()
            .map(|p| bend(p, -0.07, 0.02))
            .collect();
        let mut shuffled = bent.clone();
        shuffled.reverse();
        shuffled.swap(3, 17);
        let a = fit_lens(&bent, 7, 5, CENTRE, norm(), 0.5).unwrap();
        let b = fit_lens(&shuffled, 7, 5, CENTRE, norm(), 0.5).unwrap();
        assert!((a.k1 - b.k1).abs() < 1e-9 && (a.k2 - b.k2).abs() < 1e-9);
    }

    #[test]
    fn a_straight_grid_has_no_distortion() {
        let l = fit_lens(&grid(7, 5, 70.0), 7, 5, CENTRE, norm(), 0.5).unwrap();
        assert!(l.k1.abs() < 1e-6 && l.k2.abs() < 1e-6, "{l:?}");
        assert!(l.rms_before < 1e-6 && l.rms_after < 1e-6);
    }

    #[test]
    fn a_wrong_mark_leaves_the_fit_out_of_tolerance_and_is_refused() {
        let mut bent: Vec<(f64, f64)> = grid(7, 5, 70.0)
            .into_iter()
            .map(|p| bend(p, -0.06, 0.02))
            .collect();
        assert!(fit_lens(&bent, 7, 5, CENTRE, norm(), 0.5).is_ok());
        // one mark of the grid is 6 pixels from where it should be
        bent[11].0 += 6.0;
        let why = fit_lens(&bent, 7, 5, CENTRE, norm(), 0.5).unwrap_err();
        assert!(why.starts_with(RESIDUAL_TOO_LARGE), "{why}");
        // a wider tolerance accepts it
        assert!(fit_lens(&bent, 7, 5, CENTRE, norm(), 5.0).is_ok());
    }

    #[test]
    fn what_makes_no_sense_is_refused() {
        let g = grid(5, 4, 70.0);
        assert!(fit_lens(&g, 5, 4, CENTRE, 0.0, 0.5).is_err());
        assert!(fit_lens(&g, 5, 4, (f64::NAN, 0.0), norm(), 0.5).is_err());
        for tolerance in [0.0, -1.0, f64::NAN] {
            assert!(fit_lens(&g, 5, 4, CENTRE, norm(), tolerance).is_err());
        }
        assert!(fit_lens(&g[..19], 5, 4, CENTRE, norm(), 0.5).is_err());
        let few = fit_lens(&grid(2, 2, 70.0), 2, 2, CENTRE, norm(), 0.5).unwrap_err();
        assert!(few.contains("at least 6"), "{few}");
        assert!(fit_lens(&[(f64::NAN, 0.0); 20], 5, 4, CENTRE, norm(), 0.5).is_err());
    }

    #[test]
    fn marks_at_the_centre_cannot_tell_the_factors_apart() {
        // a grid of 2 x 3 marks around the centre has two distances from it: enough for two factors, and the straight grid gives factors of 0
        let g = vec![
            (250.0, 190.0),
            (390.0, 190.0),
            (250.0, 290.0),
            (390.0, 290.0),
            (250.0, 240.0),
            (390.0, 240.0),
        ];
        let r = fit_lens(&g, 2, 3, CENTRE, norm(), 0.5);
        let l = r.unwrap();
        assert!(l.k1.abs() < 1e-6 && l.k2.abs() < 1e-6, "{l:?}");
        // marks that are all at the centre have no distance from it, so the factors cannot be told apart
        let same = vec![CENTRE; 6];
        assert!(fit_lens(&same, 2, 3, CENTRE, norm(), 0.5).is_err());
    }

    #[test]
    fn a_factor_outside_the_range_of_a_real_lens_is_refused() {
        let bent: Vec<(f64, f64)> = grid(9, 7, 60.0)
            .into_iter()
            .map(|p| bend(p, -0.2, 0.0))
            .collect();
        let ok = fit_lens(&bent, 9, 7, CENTRE, norm(), 0.5).unwrap();
        assert!((ok.k1 + 0.2).abs() < 5e-3);
        // the same grid measured against a norm twice as large needs a factor of 0.8
        let why = fit_lens(&bent, 9, 7, CENTRE, norm() * 2.0, 0.5).unwrap_err();
        assert!(why.starts_with(OUT_OF_RANGE), "{why}");
    }

    fn bent_grid(k1: f64, k2: f64) -> Vec<(f64, f64)> {
        grid(7, 5, 60.0)
            .into_iter()
            .map(|p| bend(p, k1, k2))
            .collect()
    }

    #[test]
    fn the_residuals_of_the_fitted_lens_are_small_and_those_of_no_lens_are_not() {
        let marks = bent_grid(-0.12, 0.03);
        let fit = fit_lens(&marks, 7, 5, CENTRE, norm(), 0.5).unwrap();
        let r = residual_report(&marks, 7, 5, CENTRE, norm(), fit.k1, fit.k2).unwrap();
        assert_eq!(r.marks.len(), 35);
        assert!((r.rms_after - fit.rms_after).abs() < 1e-9);
        assert!((r.rms_before - fit.rms_before).abs() < 1e-9);
        assert!(
            r.rms_after < 0.05 && r.rms_before > 0.5,
            "{} {}",
            r.rms_after,
            r.rms_before
        );
        assert!(r.max_after < 0.2);
        // with no lens at all the residual is the one of the marks as they were found
        let none = residual_report(&marks, 7, 5, CENTRE, norm(), 0.0, 0.0).unwrap();
        assert!((none.rms_after - none.rms_before).abs() < 1e-9);
        // the places are counted row after row from the top left
        let first = &r.marks[0];
        assert_eq!((first.column, first.row), (0, 0));
        let last = &r.marks[34];
        assert_eq!((last.column, last.row), (6, 4));
        // the radius is 0 at the centre mark and grows outwards
        let centre_mark = &r.marks[17];
        assert!(centre_mark.radius < 0.01);
        assert!(r.marks[0].radius > 0.4);
    }

    #[test]
    fn a_wrong_mark_is_the_worst_one_and_the_order_of_the_marks_does_not_matter() {
        let mut marks = bent_grid(-0.1, 0.02);
        // move the mark of column 2, row 1 by 3 pixels
        let wrong = marks[9];
        marks[9] = (wrong.0 + 3.0, wrong.1 - 1.0);
        let r = residual_report(&marks, 7, 5, CENTRE, norm(), -0.1, 0.02).unwrap();
        let worst = &r.marks[r.worst];
        assert_eq!((worst.column, worst.row), (2, 1));
        assert!(r.max_after > 1.0);
        let mut shuffled = marks.clone();
        shuffled.reverse();
        shuffled.swap(3, 20);
        let again = residual_report(&shuffled, 7, 5, CENTRE, norm(), -0.1, 0.02).unwrap();
        assert_eq!(
            (
                again.marks[again.worst].column,
                again.marks[again.worst].row
            ),
            (2, 1)
        );
        assert!((again.rms_after - r.rms_after).abs() < 1e-9);
    }

    #[test]
    fn a_lens_that_two_terms_cannot_explain_leaves_a_pattern_that_grows_outwards() {
        // a lens with a third term: the two-term fit cannot bring the corners back
        let marks: Vec<(f64, f64)> = grid(9, 7, 55.0)
            .into_iter()
            .map(|p| {
                let mut m = p;
                for _ in 0..200 {
                    let r2 =
                        ((m.0 - CENTRE.0).powi(2) + (m.1 - CENTRE.1).powi(2)) / (norm() * norm());
                    let f = 1.0 - 0.05 * r2 + 0.35 * r2 * r2 * r2;
                    m = (
                        CENTRE.0 + (p.0 - CENTRE.0) / f,
                        CENTRE.1 + (p.1 - CENTRE.1) / f,
                    );
                }
                m
            })
            .collect();
        let fit = fit_lens(&marks, 9, 7, CENTRE, norm(), 50.0).unwrap();
        let r = residual_report(&marks, 9, 7, CENTRE, norm(), fit.k1, fit.k2).unwrap();
        let trend = r.radial_trend.expect("both bands have marks");
        assert!(trend > 1.0, "{trend}");
        assert!(residuals_summary(&r).contains("times the inner third"));
        // the bands hold every mark once
        assert_eq!(r.bands.iter().map(|b| b.marks).sum::<usize>(), 63);
    }

    #[test]
    fn the_residuals_are_written_as_csv_and_json_with_every_mark() {
        let marks = bent_grid(-0.12, 0.03);
        let r = residual_report(&marks, 7, 5, CENTRE, norm(), -0.12, 0.03).unwrap();
        let csv = residuals_to_csv(&r);
        let mut lines = csv.lines();
        assert_eq!(lines.next(), Some(RESIDUAL_CSV_HEADER));
        let rows: Vec<&str> = lines.collect();
        assert_eq!(rows.len(), 35);
        assert!(rows
            .iter()
            .all(|l| l.split(',').count() == RESIDUAL_CSV_HEADER.split(',').count()));
        assert!(rows[0].starts_with("0,0,"));
        let json = residuals_to_json(&r);
        assert!(json.contains("\"format\": \"eh3d-vision-lens-residuals\""));
        assert_eq!(json.matches("\"after_distance\"").count(), 35);
        assert_eq!(json.matches("\"rms_before\"").count(), 1 + 3);
        assert!(json.contains("\"unit\": \"px\""));
        assert!(residuals_summary(&r).starts_with("35 marks"));
    }

    #[test]
    fn residuals_are_refused_for_a_grid_that_does_not_fit_and_for_a_bad_picture_size() {
        let marks = bent_grid(-0.1, 0.0);
        assert!(residual_report(&marks, 7, 6, CENTRE, norm(), 0.0, 0.0).is_err());
        assert!(residual_report(&marks, 7, 5, CENTRE, 0.0, 0.0, 0.0).is_err());
        assert!(residual_report(&marks, 7, 5, (f64::NAN, 1.0), norm(), 0.0, 0.0).is_err());
        assert!(residual_report(&[], 7, 5, CENTRE, norm(), 0.0, 0.0).is_err());
    }
}
