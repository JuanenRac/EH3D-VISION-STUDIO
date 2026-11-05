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

/// The marks as the options of `eh3d-isocam`, one line for each pair: `--fiducial DESIGN_X,DESIGN_Y,MEASURED_X,MEASURED_Y`.
pub fn isocam_args(design: &[(f64, f64)], measured: &[(f64, f64)]) -> Vec<String> {
    design
        .iter()
        .zip(measured)
        .map(|(d, m)| format!("--fiducial {},{},{},{}", d.0, d.1, m.0, m.1))
        .collect()
}

/// Puts the `measured` points (the marks as the camera found them, in any order) in the order of the `design` ones: each design point takes the measured point that lies closest to it once
/// both sets are moved to their own centre, so a board that is shifted is paired right and a board that is turned a little too. The marks must be well apart from each other; two design points
/// that would take the same mark are not told apart, and the points are paired by distance only, so a mark that is missing or extra is an error.
pub fn pair_nearest(
    design: &[(f64, f64)],
    measured: &[(f64, f64)],
) -> Result<Vec<(f64, f64)>, String> {
    if design.len() != measured.len() || design.len() < 2 {
        return Err(format!(
            "{} design and {} measured points: they must be the same number, and at least 2",
            design.len(),
            measured.len()
        ));
    }
    let n = design.len() as f64;
    let centre = |v: &[(f64, f64)]| {
        (
            v.iter().map(|p| p.0).sum::<f64>() / n,
            v.iter().map(|p| p.1).sum::<f64>() / n,
        )
    };
    let (cd, cm) = (centre(design), centre(measured));
    let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
    for (i, d) in design.iter().enumerate() {
        for (j, m) in measured.iter().enumerate() {
            let (dx, dy) = ((d.0 - cd.0) - (m.0 - cm.0), (d.1 - cd.1) - (m.1 - cm.1));
            pairs.push((dx * dx + dy * dy, i, j));
        }
    }
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut taken: Vec<Option<usize>> = vec![None; design.len()];
    let mut used = vec![false; measured.len()];
    for (_, i, j) in pairs {
        if taken[i].is_none() && !used[j] {
            taken[i] = Some(j);
            used[j] = true;
        }
    }
    taken
        .into_iter()
        .map(|t| {
            t.map(|j| measured[j])
                .ok_or_else(|| "the marks could not be paired with the design".to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_options_for_isocam_have_one_line_for_each_pair() {
        let lines = isocam_args(&[(5.0, 5.0), (95.0, 5.5)], &[(6.0, 4.5), (96.0, 5.25)]);
        assert_eq!(lines, ["--fiducial 5,5,6,4.5", "--fiducial 95,5.5,96,5.25"]);
    }

    #[test]
    fn marks_found_in_any_order_are_paired_with_the_design() {
        let design = [(0.0, 0.0), (80.0, 0.0), (80.0, 50.0), (0.0, 50.0)];
        // the same board shifted by (3, -2) and found in another order
        let found = [(83.0, 48.0), (3.0, -2.0), (3.0, 48.0), (83.0, -2.0)];
        let paired = pair_nearest(&design, &found).unwrap();
        assert_eq!(
            paired,
            vec![(3.0, -2.0), (83.0, -2.0), (83.0, 48.0), (3.0, 48.0)]
        );
    }

    #[test]
    fn a_missing_mark_cannot_be_paired() {
        let design = [(0.0, 0.0), (80.0, 0.0), (80.0, 50.0)];
        assert!(pair_nearest(&design, &[(0.0, 0.0), (80.0, 0.0)]).is_err());
        assert!(pair_nearest(&design[..1], &[(0.0, 0.0)]).is_err());
    }

    #[test]
    fn pairing_and_fitting_find_the_turn_of_a_board() {
        let design = [(0.0, 0.0), (80.0, 0.0), (80.0, 50.0), (0.0, 50.0)];
        let t = Transform {
            dx: 4.0,
            dy: -3.0,
            theta_deg: 2.0,
            scale: 1.0,
            rms_mm: 0.0,
        };
        let mut found: Vec<(f64, f64)> = design.iter().map(|p| t.apply(*p)).collect();
        found.reverse();
        let paired = pair_nearest(&design, &found).unwrap();
        let fitted = fit(&design, &paired, false).unwrap();
        assert!((fitted.theta_deg - 2.0).abs() < 1e-6);
        assert!(fitted.rms_mm < 1e-6);
    }
}
