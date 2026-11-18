//! From pixels to millimetres: the scale of the camera from a mark of a known size, the machine axes (y goes up, the image goes down) and the radial distortion of the lens.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale {
    pub mm_per_px: f64,
}

impl Scale {
    /// The scale from a mark that measures `diameter_px` in the picture and `known_mm` in reality.
    pub fn from_known_diameter(diameter_px: f64, known_mm: f64) -> Result<Scale, String> {
        if !(diameter_px.is_finite() && diameter_px > 0.0 && known_mm.is_finite() && known_mm > 0.0)
        {
            return Err(format!("a mark of {diameter_px} pixels and {known_mm} mm gives no scale: both must be more than 0"));
        }
        Ok(Scale {
            mm_per_px: known_mm / diameter_px,
        })
    }

    pub fn mm(&self, px: f64) -> f64 {
        px * self.mm_per_px
    }

    /// The place of a pixel in millimetres from the centre of the picture, with y going up.
    pub fn point(&self, x_px: f64, y_px: f64, centre_px: (f64, f64)) -> (f64, f64) {
        (
            (x_px - centre_px.0) * self.mm_per_px,
            (centre_px.1 - y_px) * self.mm_per_px,
        )
    }
}

/// The radial correction `p' = c + (p - c) * (1 + k1 * (r / norm)^2)` that straightens the picture of a lens that bends the lines: `norm` is the distance from the centre to a corner.
pub fn undistort(p: (f64, f64), centre: (f64, f64), k1: f64, norm: f64) -> (f64, f64) {
    let (dx, dy) = (p.0 - centre.0, p.1 - centre.1);
    let r2 = (dx * dx + dy * dy) / (norm * norm);
    (
        centre.0 + dx * (1.0 + k1 * r2),
        centre.1 + dy * (1.0 + k1 * r2),
    )
}

/// The `k1` that best brings the `measured` points (a grid of marks as the camera sees it) onto the `ideal` ones (where they are), by least squares.
pub fn fit_k1(
    measured: &[(f64, f64)],
    ideal: &[(f64, f64)],
    centre: (f64, f64),
    norm: f64,
) -> Result<f64, String> {
    if measured.len() != ideal.len() || measured.len() < 3 {
        return Err(format!(
            "{} measured and {} ideal points: they must be the same number, and at least 3",
            measured.len(),
            ideal.len()
        ));
    }
    if !(norm.is_finite() && norm > 0.0) {
        return Err("the norm must be more than 0".into());
    }
    let (mut num, mut den) = (0.0, 0.0);
    for (m, i) in measured.iter().zip(ideal) {
        let (ux, uy) = (m.0 - centre.0, m.1 - centre.1);
        let r2 = (ux * ux + uy * uy) / (norm * norm);
        let (vx, vy) = (i.0 - centre.0, i.1 - centre.1);
        num += (vx - ux) * ux * r2 + (vy - uy) * uy * r2;
        den += (ux * r2).powi(2) + (uy * r2).powi(2);
    }
    if den < 1e-12 {
        return Err("the points are all at the centre: the distortion cannot be measured".into());
    }
    Ok(num / den)
}

/// The scale worked out from the distances between neighbouring marks of a target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PitchScale {
    pub scale: Scale,
    /// How many marks were used.
    pub marks: usize,
    /// The difference between the longest and the shortest distance to the nearest neighbour, as a fraction of the median one (0.02 is two percent).
    pub spread: f64,
}

/// The most marks that are looked at (the work grows with the square of their number).
pub const MAX_PITCH_MARKS: usize = 2000;

/// The millimetres of a pixel from the marks of a target whose neighbours are `pitch_mm` apart (a grid of marks, in the places of the picture). The distance from each mark to its nearest
/// neighbour is that pitch; the median of them is taken, so one mark that is missing or extra does not move the result. When the longest and the shortest of them differ by more than
/// `tolerance` of the median the target is not seen square-on or the lens bends it, and the answer is refused instead of averaged. At least 4 marks are needed.
pub fn from_pitch(
    points: &[(f64, f64)],
    pitch_mm: f64,
    tolerance: f64,
) -> Result<PitchScale, String> {
    if !(pitch_mm.is_finite() && pitch_mm > 0.0) {
        return Err("the pitch between marks must be more than 0 millimetres".into());
    }
    if !(tolerance.is_finite() && tolerance >= 0.0) {
        return Err("the tolerance must be a number that is not negative".into());
    }
    if points.len() < 4 {
        return Err(format!(
            "{} marks: the scale from a pitch needs at least 4",
            points.len()
        ));
    }
    if points.len() > MAX_PITCH_MARKS {
        return Err(format!(
            "{} marks: the scale from a pitch looks at {MAX_PITCH_MARKS} marks at most",
            points.len()
        ));
    }
    if points.iter().any(|p| !(p.0.is_finite() && p.1.is_finite())) {
        return Err("a mark has a place that is not a number".into());
    }
    let mut nearest = Vec::with_capacity(points.len());
    for (i, a) in points.iter().enumerate() {
        let d = points
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, b)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt())
            .fold(f64::INFINITY, f64::min);
        if d <= 0.0 {
            return Err("two marks are in the same place".into());
        }
        nearest.push(d);
    }
    nearest.sort_by(|a, b| a.total_cmp(b));
    let n = nearest.len();
    let median = if n % 2 == 1 {
        nearest[n / 2]
    } else {
        (nearest[n / 2 - 1] + nearest[n / 2]) / 2.0
    };
    let spread = (nearest[n - 1] - nearest[0]) / median;
    if spread > tolerance {
        return Err(format!(
            "the distances between neighbouring marks differ by {:.1} %, more than the {:.1} % that is allowed: the target is not seen square-on, the lens bends it, or a mark is wrong",
            spread * 100.0,
            tolerance * 100.0
        ));
    }
    Ok(PitchScale {
        scale: Scale {
            mm_per_px: pitch_mm / median,
        },
        marks: n,
        spread,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lattice(columns: usize, rows: usize, pitch: f64) -> Vec<(f64, f64)> {
        (0..rows)
            .flat_map(|r| {
                (0..columns).map(move |c| (30.0 + c as f64 * pitch, 25.0 + r as f64 * pitch))
            })
            .collect()
    }

    #[test]
    fn the_scale_comes_from_the_pitch_of_a_grid_of_marks() {
        let p = from_pitch(&lattice(5, 4, 40.0), 2.0, 0.05).unwrap();
        assert!((p.scale.mm_per_px - 0.05).abs() < 1e-12, "{p:?}");
        assert_eq!((p.marks, p.spread), (20, 0.0));
        // a scale that is not the same at both ends of the picture is within the tolerance when it is small
        let mut bent = lattice(5, 4, 40.0);
        bent[3].0 += 0.8;
        let q = from_pitch(&bent, 2.0, 0.05).unwrap();
        assert!(q.spread > 0.0 && q.spread < 0.05, "{q:?}");
        assert!((q.scale.mm_per_px - 0.05).abs() < 1e-3);
    }

    #[test]
    fn a_missing_or_an_extra_mark_does_not_move_the_median() {
        let mut marks = lattice(5, 4, 40.0);
        marks.remove(7);
        let p = from_pitch(&marks, 2.0, 0.05).unwrap();
        assert!((p.scale.mm_per_px - 0.05).abs() < 1e-12, "{p:?}");
    }

    #[test]
    fn a_target_seen_at_a_slant_is_refused_and_the_tolerance_can_be_widened() {
        // the pitch grows from 40 pixels at one side to 48 at the other
        let xs = [20.0, 60.0, 100.0, 144.0, 192.0];
        let marks: Vec<(f64, f64)> = [30.0, 78.0]
            .iter()
            .flat_map(|&y| xs.iter().map(move |&x| (x, y)))
            .collect();
        let why = from_pitch(&marks, 2.0, 0.05).unwrap_err();
        assert!(why.contains("20.0 %") && why.contains("5.0 %"), "{why}");
        assert!(from_pitch(&marks, 2.0, 0.25).is_ok());
    }

    #[test]
    fn what_makes_no_sense_is_refused() {
        let ok = lattice(3, 3, 40.0);
        for pitch in [0.0, -1.0, f64::NAN] {
            assert!(from_pitch(&ok, pitch, 0.05).is_err());
        }
        for tolerance in [-0.1, f64::NAN, f64::INFINITY] {
            assert!(from_pitch(&ok, 2.0, tolerance).is_err());
        }
        assert!(from_pitch(&ok[..3], 2.0, 0.05)
            .unwrap_err()
            .contains("at least 4"));
        let mut twice = ok.clone();
        twice.push(ok[0]);
        assert!(from_pitch(&twice, 2.0, 0.05)
            .unwrap_err()
            .contains("same place"));
        let mut nan = ok.clone();
        nan[2].0 = f64::NAN;
        assert!(from_pitch(&nan, 2.0, 0.05).is_err());
        let many = lattice(50, 50, 10.0);
        assert!(from_pitch(&many, 2.0, 0.05).unwrap_err().contains("2000"));
    }
}
