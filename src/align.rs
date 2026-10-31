//! Where the board really is: the design says where the fiducials are, the camera says where they were found, and the transform that takes the first to the second (a turn, a shift
//! and, if allowed, a change of size) is the correction that the machine applies to everything else of the board.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub dx: f64,
    pub dy: f64,
    /// Counter-clockwise, in degrees.
    pub theta_deg: f64,
    pub scale: f64,
    /// What is left between the transformed design points and the measured ones.
    pub rms_mm: f64,
}

impl Transform {
    /// Where a point of the design is on the machine.
    pub fn apply(&self, p: (f64, f64)) -> (f64, f64) {
        let (s, c) = self.theta_deg.to_radians().sin_cos();
        (
            self.scale * (c * p.0 - s * p.1) + self.dx,
            self.scale * (s * p.0 + c * p.1) + self.dy,
        )
    }

    /// Where a point of the machine is on the design.
    pub fn inverse(&self, p: (f64, f64)) -> (f64, f64) {
        let (s, c) = self.theta_deg.to_radians().sin_cos();
        let (x, y) = ((p.0 - self.dx) / self.scale, (p.1 - self.dy) / self.scale);
        (c * x + s * y, -s * x + c * y)
    }
}

/// The turn and the shift (and the size, with `allow_scale`) that take the `design` points closest to the `measured` ones, by least squares. At least two points that are not the same.
pub fn fit(
    design: &[(f64, f64)],
    measured: &[(f64, f64)],
    allow_scale: bool,
) -> Result<Transform, String> {
    if design.len() != measured.len() || design.len() < 2 {
        return Err(format!(
            "{} design and {} measured points: they must be the same number, and at least 2",
            design.len(),
            measured.len()
        ));
    }
    if design
        .iter()
        .chain(measured)
        .any(|p| !p.0.is_finite() || !p.1.is_finite())
    {
        return Err("a point is not a number".into());
    }
    let n = design.len() as f64;
    let mean = |v: &[(f64, f64)]| {
        (
            v.iter().map(|p| p.0).sum::<f64>() / n,
            v.iter().map(|p| p.1).sum::<f64>() / n,
        )
    };
    let (cd, cm) = (mean(design), mean(measured));
    let (mut dot, mut cross, mut dd) = (0.0, 0.0, 0.0);
    for (d, m) in design.iter().zip(measured) {
        let (dx, dy, mx, my) = (d.0 - cd.0, d.1 - cd.1, m.0 - cm.0, m.1 - cm.1);
        dot += dx * mx + dy * my;
        cross += dx * my - dy * mx;
        dd += dx * dx + dy * dy;
    }
    if dd < 1e-12 {
        return Err("the design points are all the same point: no turn can be measured".into());
    }
    let theta = cross.atan2(dot);
    let scale = if allow_scale {
        (dot * theta.cos() + cross * theta.sin()) / dd
    } else {
        1.0
    };
    if !(scale.is_finite() && scale > 0.0) {
        return Err(format!(
            "the size of the board came out as {scale}: the points do not describe the same board"
        ));
    }
    let (s, c) = theta.sin_cos();
    let (dx, dy) = (
        cm.0 - scale * (c * cd.0 - s * cd.1),
        cm.1 - scale * (s * cd.0 + c * cd.1),
    );
    let t = Transform {
        dx,
        dy,
        theta_deg: theta.to_degrees(),
        scale,
        rms_mm: 0.0,
    };
    let sum: f64 = design
        .iter()
        .zip(measured)
        .map(|(d, m)| {
            let a = t.apply(*d);
            (a.0 - m.0).powi(2) + (a.1 - m.1).powi(2)
        })
        .sum();
    Ok(Transform {
        rms_mm: (sum / n).sqrt(),
        ..t
    })
}

/// The transform is accepted only when the points fit it well (`max_rms_mm`) and, with a size, when the board is not stretched more than `max_stretch` (0.01 is one percent).
pub fn accept(t: &Transform, max_rms_mm: f64, max_stretch: f64) -> Result<(), String> {
    if t.rms_mm > max_rms_mm {
        return Err(format!("the fiducials fit the design to {:.3} mm, more than the {max_rms_mm} mm allowed: one of them is probably not the one that it should be", t.rms_mm));
    }
    if (t.scale - 1.0).abs() > max_stretch {
        return Err(format!(
            "the board is {:.2} % bigger or smaller than its design, more than the {:.2} % allowed",
            (t.scale - 1.0) * 100.0,
            max_stretch * 100.0
        ));
    }
    Ok(())
}
