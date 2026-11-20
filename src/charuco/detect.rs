//! Finding the board in a picture. The markers are found first: dark spots whose outline is a square (the smallest rectangle round their points) and whose 6 by 6 cells read as a marker of the
//! dictionary, turned any way. Every marker says which marker it is and so where in the board it is; the four corners of the markers give a map from the board to the picture, and that map
//! says where each inner corner of the chessboard should be. Each is then looked for there by its own edges: the point is moved until the gradient of the picture at every pixel around it is
//! at right angles to the line from the point to the pixel, which for a corner of two crossing edges is the corner itself, whatever the turn of the board. A point whose edges do not cross
//! (a straight edge, a flat area) is not a corner and is left out.

use super::solve::fit_homography;
use super::{BoardSpec, Dictionary, BITS, CELLS};
use crate::capture::{CapError, Code};
use crate::fiducial::otsu;
use crate::image::Gray;
use crate::perspective::Homography;

/// A corner of the chessboard that was found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Corner {
    /// Which inner corner: `(i, j)`, from 1.
    pub id: (usize, usize),
    /// Where it is in the picture (a pixel covers the square from its index to the index plus one).
    pub px: (f64, f64),
    /// Where it is on the board, in millimetres.
    pub mm: (f64, f64),
    /// How much it looks like a corner of two crossing edges, 0 to 1.
    pub quality: f64,
}

/// A marker that was read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoundMarker {
    pub id: usize,
    /// Its corners in the picture, in the order of the board: top left, top right, bottom right, bottom left.
    pub quad: [(f64, f64); 4],
}

/// What was found.
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    pub corners: Vec<Corner>,
    pub markers: Vec<FoundMarker>,
    /// What the map from the markers leaves at their own corners, in pixels (root mean square).
    pub marker_rms_px: f64,
    /// The side of a square of the board in the picture, in pixels.
    pub square_px: f64,
}

/// The most spots looked at, so that a picture full of noise does not take long.
const MAX_SPOTS: usize = 600;

fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// The convex hull of points (Andrew's monotone chain), counter-clockwise in the usual axes.
fn hull(mut pts: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    pts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    pts.dedup();
    if pts.len() < 3 {
        return pts;
    }
    let mut lower: Vec<(f64, f64)> = Vec::new();
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<(f64, f64)> = Vec::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// The corners of a rectangle, its area and its two sides.
type Rect = ([(f64, f64); 4], f64, f64, f64);
/// The area, the corners and the two sides of the best rectangle so far.
type Best = (f64, [(f64, f64); 4], f64, f64);

/// The smallest rectangle round the hull: its four corners in the order top left, top right, bottom right, bottom left as the picture shows them (y down), its area and its two sides.
fn min_rect(h: &[(f64, f64)]) -> Option<Rect> {
    if h.len() < 3 {
        return None;
    }
    let mut best: Option<Best> = None;
    for k in 0..h.len() {
        let (a, b) = (h[k], h[(k + 1) % h.len()]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-9 {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len);
        let (mut lo_u, mut hi_u, mut lo_v, mut hi_v) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for p in h {
            let (u, v) = (p.0 * ux + p.1 * uy, -p.0 * uy + p.1 * ux);
            lo_u = lo_u.min(u);
            hi_u = hi_u.max(u);
            lo_v = lo_v.min(v);
            hi_v = hi_v.max(v);
        }
        let area = (hi_u - lo_u) * (hi_v - lo_v);
        if best.as_ref().is_none_or(|b| area < b.0) {
            let at = |u: f64, v: f64| (u * ux - v * uy, u * uy + v * ux);
            best = Some((
                area,
                [
                    at(lo_u, lo_v),
                    at(hi_u, lo_v),
                    at(hi_u, hi_v),
                    at(lo_u, hi_v),
                ],
                hi_u - lo_u,
                hi_v - lo_v,
            ));
        }
    }
    let (area, mut corners, w, hgt) = best?;
    // clockwise as the picture shows it, starting at the corner nearest the top left
    let cx = corners.iter().map(|c| c.0).sum::<f64>() / 4.0;
    let cy = corners.iter().map(|c| c.1).sum::<f64>() / 4.0;
    corners.sort_by(|a, b| {
        (a.1 - cy)
            .atan2(a.0 - cx)
            .partial_cmp(&(b.1 - cy).atan2(b.0 - cx))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // sorted by angle with y down this goes clockwise on the screen; start at the smallest x + y
    let first = (0..4)
        .min_by(|&i, &j| {
            (corners[i].0 + corners[i].1)
                .partial_cmp(&(corners[j].0 + corners[j].1))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(0);
    corners.rotate_left(first);
    Some((corners, area, w, hgt))
}

/// The dark spots of the picture as lists of their points (pixel centres), with the spots that are too small, too big or too many left out.
fn spots(img: &Gray, thr: u8) -> Vec<Vec<(f64, f64)>> {
    let (w, h) = (img.w, img.h);
    let max_n = w * h / 4;
    let mut seen = vec![false; w * h];
    let mut out = Vec::new();
    let mut stack = Vec::new();
    for start in 0..w * h {
        if seen[start] || img.data[start] >= thr {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let mut pts: Vec<(f64, f64)> = Vec::new();
        let mut n = 0usize;
        while let Some(i) = stack.pop() {
            n += 1;
            let (x, y) = (i % w, i / w);
            if n <= max_n {
                pts.push((x as f64 + 0.5, y as f64 + 0.5));
            }
            for (nx, ny) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if nx < w && ny < h && !seen[ny * w + nx] && img.data[ny * w + nx] < thr {
                    seen[ny * w + nx] = true;
                    stack.push(ny * w + nx);
                }
            }
        }
        if (30..=max_n).contains(&n) {
            out.push(pts);
            if out.len() >= MAX_SPOTS {
                break;
            }
        }
    }
    out
}

/// The brightness at a point (a pixel centre is at its index plus a half), by bilinear mix.
fn sample(img: &Gray, x: f64, y: f64) -> f64 {
    let (fx, fy) = (x - 0.5, y - 0.5);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let at = |ix: f64, iy: f64| {
        let cx = (ix.max(0.0) as usize).min(img.w - 1);
        let cy = (iy.max(0.0) as usize).min(img.h - 1);
        f64::from(img.data[cy * img.w + cx])
    };
    let top = at(x0, y0) * (1.0 - tx) + at(x0 + 1.0, y0) * tx;
    let bottom = at(x0, y0 + 1.0) * (1.0 - tx) + at(x0 + 1.0, y0 + 1.0) * tx;
    top * (1.0 - ty) + bottom * ty
}

/// Reads the 6 by 6 cells inside a quadrilateral. The border must be dark; the answer is the 16 bits inside, or `None`.
fn read_bits(img: &Gray, quad: &[(f64, f64); 4], thr: f64) -> Option<u16> {
    let unit = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
    let h = Homography::from_points(&unit, quad)?;
    let mut code = 0u16;
    for r in 0..CELLS {
        for c in 0..CELLS {
            let p = h.apply((
                (c as f64 + 0.5) / CELLS as f64,
                (r as f64 + 0.5) / CELLS as f64,
            ))?;
            if p.0 < 0.0 || p.1 < 0.0 || p.0 >= img.w as f64 || p.1 >= img.h as f64 {
                return None;
            }
            let v = sample(img, p.0, p.1);
            let border = r == 0 || c == 0 || r == CELLS - 1 || c == CELLS - 1;
            if border {
                if v >= thr {
                    return None;
                }
            } else if v >= thr {
                code |= 1 << ((r - 1) * BITS + (c - 1));
            }
        }
    }
    Some(code)
}

/// The markers of the picture.
pub fn find_markers(img: &Gray, dict: &Dictionary) -> Vec<FoundMarker> {
    let thr = f64::from(otsu(img));
    let mut found: Vec<FoundMarker> = Vec::new();
    for pts in spots(img, thr as u8) {
        let n = pts.len() as f64;
        let Some((mut quad, area, a, b)) = min_rect(&hull(pts)) else {
            continue;
        };
        if a.min(b) < 5.0 || a.max(b) / a.min(b) > 1.4 || n / area < 0.45 {
            continue;
        }
        // the points are pixel centres: the edge of the spot is half a pixel farther out
        let cx = quad.iter().map(|c| c.0).sum::<f64>() / 4.0;
        let cy = quad.iter().map(|c| c.1).sum::<f64>() / 4.0;
        for q in &mut quad {
            let (dx, dy) = (q.0 - cx, q.1 - cy);
            let d = (dx * dx + dy * dy).sqrt().max(1e-9);
            *q = (
                q.0 + 0.5 * dx / d * std::f64::consts::SQRT_2,
                q.1 + 0.5 * dy / d * std::f64::consts::SQRT_2,
            );
        }
        let Some(bits) = read_bits(img, &quad, thr) else {
            continue;
        };
        let Some((id, k)) = dict.identify(bits) else {
            continue;
        };
        if found.iter().any(|m| m.id == id) {
            continue;
        }
        let m = (4 - usize::from(k)) % 4;
        found.push(FoundMarker {
            id,
            quad: [
                quad[m],
                quad[(m + 1) % 4],
                quad[(m + 2) % 4],
                quad[(m + 3) % 4],
            ],
        });
    }
    found.sort_by_key(|m| m.id);
    found
}

/// Moves a point to the corner near it: the point for which the gradients at the pixels around it are at right angles to the lines to it. `None` when the edges there do not cross.
fn refine(img: &Gray, start: (f64, f64), radius: f64) -> Option<((f64, f64), f64)> {
    let mut q = start;
    let r = radius.ceil() as i64;
    let mut quality = 0.0;
    for _ in 0..12 {
        let (cx, cy) = (q.0.floor() as i64, q.1.floor() as i64);
        let (mut a, mut b, mut c, mut bx, mut by) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for dy in -r..=r {
            for dx in -r..=r {
                let (x, y) = (cx + dx, cy + dy);
                if x < 1 || y < 1 || x >= img.w as i64 - 1 || y >= img.h as i64 - 1 {
                    return None;
                }
                let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                let d2 = (px - q.0).powi(2) + (py - q.1).powi(2);
                if d2 > radius * radius {
                    continue;
                }
                let w = (-d2 / (2.0 * (radius / 2.0).powi(2))).exp();
                let g = |ix: i64, iy: i64| f64::from(img.data[iy as usize * img.w + ix as usize]);
                let gx = (g(x + 1, y) - g(x - 1, y)) / 2.0;
                let gy = (g(x, y + 1) - g(x, y - 1)) / 2.0;
                a += w * gx * gx;
                b += w * gx * gy;
                c += w * gy * gy;
                bx += w * (gx * gx * px + gx * gy * py);
                by += w * (gx * gy * px + gy * gy * py);
            }
        }
        let det = a * c - b * b;
        let trace = a + c;
        if trace < 1e-6 || det <= 1e-9 * trace * trace {
            return None;
        }
        let next = ((c * bx - b * by) / det, (a * by - b * bx) / det);
        // the smaller eigenvalue over the trace: 0 for one straight edge, 0.5 at best
        let disc = ((a - c) * (a - c) / 4.0 + b * b).sqrt();
        quality = ((trace / 2.0 - disc) / trace * 2.0).clamp(0.0, 1.0);
        let moved = ((next.0 - q.0).powi(2) + (next.1 - q.1).powi(2)).sqrt();
        q = next;
        if !q.0.is_finite()
            || !q.1.is_finite()
            || (q.0 - start.0).hypot(q.1 - start.1) > radius * 2.5
        {
            return None;
        }
        if moved < 0.005 {
            break;
        }
    }
    Some((q, quality))
}

/// The board in a picture: its markers, and the inner corners of the chessboard that the markers say should be there and that really are corners. A picture with no marker is
/// `calib_few_corners`; markers that do not fit one map are `calib_ambiguous`.
pub fn detect(img: &Gray, spec: &BoardSpec, dict: &Dictionary) -> Result<Detection, CapError> {
    spec.check()
        .map_err(|e| CapError::new(Code::CalibFewCorners, e))?;
    let markers = find_markers(img, dict);
    if markers.is_empty() {
        return Err(CapError::with_key(
            Code::CalibFewCorners,
            "calib_min_corners",
            "no marker of the board was read in the picture",
        ));
    }
    let mut board_pts = Vec::new();
    let mut px_pts = Vec::new();
    for m in &markers {
        let Some(mm) = spec.marker_corners_mm(m.id) else {
            continue;
        };
        board_pts.extend_from_slice(&mm);
        px_pts.extend_from_slice(&m.quad);
    }
    let h = fit_homography(&board_pts, &px_pts).ok_or_else(|| {
        CapError::new(
            Code::CalibAmbiguous,
            "the corners of the markers do not make a map",
        )
    })?;
    let side = {
        let a = spec.marker_corners_mm(markers[0].id).map(|c| c[0]);
        let p = a.and_then(|a| h.apply(a));
        let q = a.and_then(|a| h.apply((a.0 + spec.square_mm, a.1)));
        match (p, q) {
            (Some(p), Some(q)) => ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt(),
            _ => 0.0,
        }
    };
    let rms = (board_pts
        .iter()
        .zip(&px_pts)
        .map(|(m, p)| {
            h.apply(*m)
                .map_or(f64::INFINITY, |q| (q.0 - p.0).powi(2) + (q.1 - p.1).powi(2))
        })
        .sum::<f64>()
        / board_pts.len() as f64)
        .sqrt();
    if markers.len() >= 2 && rms > 0.25 * side {
        return Err(CapError::new(
            Code::CalibAmbiguous,
            format!("the markers leave {rms:.1} px on a map with a square of {side:.1} px"),
        ));
    }
    if side < 8.0 {
        return Err(CapError::with_key(
            Code::CalibFewCorners,
            "calib_min_corners",
            format!("a square of the board is {side:.1} px: too small to find its corners"),
        ));
    }
    let radius = (0.16 * side).max(3.0);
    let mut corners: Vec<Corner> = Vec::new();
    let mut map = h;
    for pass in 0..2 {
        corners.clear();
        for j in 1..spec.rows {
            for i in 1..spec.cols {
                let mm = spec.corner_mm(i, j);
                let Some(guess) = map.apply(mm) else {
                    continue;
                };
                if let Some((px, quality)) = refine(img, guess, radius) {
                    if quality >= 0.15 {
                        corners.push(Corner {
                            id: (i, j),
                            px,
                            mm,
                            quality,
                        });
                    }
                }
            }
        }
        if pass == 0 && corners.len() >= 6 {
            let from: Vec<(f64, f64)> = corners.iter().map(|c| c.mm).collect();
            let to: Vec<(f64, f64)> = corners.iter().map(|c| c.px).collect();
            if let Some(better) = fit_homography(&from, &to) {
                map = better;
            }
        }
    }
    Ok(Detection {
        corners,
        markers,
        marker_rms_px: rms,
        square_px: side,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::charuco::{render, view_flat, view_tilted};

    fn scene(deg: f64, noise: u8, seed: u64) -> (BoardSpec, Dictionary, Gray, Homography) {
        let spec = BoardSpec::default();
        let dict = Dictionary::for_board(&spec).unwrap();
        let view = view_flat(&spec, 7.0, deg, 160.0, 120.0);
        let img = render(&spec, &dict, &view, 320, 240, noise, seed).unwrap();
        (spec, dict, img, view)
    }

    fn worst(det: &Detection, truth: &Homography) -> f64 {
        det.corners
            .iter()
            .map(|c| {
                let t = truth.apply(c.mm).unwrap();
                (t.0 - c.px.0).hypot(t.1 - c.px.1)
            })
            .fold(0.0, f64::max)
    }

    #[test]
    fn all_the_markers_of_a_flat_view_are_read_with_their_numbers() {
        let (spec, dict, img, _) = scene(0.0, 0, 1);
        let found = find_markers(&img, &dict);
        assert_eq!(found.len(), spec.marker_count(), "{found:?}");
        for (k, m) in found.iter().enumerate() {
            assert_eq!(m.id, k);
        }
    }

    #[test]
    fn the_corners_are_found_to_a_fraction_of_a_pixel_whatever_the_turn_of_the_board() {
        for deg in [0.0, 7.0, 30.0, 45.0, 90.0, 133.0, 180.0, 271.0, -20.0] {
            let (spec, dict, img, view) = scene(deg, 0, 2);
            let det = detect(&img, &spec, &dict).unwrap_or_else(|e| panic!("{deg}: {e}"));
            assert!(
                det.corners.len() >= 30,
                "{deg}: {} corners",
                det.corners.len()
            );
            let w = worst(&det, &view);
            assert!(w < 0.35, "{deg} degrees: the worst corner is {w} px off");
        }
    }

    #[test]
    fn noise_in_the_picture_costs_precision_but_not_the_corners() {
        let (spec, dict, img, view) = scene(12.0, 6, 3);
        let det = detect(&img, &spec, &dict).unwrap();
        assert!(det.corners.len() >= 30);
        assert!(worst(&det, &view) < 0.6, "{}", worst(&det, &view));
    }

    #[test]
    fn a_view_with_perspective_is_found_too() {
        let spec = BoardSpec::default();
        let dict = Dictionary::for_board(&spec).unwrap();
        let view = view_tilted(&spec, 7.0, 10.0, 160.0, 120.0, 0.12).unwrap();
        let img = render(&spec, &dict, &view, 320, 240, 0, 4).unwrap();
        let det = detect(&img, &spec, &dict).unwrap();
        assert!(det.corners.len() >= 28, "{}", det.corners.len());
        assert!(worst(&det, &view) < 0.5, "{}", worst(&det, &view));
    }

    #[test]
    fn a_board_only_partly_in_the_picture_gives_the_corners_that_are_there() {
        let spec = BoardSpec::default();
        let dict = Dictionary::for_board(&spec).unwrap();
        let view = view_flat(&spec, 7.0, 5.0, 250.0, 150.0);
        let img = render(&spec, &dict, &view, 320, 240, 0, 5).unwrap();
        let det = detect(&img, &spec, &dict).unwrap();
        assert!(!det.corners.is_empty() && det.corners.len() < spec.corner_count());
        assert!(worst(&det, &view) < 0.4);
        for c in &det.corners {
            assert!(c.px.0 > 0.0 && c.px.0 < 320.0 && c.px.1 > 0.0 && c.px.1 < 240.0);
        }
    }

    #[test]
    fn a_picture_without_the_board_has_no_markers_and_says_so() {
        let spec = BoardSpec::default();
        let dict = Dictionary::for_board(&spec).unwrap();
        for img in [
            Gray::blank(200, 200, 128).unwrap(),
            Gray::blank(200, 200, 0).unwrap(),
        ] {
            let e = detect(&img, &spec, &dict).unwrap_err();
            assert_eq!(e.code, Code::CalibFewCorners);
            assert_eq!(e.key, Some("calib_min_corners"));
        }
        let mut noise = Gray::blank(200, 200, 0).unwrap();
        let mut rng = crate::sim::SimRng::new(11);
        noise
            .data
            .iter_mut()
            .for_each(|v| *v = rng.below(256) as u8);
        assert!(detect(&noise, &spec, &dict).is_err());
    }

    #[test]
    fn a_dictionary_of_another_seed_reads_no_marker() {
        let (spec, _, img, _) = scene(0.0, 0, 1);
        let other = Dictionary::new(77, spec.marker_count()).unwrap();
        assert!(find_markers(&img, &other).len() < spec.marker_count() / 2);
    }

    #[test]
    fn a_picture_too_small_for_the_squares_is_refused_with_the_size_in_the_message() {
        let spec = BoardSpec::default();
        let dict = Dictionary::for_board(&spec).unwrap();
        let view = view_flat(&spec, 1.5, 0.0, 40.0, 30.0);
        let img = render(&spec, &dict, &view, 80, 60, 0, 6).unwrap();
        assert!(detect(&img, &spec, &dict).is_err());
    }

    #[test]
    fn the_hull_and_the_smallest_rectangle_of_a_turned_square() {
        let (s, c) = 25f64.to_radians().sin_cos();
        let pts: Vec<(f64, f64)> = (0..20)
            .flat_map(|i| (0..20).map(move |j| (f64::from(i), f64::from(j))))
            .map(|(x, y)| (50.0 + c * x - s * y, 40.0 + s * x + c * y))
            .collect();
        let h = hull(pts);
        let (quad, area, a, b) = min_rect(&h).unwrap();
        assert!((area - 361.0).abs() < 1e-6 && (a - 19.0).abs() < 1e-6 && (b - 19.0).abs() < 1e-6);
        // clockwise on the screen: the turns between the sides are all the same way
        let signs: Vec<f64> = (0..4)
            .map(|k| cross(quad[k], quad[(k + 1) % 4], quad[(k + 2) % 4]).signum())
            .collect();
        assert!(signs.iter().all(|&v| v == signs[0]));
        assert!(min_rect(&[(0.0, 0.0)]).is_none());
    }

    #[test]
    fn refining_a_point_on_a_straight_edge_or_on_flat_ground_gives_nothing() {
        let flat = Gray::blank(60, 60, 100).unwrap();
        assert!(refine(&flat, (30.0, 30.0), 5.0).is_none());
        let mut edge = Gray::blank(60, 60, 30).unwrap();
        for y in 0..60 {
            for x in 30..60 {
                edge.data[y * 60 + x] = 220;
            }
        }
        assert!(refine(&edge, (30.0, 30.0), 5.0).is_none());
    }
}
