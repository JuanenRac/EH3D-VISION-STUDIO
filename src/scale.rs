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
