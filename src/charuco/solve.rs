//! The calibration from the corners of the board that were found. The same corners are fitted three ways: a full projective map (a homography), an affine map and a similarity (a turn, a
//! shift and one scale). When the camera looks straight down at a flat board, no lens error and a pixel that is square, the three agree on the scale; when they do not, the camera is tilted,
//! the lens bends the picture, or a corner was taken for another, and the size of the disagreement is the measure of how far the calibration can be trusted. Two more checks stand against
//! it: what the fit leaves over at the corners, and a distance measured on the board with a ruler. A calibration that fails any of them is refused with the key that governs it, and
//! nothing in it is shifted by an offset that the person did not write: the offset of the camera is a value in the configuration that is added where the calibration is used, never inside it.

use super::detect::Detection;
use super::BoardSpec;
use crate::capture::{CapError, Code};
use crate::perspective::Homography;

/// What is asked of a calibration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// The fewest corners the fit is made from.
    pub min_corners: usize,
    /// The most the fit may leave at the corners, in pixels (root mean square).
    pub max_residual_px: f64,
    /// The most the three methods may differ in where they put the corners, as a share of the side of a square.
    pub max_method_gap: f64,
    /// The distance measured on the board between the two outer corners of a row, in millimetres; 0 when it was not measured.
    pub check_mm: f64,
    /// How far the distance found may be from the measured one, in millimetres.
    pub check_tol_mm: f64,
    /// Whether a calibration that was not checked against a measured distance may be used.
    pub allow_unchecked: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            min_corners: 12,
            max_residual_px: 0.5,
            max_method_gap: 0.02,
            check_mm: 0.0,
            check_tol_mm: 0.05,
            allow_unchecked: false,
        }
    }
}

/// Whether the values can be used.
pub fn check_settings(s: &Settings) -> Result<(), CapError> {
    let bad = |code, key: &'static str, why: &str| Err(CapError::with_key(code, key, why));
    if !(4..=400).contains(&s.min_corners) {
        return bad(
            Code::CalibFewCorners,
            "calib_min_corners",
            "it must be from 4 to 400",
        );
    }
    if !(s.max_residual_px.is_finite() && s.max_residual_px > 0.0 && s.max_residual_px <= 20.0) {
        return bad(
            Code::CalibResidual,
            "calib_max_residual_px",
            "it must be more than 0 and at most 20 pixels",
        );
    }
    if !(s.max_method_gap.is_finite() && s.max_method_gap > 0.0 && s.max_method_gap <= 1.0) {
        return bad(
            Code::CalibMethodsDisagree,
            "calib_max_method_gap",
            "it must be more than 0 and at most 1",
        );
    }
    if !(s.check_mm.is_finite() && (0.0..=2000.0).contains(&s.check_mm)) {
        return bad(
            Code::CalibCheckFailed,
            "calib_check_mm",
            "it must be 0 (not measured) or up to 2000 millimetres",
        );
    }
    if !(s.check_tol_mm.is_finite() && s.check_tol_mm > 0.0 && s.check_tol_mm <= 10.0) {
        return bad(
            Code::CalibCheckFailed,
            "calib_check_tol_mm",
            "it must be more than 0 and at most 10 millimetres",
        );
    }
    Ok(())
}

/// A solution of a linear system (Gauss with the largest pivot); `None` when it has no single solution.
pub fn gauss(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        let best = (col..n).max_by(|&i, &j| {
            a[i][col]
                .abs()
                .partial_cmp(&a[j][col].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        if a[best][col].is_nan() || a[best][col].abs() <= 1e-12 {
            return None;
        }
        a.swap(col, best);
        b.swap(col, best);
        for row in col + 1..n {
            let f = a[row][col] / a[col][col];
            let (top, bottom) = a.split_at_mut(row);
            for (cell, p) in bottom[0][col..n].iter_mut().zip(&top[col][col..n]) {
                *cell -= f * p;
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let mut v = b[row];
        for k in row + 1..n {
            v -= a[row][k] * x[k];
        }
        x[row] = v / a[row][row];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

type P = (f64, f64);

fn normalise(p: &[P]) -> ([f64; 9], Vec<P>) {
    let n = p.len() as f64;
    let (mx, my) = (
        p.iter().map(|q| q.0).sum::<f64>() / n,
        p.iter().map(|q| q.1).sum::<f64>() / n,
    );
    let mean_d = p
        .iter()
        .map(|q| ((q.0 - mx).powi(2) + (q.1 - my).powi(2)).sqrt())
        .sum::<f64>()
        / n;
    let s = if mean_d > 1e-12 {
        std::f64::consts::SQRT_2 / mean_d
    } else {
        1.0
    };
    (
        [s, 0.0, -s * mx, 0.0, s, -s * my, 0.0, 0.0, 1.0],
        p.iter().map(|q| ((q.0 - mx) * s, (q.1 - my) * s)).collect(),
    )
}

fn mul3(a: &[f64; 9], b: &[f64; 9]) -> [f64; 9] {
    let mut o = [0.0; 9];
    for r in 0..3 {
        for c in 0..3 {
            o[r * 3 + c] = (0..3).map(|k| a[r * 3 + k] * b[k * 3 + c]).sum();
        }
    }
    o
}

/// The projective map that takes the points `from` to the points `to` with the least squared error (the normalised direct linear transform). At least four points, not all on a line.
pub fn fit_homography(from: &[P], to: &[P]) -> Option<Homography> {
    if from.len() != to.len() || from.len() < 4 {
        return None;
    }
    let (ts, ns) = normalise(from);
    let (td, nd) = normalise(to);
    let mut ata = vec![vec![0.0; 8]; 8];
    let mut atb = vec![0.0; 8];
    for (&(x, y), &(u, v)) in ns.iter().zip(&nd) {
        for (row, rhs) in [
            ([x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y], u),
            ([0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y], v),
        ] {
            for i in 0..8 {
                for j in 0..8 {
                    ata[i][j] += row[i] * row[j];
                }
                atb[i] += row[i] * rhs;
            }
        }
    }
    let h = gauss(ata, atb)?;
    let hn = [h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7], 1.0];
    let inv_td = Homography(td).inverse()?.0;
    let mut full = mul3(&inv_td, &mul3(&hn, &ts));
    let last = full[8];
    if last.is_nan() || last.abs() <= 1e-12 {
        return None;
    }
    for v in &mut full {
        *v /= last;
    }
    full.iter()
        .all(|v| v.is_finite())
        .then_some(Homography(full))
}

/// The affine map `[a b; c d]` and the shift that take `from` to `to` with the least squared error: `(a, b, tx, c, d, ty)`.
pub fn fit_affine(from: &[P], to: &[P]) -> Option<[f64; 6]> {
    if from.len() != to.len() || from.len() < 3 {
        return None;
    }
    let mut ata = vec![vec![0.0; 3]; 3];
    let (mut bx, mut by) = (vec![0.0; 3], vec![0.0; 3]);
    for (&(x, y), &(u, v)) in from.iter().zip(to) {
        let row = [x, y, 1.0];
        for i in 0..3 {
            for j in 0..3 {
                ata[i][j] += row[i] * row[j];
            }
            bx[i] += row[i] * u;
            by[i] += row[i] * v;
        }
    }
    let sx = gauss(ata.clone(), bx)?;
    let sy = gauss(ata, by)?;
    Some([sx[0], sx[1], sx[2], sy[0], sy[1], sy[2]])
}

/// The similarity (a turn, one scale and a shift, no mirror) that takes `from` to `to` with the least squared error (Umeyama): `(scale, turn in radians, tx, ty)`.
pub fn fit_similarity(from: &[P], to: &[P]) -> Option<(f64, f64, f64, f64)> {
    if from.len() != to.len() || from.len() < 2 {
        return None;
    }
    let n = from.len() as f64;
    let mean = |v: &[P]| {
        (
            v.iter().map(|p| p.0).sum::<f64>() / n,
            v.iter().map(|p| p.1).sum::<f64>() / n,
        )
    };
    let (cf, ct) = (mean(from), mean(to));
    let (mut dot, mut cross, mut var) = (0.0, 0.0, 0.0);
    for (f, t) in from.iter().zip(to) {
        let (fx, fy, tx, ty) = (f.0 - cf.0, f.1 - cf.1, t.0 - ct.0, t.1 - ct.1);
        dot += fx * tx + fy * ty;
        cross += fx * ty - fy * tx;
        var += fx * fx + fy * fy;
    }
    if var.is_nan() || var <= 1e-12 {
        return None;
    }
    let turn = cross.atan2(dot);
    let scale = (dot * turn.cos() + cross * turn.sin()) / var;
    let (s, c) = turn.sin_cos();
    let tx = ct.0 - scale * (c * cf.0 - s * cf.1);
    let ty = ct.1 - scale * (s * cf.0 + c * cf.1);
    (scale.is_finite() && scale > 0.0).then_some((scale, turn, tx, ty))
}

/// The scale of a map at a point: the millimetres a pixel stands for there, as the square root of the area of the image of one pixel, and the ratio of the scale along x to the one along y.
pub fn local_scale(h: &Homography, at: P) -> Option<(f64, f64)> {
    let e = 0.5;
    let (x1, x0) = (h.apply((at.0 + e, at.1))?, h.apply((at.0 - e, at.1))?);
    let (y1, y0) = (h.apply((at.0, at.1 + e))?, h.apply((at.0, at.1 - e))?);
    let (jx, jy) = ((x1.0 - x0.0, x1.1 - x0.1), (y1.0 - y0.0, y1.1 - y0.1));
    let det = (jx.0 * jy.1 - jx.1 * jy.0).abs();
    let (sx, sy) = (
        (jx.0.powi(2) + jx.1.powi(2)).sqrt(),
        (jy.0.powi(2) + jy.1.powi(2)).sqrt(),
    );
    (det > 1e-12 && sy > 1e-12).then(|| (det.sqrt(), sx / sy))
}

/// What one method found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Method {
    pub name: &'static str,
    /// Millimetres for a pixel.
    pub mm_per_px: f64,
    /// What it leaves at the corners, in millimetres (root mean square).
    pub rms_mm: f64,
}

/// What the distance measured on the board says.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Check {
    /// Nobody measured a distance.
    NotMeasured,
    Measured {
        measured_mm: f64,
        found_mm: f64,
        error_mm: f64,
    },
}

/// A calibration that passed.
#[derive(Debug, Clone, PartialEq)]
pub struct Calibration {
    pub spec: BoardSpec,
    pub corners: usize,
    /// The map from pixels to millimetres on the board.
    pub homography: Homography,
    /// Millimetres for a pixel at the middle of the corners that were seen.
    pub mm_per_px: f64,
    /// The scale along x over the scale along y (1 is a square pixel).
    pub aspect: f64,
    /// What the homography leaves at the corners, in pixels (root mean square) and the largest.
    pub rms_px: f64,
    pub max_px: f64,
    pub methods: [Method; 3],
    /// How far apart the three methods put the corners: the largest root mean square distance between two of them, as a share of the side of a square.
    pub gap: f64,
    /// The scales of the three methods: (largest - smallest) over their mean.
    pub scale_gap: f64,
    pub check: Check,
    /// Whether the calibration was checked against a measured distance.
    pub verified: bool,
    /// What the pictures it was made from prove: `picture` (a file), `camera` or `simulator`.
    pub evidence: &'static str,
}

fn rms(v: impl Iterator<Item = f64>) -> f64 {
    let (sum, n) = v.fold((0.0, 0usize), |(s, n), e| (s + e * e, n + 1));
    if n == 0 {
        0.0
    } else {
        (sum / n as f64).sqrt()
    }
}

/// The calibration from the corners of a detection, or the reason it is not good enough to be used.
pub fn calibrate(det: &Detection, spec: &BoardSpec, s: &Settings) -> Result<Calibration, CapError> {
    check_settings(s)?;
    let n = det.corners.len();
    if n < s.min_corners {
        return Err(CapError::with_key(
            Code::CalibFewCorners,
            "calib_min_corners",
            format!("{n} corners found, {} needed", s.min_corners),
        ));
    }
    let px: Vec<P> = det.corners.iter().map(|c| c.px).collect();
    let mm: Vec<P> = det.corners.iter().map(|c| c.mm).collect();
    let singular = || {
        CapError::new(
            Code::CalibAmbiguous,
            "the corners are on one line, no map can be made",
        )
    };
    let h = fit_homography(&px, &mm).ok_or_else(singular)?;
    let a = fit_affine(&px, &mm).ok_or_else(singular)?;
    let (sim_scale, sim_turn, sim_tx, sim_ty) = fit_similarity(&px, &mm).ok_or_else(singular)?;

    let centre = (
        px.iter().map(|p| p.0).sum::<f64>() / n as f64,
        px.iter().map(|p| p.1).sum::<f64>() / n as f64,
    );
    let (h_scale, aspect) = local_scale(&h, centre).ok_or_else(singular)?;
    let a_scale = (a[0] * a[4] - a[1] * a[3]).abs().sqrt();
    let (ss, sc) = sim_turn.sin_cos();
    let by_h: Vec<P> = px
        .iter()
        .map(|p| h.apply(*p).unwrap_or((f64::NAN, f64::NAN)))
        .collect();
    let by_a: Vec<P> = px
        .iter()
        .map(|p| {
            (
                a[0] * p.0 + a[1] * p.1 + a[2],
                a[3] * p.0 + a[4] * p.1 + a[5],
            )
        })
        .collect();
    let by_s: Vec<P> = px
        .iter()
        .map(|p| {
            (
                sim_scale * (sc * p.0 - ss * p.1) + sim_tx,
                sim_scale * (ss * p.0 + sc * p.1) + sim_ty,
            )
        })
        .collect();
    let apart = |u: &[P], v: &[P]| {
        rms(u
            .iter()
            .zip(v)
            .map(|(p, q)| ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt()))
    };
    let rms_h = apart(&by_h, &mm);
    let rms_a = apart(&by_a, &mm);
    let rms_s = apart(&by_s, &mm);
    let gap_mm = apart(&by_h, &by_a)
        .max(apart(&by_h, &by_s))
        .max(apart(&by_a, &by_s));
    let methods = [
        Method {
            name: "homography",
            mm_per_px: h_scale,
            rms_mm: rms_h,
        },
        Method {
            name: "affine",
            mm_per_px: a_scale,
            rms_mm: rms_a,
        },
        Method {
            name: "similarity",
            mm_per_px: sim_scale,
            rms_mm: rms_s,
        },
    ];
    if methods
        .iter()
        .any(|m| !(m.mm_per_px.is_finite() && m.mm_per_px > 0.0 && m.rms_mm.is_finite()))
    {
        return Err(singular());
    }

    // the residual in pixels: the corners put through the inverse map
    let back = h.inverse().ok_or_else(singular)?;
    let errors: Vec<f64> = px
        .iter()
        .zip(&mm)
        .map(|(p, m)| {
            back.apply(*m).map_or(f64::INFINITY, |q| {
                ((q.0 - p.0).powi(2) + (q.1 - p.1).powi(2)).sqrt()
            })
        })
        .collect();
    let rms_px = rms(errors.iter().copied());
    let max_px = errors.iter().copied().fold(0.0, f64::max);
    if rms_px.is_nan() || rms_px > s.max_residual_px {
        return Err(CapError::with_key(
            Code::CalibResidual,
            "calib_max_residual_px",
            format!(
                "{rms_px:.3} px left at the corners (largest {max_px:.3}), {} allowed",
                s.max_residual_px
            ),
        ));
    }
    let gap = gap_mm / spec.square_mm;
    let hi = methods.iter().map(|m| m.mm_per_px).fold(f64::MIN, f64::max);
    let lo = methods.iter().map(|m| m.mm_per_px).fold(f64::MAX, f64::min);
    let mean = methods.iter().map(|m| m.mm_per_px).sum::<f64>() / 3.0;
    let scale_gap = (hi - lo) / mean;
    if gap.is_nan() || gap > s.max_method_gap {
        return Err(CapError::with_key(
            Code::CalibMethodsDisagree,
            "calib_max_method_gap",
            format!(
                "the corners are put {:.3} mm apart by the methods ({:.1} % of a square), {:.1} % allowed; scales {:.5}, {:.5} and {:.5} mm/px",
                gap_mm,
                gap * 100.0,
                s.max_method_gap * 100.0,
                methods[0].mm_per_px,
                methods[1].mm_per_px,
                methods[2].mm_per_px
            ),
        ));
    }

    // the ruler: the two outer corners of a row
    let check = if s.check_mm > 0.0 {
        let ends = (1..spec.rows).find_map(|j| {
            let a = det.corners.iter().find(|c| c.id == (1, j))?;
            let b = det.corners.iter().find(|c| c.id == (spec.cols - 1, j))?;
            Some((a.px, b.px))
        });
        let Some((pa, pb)) = ends else {
            return Err(CapError::with_key(
                Code::CalibUnverified,
                "calib_check_mm",
                "the two outer corners of a row were not both seen, so the measured distance cannot be compared",
            ));
        };
        let (qa, qb) = (
            h.apply(pa).ok_or_else(singular)?,
            h.apply(pb).ok_or_else(singular)?,
        );
        let found = ((qa.0 - qb.0).powi(2) + (qa.1 - qb.1).powi(2)).sqrt();
        let error = found - s.check_mm;
        if error.abs() > s.check_tol_mm {
            return Err(CapError::with_key(
                Code::CalibCheckFailed,
                "calib_check_tol_mm",
                format!(
                    "{found:.3} mm found between the corners, {} mm measured: {:+.3} mm off, {} allowed",
                    s.check_mm, error, s.check_tol_mm
                ),
            ));
        }
        Check::Measured {
            measured_mm: s.check_mm,
            found_mm: found,
            error_mm: error,
        }
    } else {
        Check::NotMeasured
    };
    let verified = matches!(check, Check::Measured { .. });
    if !verified && !s.allow_unchecked {
        return Err(CapError::with_key(
            Code::CalibUnverified,
            "calib_check_mm",
            "measure the distance between the two outer corners of a row and write it, or allow an unchecked calibration with calib_allow_unchecked",
        ));
    }
    Ok(Calibration {
        spec: *spec,
        corners: n,
        homography: h,
        mm_per_px: h_scale,
        aspect,
        rms_px,
        max_px,
        methods,
        gap,
        scale_gap,
        check,
        verified,
        evidence: "picture",
    })
}

/// An offset of the camera written in the configuration: where the middle of the picture is in the frame of the machine, in millimetres, and how the camera is turned.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CameraOffset {
    pub x_mm: f64,
    pub y_mm: f64,
    pub deg: f64,
}

impl Calibration {
    /// The same calibration with the evidence of the pictures it was made from (`camera`, `picture` or `simulator`).
    pub fn with_evidence(mut self, evidence: &str) -> Result<Calibration, CapError> {
        self.evidence = crate::convention::EVIDENCE
            .iter()
            .copied()
            .find(|e| *e == evidence)
            .ok_or_else(|| {
                CapError::new(
                    Code::CalibFileBad,
                    format!("{evidence:?} is not an evidence"),
                )
            })?;
        Ok(self)
    }

    /// Whether this calibration may be used with a camera that gives frames of this evidence: a calibration made from simulated frames says nothing about a real lens, and is refused for one.
    pub fn check_evidence(&self, camera_evidence: &str) -> Result<(), CapError> {
        if self.evidence == "simulator" && camera_evidence == "camera" {
            return Err(CapError::new(
                Code::CalibFileBad,
                "the calibration was made from simulated frames and is not used with a real camera",
            ));
        }
        Ok(())
    }
}

impl CameraOffset {
    /// A point in the frame of the camera, turned by the offset and shifted by it: its place in the frame of the machine.
    pub fn apply(&self, p: P) -> P {
        let (s, c) = self.deg.to_radians().sin_cos();
        (c * p.0 - s * p.1 + self.x_mm, s * p.0 + c * p.1 + self.y_mm)
    }
}

impl Calibration {
    /// Where a pixel is on the board, in millimetres, as the calibration alone says.
    pub fn to_board(&self, px: P) -> Option<P> {
        self.homography.apply(px)
    }

    /// Where a point of the board is in the picture.
    pub fn to_pixel(&self, mm: P) -> Option<P> {
        self.homography.inverse()?.apply(mm)
    }

    /// Where a pixel is on the machine: the position on the board turned by the offset of the camera and shifted by it. Nothing but the offset that is passed is added.
    pub fn to_machine(&self, px: P, offset: &CameraOffset) -> Option<P> {
        Some(offset.apply(self.to_board(px)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::charuco::detect::Corner;

    fn grid(spec: &BoardSpec, to_px: impl Fn(P) -> P, jitter: impl Fn(usize) -> P) -> Detection {
        let mut corners = Vec::new();
        for j in 1..spec.rows {
            for i in 1..spec.cols {
                let mm = spec.corner_mm(i, j);
                let mut px = to_px(mm);
                let e = jitter(corners.len());
                px = (px.0 + e.0, px.1 + e.1);
                corners.push(Corner {
                    id: (i, j),
                    px,
                    mm,
                    quality: 1.0,
                });
            }
        }
        Detection {
            corners,
            markers: Vec::new(),
            marker_rms_px: 0.0,
            square_px: 40.0,
        }
    }

    fn flat(mm: P) -> P {
        // 8 pixels for a millimetre, turned 20 degrees, with the middle of the board at (300, 200)
        let (s, c) = 20f64.to_radians().sin_cos();
        let (x, y) = (mm.0 - 20.0, mm.1 - 15.0);
        (300.0 + 8.0 * (c * x - s * y), 200.0 + 8.0 * (s * x + c * y))
    }

    fn no_jitter(_: usize) -> P {
        (0.0, 0.0)
    }

    fn lenient() -> Settings {
        Settings {
            allow_unchecked: true,
            ..Settings::default()
        }
    }

    #[test]
    fn the_least_squares_maps_find_the_scale_and_the_three_methods_agree_on_a_flat_view() {
        let spec = BoardSpec::default();
        let det = grid(&spec, flat, no_jitter);
        let cal = calibrate(&det, &spec, &lenient()).unwrap();
        assert!((cal.mm_per_px - 0.125).abs() < 1e-6, "{}", cal.mm_per_px);
        assert!(cal.gap < 1e-6 && cal.scale_gap < 1e-6);
        assert!(cal.rms_px < 1e-6 && cal.max_px < 1e-6);
        assert!((cal.aspect - 1.0).abs() < 1e-9);
        assert_eq!(cal.corners, 35);
        assert!(!cal.verified);
        for m in cal.methods {
            assert!((m.mm_per_px - 0.125).abs() < 1e-6, "{m:?}");
        }
    }

    #[test]
    fn a_pixel_that_is_not_square_shows_in_the_aspect_and_is_refused_by_default() {
        let spec = BoardSpec::default();
        let det = grid(&spec, |m| (100.0 + 8.0 * m.0, 100.0 + 9.0 * m.1), no_jitter);
        let cal = calibrate(
            &det,
            &spec,
            &Settings {
                max_method_gap: 1.0,
                ..lenient()
            },
        )
        .unwrap();
        assert!((cal.aspect - 9.0 / 8.0).abs() < 1e-6, "{}", cal.aspect);
        assert!(cal.gap > 0.02, "{}", cal.gap);
        let e = calibrate(&det, &spec, &lenient()).unwrap_err();
        assert_eq!(e.code, Code::CalibMethodsDisagree);
    }

    #[test]
    fn a_bent_view_makes_the_methods_disagree_and_is_refused_naming_the_key() {
        let spec = BoardSpec::default();
        // one side of the board nearer than the other: the scale changes along x
        let tilt = Homography([8.0, 0.0, 100.0, 0.0, 8.0, 100.0, 0.004, 0.0, 1.0]);
        let det = grid(&spec, |m| tilt.apply(m).unwrap(), no_jitter);
        let e = calibrate(
            &det,
            &spec,
            &Settings {
                max_residual_px: 20.0,
                ..lenient()
            },
        )
        .unwrap_err();
        assert!(matches!(e.code, Code::CalibMethodsDisagree), "{e}");
        assert_eq!(e.key, Some("calib_max_method_gap"));
    }

    #[test]
    fn corners_with_too_much_error_leave_a_residual_that_is_refused_and_it_is_the_number_in_the_message(
    ) {
        let spec = BoardSpec::default();
        let det = grid(&spec, flat, |k| {
            if k % 5 == 0 {
                (1.5, -1.0)
            } else {
                (0.0, 0.0)
            }
        });
        let e = calibrate(&det, &spec, &lenient()).unwrap_err();
        assert_eq!(e.code, Code::CalibResidual);
        assert_eq!(e.key, Some("calib_max_residual_px"));
        assert!(e.detail.contains("px left"), "{e}");
    }

    #[test]
    fn too_few_corners_are_refused() {
        let spec = BoardSpec::default();
        let mut det = grid(&spec, flat, no_jitter);
        det.corners.truncate(8);
        let e = calibrate(&det, &spec, &lenient()).unwrap_err();
        assert_eq!(e.code, Code::CalibFewCorners);
        assert_eq!(e.key, Some("calib_min_corners"));
    }

    #[test]
    fn without_a_measured_distance_the_calibration_is_refused_unless_it_is_allowed() {
        let spec = BoardSpec::default();
        let det = grid(&spec, flat, no_jitter);
        let e = calibrate(&det, &spec, &Settings::default()).unwrap_err();
        assert_eq!(e.code, Code::CalibUnverified);
        assert_eq!(e.key, Some("calib_check_mm"));
        assert!(calibrate(&det, &spec, &lenient()).is_ok());
    }

    #[test]
    fn the_measured_distance_is_compared_with_the_one_the_calibration_gives() {
        let spec = BoardSpec::default();
        let det = grid(&spec, flat, no_jitter);
        // the outer corners of a row are 6 squares of 5 mm apart: 30 mm
        let good = Settings {
            check_mm: 30.0,
            ..Settings::default()
        };
        let cal = calibrate(&det, &spec, &good).unwrap();
        assert!(cal.verified);
        match cal.check {
            Check::Measured {
                measured_mm,
                found_mm,
                error_mm,
            } => {
                assert_eq!(measured_mm, 30.0);
                assert!((found_mm - 30.0).abs() < 1e-6 && error_mm.abs() < 1e-6);
            }
            Check::NotMeasured => panic!("it was measured"),
        }
        // a board printed 2 % too big: the ruler says 30.6 mm and the calibration says 30
        let e = calibrate(
            &det,
            &spec,
            &Settings {
                check_mm: 30.6,
                ..Settings::default()
            },
        )
        .unwrap_err();
        assert_eq!(e.code, Code::CalibCheckFailed);
        assert_eq!(e.key, Some("calib_check_tol_mm"));
        assert!(e.detail.contains("-0.600"), "{e}");
    }

    #[test]
    fn a_ruler_distance_cannot_be_used_when_the_ends_of_the_row_were_not_seen() {
        let spec = BoardSpec::default();
        let mut det = grid(&spec, flat, no_jitter);
        det.corners.retain(|c| c.id.0 != 7);
        let e = calibrate(
            &det,
            &spec,
            &Settings {
                check_mm: 30.0,
                min_corners: 4,
                ..Settings::default()
            },
        )
        .unwrap_err();
        assert_eq!(e.code, Code::CalibUnverified);
    }

    #[test]
    fn settings_that_make_no_sense_are_refused_with_their_key() {
        let base = Settings::default();
        for (s, key) in [
            (
                Settings {
                    min_corners: 3,
                    ..base
                },
                "calib_min_corners",
            ),
            (
                Settings {
                    max_residual_px: 0.0,
                    ..base
                },
                "calib_max_residual_px",
            ),
            (
                Settings {
                    max_method_gap: 2.0,
                    ..base
                },
                "calib_max_method_gap",
            ),
            (
                Settings {
                    check_mm: -1.0,
                    ..base
                },
                "calib_check_mm",
            ),
            (
                Settings {
                    check_tol_mm: f64::NAN,
                    ..base
                },
                "calib_check_tol_mm",
            ),
        ] {
            assert_eq!(check_settings(&s).unwrap_err().key, Some(key));
        }
    }

    #[test]
    fn the_calibration_adds_no_offset_of_its_own_and_the_offset_passed_is_added_exactly() {
        let spec = BoardSpec::default();
        let det = grid(&spec, flat, no_jitter);
        let cal = calibrate(&det, &spec, &lenient()).unwrap();
        let p = flat(spec.corner_mm(3, 2));
        let board = cal.to_board(p).unwrap();
        assert!((board.0 - 15.0).abs() < 1e-6 && (board.1 - 10.0).abs() < 1e-6);
        let none = cal.to_machine(p, &CameraOffset::default()).unwrap();
        assert_eq!(none, board);
        let shifted = cal
            .to_machine(
                p,
                &CameraOffset {
                    x_mm: 1.5,
                    y_mm: -2.0,
                    deg: 0.0,
                },
            )
            .unwrap();
        assert!(
            (shifted.0 - board.0 - 1.5).abs() < 1e-9 && (shifted.1 - board.1 + 2.0).abs() < 1e-9
        );
        let turned = cal
            .to_machine(
                (0.0, 0.0),
                &CameraOffset {
                    x_mm: 0.0,
                    y_mm: 0.0,
                    deg: 90.0,
                },
            )
            .unwrap();
        let origin = cal.to_board((0.0, 0.0)).unwrap();
        assert!((turned.0 + origin.1).abs() < 1e-9 && (turned.1 - origin.0).abs() < 1e-9);
        let back = cal.to_pixel(board).unwrap();
        assert!((back.0 - p.0).abs() < 1e-6 && (back.1 - p.1).abs() < 1e-6);
    }

    #[test]
    fn the_homography_fit_recovers_a_known_projective_map_from_many_points() {
        let truth = Homography([1.2, 0.1, 5.0, -0.05, 0.9, -3.0, 0.0005, 0.0002, 1.0]);
        let from: Vec<P> = (0..6)
            .flat_map(|i| (0..5).map(move |j| (f64::from(i) * 40.0, f64::from(j) * 35.0)))
            .collect();
        let to: Vec<P> = from.iter().map(|&p| truth.apply(p).unwrap()).collect();
        let h = fit_homography(&from, &to).unwrap();
        for (&f, &t) in from.iter().zip(&to) {
            let q = h.apply(f).unwrap();
            assert!((q.0 - t.0).abs() < 1e-6 && (q.1 - t.1).abs() < 1e-6);
        }
        assert!(fit_homography(&from[..3], &to[..3]).is_none());
        let line: Vec<P> = (0..8).map(|i| (f64::from(i), f64::from(i))).collect();
        assert!(fit_homography(&line, &line).is_none());
    }

    #[test]
    fn the_similarity_and_affine_fits_recover_their_own_maps() {
        let from: Vec<P> = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 5.0), (0.0, 5.0), (4.0, 2.0)];
        let (s, c) = 0.3f64.sin_cos();
        let to: Vec<P> = from
            .iter()
            .map(|p| {
                (
                    2.0 * (c * p.0 - s * p.1) + 7.0,
                    2.0 * (s * p.0 + c * p.1) - 1.0,
                )
            })
            .collect();
        let (scale, turn, tx, ty) = fit_similarity(&from, &to).unwrap();
        assert!(
            (scale - 2.0).abs() < 1e-9
                && (turn - 0.3).abs() < 1e-9
                && (tx - 7.0).abs() < 1e-9
                && (ty + 1.0).abs() < 1e-9
        );
        let a = fit_affine(&from, &to).unwrap();
        assert!(
            (a[0] - 2.0 * c).abs() < 1e-9
                && (a[1] + 2.0 * s).abs() < 1e-9
                && (a[2] - 7.0).abs() < 1e-9
        );
        assert!(fit_similarity(&[(1.0, 1.0), (1.0, 1.0)], &[(0.0, 0.0), (1.0, 1.0)]).is_none());
        assert!(fit_affine(&from[..2], &to[..2]).is_none());
    }

    #[test]
    fn a_linear_system_with_no_single_solution_gives_none() {
        assert!(gauss(vec![vec![1.0, 2.0], vec![2.0, 4.0]], vec![1.0, 2.0]).is_none());
        let x = gauss(vec![vec![2.0, 1.0], vec![1.0, 3.0]], vec![5.0, 10.0]).unwrap();
        assert!((x[0] - 1.0).abs() < 1e-12 && (x[1] - 3.0).abs() < 1e-12);
    }
}
