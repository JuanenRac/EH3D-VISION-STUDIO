//! Correction of perspective: a camera that does not look straight down at a board (or at a tray, or at a target) sees a rectangle as a four-sided shape with no right angles. From the four
//! corners of the rectangle in the picture the projective transform (a homography) that puts them on a true rectangle is worked out, and the picture is drawn again as it would look
//! from straight above, so that distances in it are the same everywhere and the other searches can be used on it.

use crate::image::Gray;

/// The code of the refusal of four corners that are not a convex four-sided shape.
pub const BAD_CORNERS: &str = "bad_corners";
/// The code of the refusal of a size of the result that is not allowed.
pub const BAD_SIZE: &str = "bad_size";
/// The smallest and the largest side of the picture that is made, in pixels (the largest is a key of the configuration file).
pub const MIN_SIDE: usize = 2;
pub const MAX_SIDE_LIMIT: usize = 16_384;

/// A projective transform of the plane: `(x, y)` goes to `((a x + b y + c) / w, (d x + e y + f) / w)` with `w = g x + h y + 1`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Homography(pub [f64; 9]);

/// The solution of eight equations with eight unknowns (Gauss, with the largest pivot of the column); `None` when the system has no single solution.
fn solve8(mut m: [[f64; 9]; 8]) -> Option<[f64; 8]> {
    for col in 0..8 {
        let best = (col..8).max_by(|&a, &b| {
            m[a][col]
                .abs()
                .partial_cmp(&m[b][col].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        let size = m[best][col].abs();
        if size.is_nan() || size <= 1e-12 {
            return None;
        }
        m.swap(col, best);
        let pivot = m[col];
        for row in m.iter_mut().skip(col + 1) {
            let f = row[col] / pivot[col];
            for (k, cell) in row.iter_mut().enumerate().skip(col) {
                *cell -= f * pivot[k];
            }
        }
    }
    let mut x = [0f64; 8];
    for row in (0..8).rev() {
        let mut v = m[row][8];
        for (a, b) in m[row][row + 1..8].iter().zip(&x[row + 1..8]) {
            v -= a * b;
        }
        x[row] = v / m[row][row];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

impl Homography {
    /// The transform that takes each of the four points `from` to the point `to` of the same place in the list; `None` when no such transform is there (three of the points on a line).
    pub fn from_points(from: &[(f64, f64); 4], to: &[(f64, f64); 4]) -> Option<Homography> {
        let mut m = [[0f64; 9]; 8];
        for (k, (&(x, y), &(u, v))) in from.iter().zip(to.iter()).enumerate() {
            m[2 * k] = [x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y, u];
            m[2 * k + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y, v];
        }
        let h = solve8(m)?;
        Some(Homography([
            h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7], 1.0,
        ]))
    }

    /// Where a point goes; `None` when it goes to infinity (the point is on the line that the transform sends away).
    pub fn apply(&self, p: (f64, f64)) -> Option<(f64, f64)> {
        let h = &self.0;
        let w = h[6] * p.0 + h[7] * p.1 + h[8];
        if w.abs() < 1e-12 {
            return None;
        }
        let out = (
            (h[0] * p.0 + h[1] * p.1 + h[2]) / w,
            (h[3] * p.0 + h[4] * p.1 + h[5]) / w,
        );
        (out.0.is_finite() && out.1.is_finite()).then_some(out)
    }

    /// The transform that does the opposite; `None` when it cannot be undone.
    pub fn inverse(&self) -> Option<Homography> {
        let h = &self.0;
        let det = h[0] * (h[4] * h[8] - h[5] * h[7]) - h[1] * (h[3] * h[8] - h[5] * h[6])
            + h[2] * (h[3] * h[7] - h[4] * h[6]);
        if det.is_nan() || det.abs() <= 1e-14 {
            return None;
        }
        let adj = [
            h[4] * h[8] - h[5] * h[7],
            h[2] * h[7] - h[1] * h[8],
            h[1] * h[5] - h[2] * h[4],
            h[5] * h[6] - h[3] * h[8],
            h[0] * h[8] - h[2] * h[6],
            h[2] * h[3] - h[0] * h[5],
            h[3] * h[7] - h[4] * h[6],
            h[1] * h[6] - h[0] * h[7],
            h[0] * h[4] - h[1] * h[3],
        ];
        let scale = adj[8];
        if scale.is_nan() || scale.abs() <= 1e-14 {
            return None;
        }
        let mut out = [0f64; 9];
        for (o, a) in out.iter_mut().zip(adj) {
            *o = a / scale;
        }
        Some(Homography(out))
    }
}

/// Whether four corners, in the order top left, top right, bottom right, bottom left (as the picture shows them, y down), are the corners of a convex four-sided shape that is not flat:
/// every turn from one side to the next is the same way round.
pub fn convex(corners: &[(f64, f64); 4]) -> bool {
    let mut sign = 0f64;
    for k in 0..4 {
        let (a, b, c) = (corners[k], corners[(k + 1) % 4], corners[(k + 2) % 4]);
        let cross = (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0);
        if !cross.is_finite() || cross == 0.0 {
            return false;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return false;
        }
    }
    true
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// The size, in pixels, that a rectified picture gets when none is given: the mean length of each pair of opposite sides, rounded, plus one (the corners are the centres of the first and the last pixel).
pub fn natural_size(corners: &[(f64, f64); 4]) -> (usize, usize) {
    let top = distance(corners[0], corners[1]);
    let bottom = distance(corners[3], corners[2]);
    let left = distance(corners[0], corners[3]);
    let right = distance(corners[1], corners[2]);
    (
        (((top + bottom) / 2.0).round() as usize)
            .saturating_add(1)
            .max(MIN_SIDE),
        (((left + right) / 2.0).round() as usize)
            .saturating_add(1)
            .max(MIN_SIDE),
    )
}

/// The grey level of the picture at a place between pixels, by the four pixels around it; `None` outside the picture.
fn sample(img: &Gray, x: f64, y: f64) -> Option<f64> {
    if !(x >= 0.0 && y >= 0.0 && x <= (img.w - 1) as f64 && y <= (img.h - 1) as f64) {
        return None;
    }
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(img.w - 1), (y0 + 1).min(img.h - 1));
    let (fx, fy) = (x - x0 as f64, y - y0 as f64);
    let at = |xx: usize, yy: usize| f64::from(img.data[yy * img.w + xx]);
    let top = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
    let bottom = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
    Some(top * (1.0 - fy) + bottom * fy)
}

/// The picture drawn again as seen from straight above: the rectangle whose corners are `corners` (top left, top right, bottom right, bottom left, in pixels of the picture) becomes a
/// picture of `width` by `height` pixels (`None` is the natural size). A place of the result that comes from outside the picture gets `fill`. The corners must be a convex four-sided
/// shape (`bad_corners`), and no side of the result may be below 2 pixels or above `max_side` (`bad_size`).
pub fn rectify(
    img: &Gray,
    corners: &[(f64, f64); 4],
    size: Option<(usize, usize)>,
    fill: u8,
    max_side: usize,
) -> Result<Gray, String> {
    if !corners.iter().all(|c| c.0.is_finite() && c.1.is_finite()) || !convex(corners) {
        return Err(format!(
            "{BAD_CORNERS}: the four corners must be a convex four-sided shape, in the order top left, top right, bottom right, bottom left"
        ));
    }
    let (w, h) = size.unwrap_or_else(|| natural_size(corners));
    if !(MIN_SIDE..=max_side.min(MAX_SIDE_LIMIT)).contains(&w)
        || !(MIN_SIDE..=max_side.min(MAX_SIDE_LIMIT)).contains(&h)
        || w.saturating_mul(h) > crate::image::MAX_PIXELS
    {
        return Err(format!(
            "{BAD_SIZE}: the picture to make is {w} by {h} pixels; each side must be from {MIN_SIDE} to {max_side}"
        ));
    }
    let rect = [
        (0.0, 0.0),
        ((w - 1) as f64, 0.0),
        ((w - 1) as f64, (h - 1) as f64),
        (0.0, (h - 1) as f64),
    ];
    let back = Homography::from_points(&rect, corners)
        .ok_or_else(|| format!("{BAD_CORNERS}: the corners make no transform"))?;
    let mut data = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let v = back
                .apply((x as f64, y as f64))
                .and_then(|(sx, sy)| sample(img, sx, sy))
                .map_or(fill, |v| v.round().clamp(0.0, 255.0) as u8);
            data.push(v);
        }
    }
    Gray::new(w, h, data)
}

/// Four corners from the text `x1,y1,x2,y2,x3,y3,x4,y4`.
pub fn parse_corners(text: &str) -> Result<[(f64, f64); 4], String> {
    let numbers: Vec<f64> = text
        .split(',')
        .map(|t| {
            t.trim()
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("'{}' is not a number", t.trim()))
        })
        .collect::<Result<_, _>>()?;
    if numbers.len() != 8 {
        return Err(format!(
            "the corners are eight numbers (x,y of the four corners), not {}",
            numbers.len()
        ));
    }
    Ok([
        (numbers[0], numbers[1]),
        (numbers[2], numbers[3]),
        (numbers[4], numbers[5]),
        (numbers[6], numbers[7]),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: [(f64, f64); 4] = [(0.0, 0.0), (59.0, 0.0), (59.0, 39.0), (0.0, 39.0)];

    fn close(a: (f64, f64), b: (f64, f64), tol: f64) -> bool {
        (a.0 - b.0).abs() <= tol && (a.1 - b.1).abs() <= tol
    }

    #[test]
    fn four_points_go_to_four_points() {
        let to = [(12.0, 8.0), (80.0, 15.0), (70.0, 70.0), (5.0, 60.0)];
        let h = Homography::from_points(&RECT, &to).unwrap();
        for (a, b) in RECT.iter().zip(to.iter()) {
            assert!(close(h.apply(*a).unwrap(), *b, 1e-6), "{a:?}");
        }
        let back = h.inverse().unwrap();
        for p in [(10.0, 10.0), (30.5, 20.25), (59.0, 39.0)] {
            assert!(close(back.apply(h.apply(p).unwrap()).unwrap(), p, 1e-6));
        }
    }

    #[test]
    fn the_same_points_are_the_identity_and_a_shift_is_a_shift() {
        let id = Homography::from_points(&RECT, &RECT).unwrap();
        assert!(close(id.apply((12.5, 7.5)).unwrap(), (12.5, 7.5), 1e-9));
        let moved: Vec<(f64, f64)> = RECT.iter().map(|p| (p.0 + 10.0, p.1 - 4.0)).collect();
        let to = [moved[0], moved[1], moved[2], moved[3]];
        let h = Homography::from_points(&RECT, &to).unwrap();
        assert!(close(h.apply((1.0, 1.0)).unwrap(), (11.0, -3.0), 1e-9));
    }

    #[test]
    fn three_points_on_a_line_have_no_transform() {
        let line = [(0.0, 0.0), (10.0, 0.0), (20.0, 0.0), (0.0, 10.0)];
        assert!(Homography::from_points(&line, &RECT).is_none());
    }

    #[test]
    fn a_point_sent_to_infinity_has_no_place() {
        let h = Homography([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0]);
        assert!(h.apply((0.0, 5.0)).is_none());
        assert!(h.apply((2.0, 5.0)).is_some());
        assert!(Homography([0.0; 9]).inverse().is_none());
    }

    #[test]
    fn the_corners_must_be_convex_in_order() {
        assert!(convex(&RECT));
        // the same corners in the other direction are still convex
        assert!(convex(&[RECT[3], RECT[2], RECT[1], RECT[0]]));
        // a bow tie
        assert!(!convex(&[RECT[0], RECT[2], RECT[1], RECT[3]]));
        // three on a line
        assert!(!convex(&[(0.0, 0.0), (5.0, 0.0), (10.0, 0.0), (0.0, 10.0)]));
        // a dent
        assert!(!convex(&[(0.0, 0.0), (10.0, 0.0), (2.0, 2.0), (0.0, 10.0)]));
    }

    #[test]
    fn a_straight_crop_is_the_picture_cut_out() {
        let mut img = Gray::blank(80, 60, 200).unwrap();
        for y in 20..30 {
            for x in 30..40 {
                img.data[y * 80 + x] = 10;
            }
        }
        let corners = [(10.0, 5.0), (49.0, 5.0), (49.0, 34.0), (10.0, 34.0)];
        let out = rectify(&img, &corners, None, 0, 4096).unwrap();
        assert_eq!((out.w, out.h), (40, 30));
        for y in 0..out.h {
            for x in 0..out.w {
                assert_eq!(out.at(x, y), img.at(x + 10, y + 5));
            }
        }
    }

    #[test]
    fn a_picture_seen_from_the_side_is_straightened() {
        // the picture of a pattern whose grey level is 2 x + y, taken from the side: the pattern's corners are where `h` puts them
        let to = [(14.0, 6.0), (92.0, 14.0), (84.0, 70.0), (6.0, 58.0)];
        let h = Homography::from_points(&RECT, &to).unwrap();
        let back = h.inverse().unwrap();
        let (w, hh) = (100usize, 80usize);
        let mut photo = vec![0u8; w * hh];
        for y in 0..hh {
            for x in 0..w {
                if let Some((u, v)) = back.apply((x as f64, y as f64)) {
                    if (0.0..=59.0).contains(&u) && (0.0..=39.0).contains(&v) {
                        photo[y * w + x] = (2.0 * u + v).round() as u8;
                    }
                }
            }
        }
        let img = Gray::new(w, hh, photo).unwrap();
        let out = rectify(&img, &to, Some((60, 40)), 0, 4096).unwrap();
        // away from the border the pattern is back, to the rounding of the grey levels
        for y in 3..37 {
            for x in 3..57 {
                let want = 2.0 * x as f64 + y as f64;
                let got = f64::from(out.at(x, y));
                assert!(
                    (got - want).abs() <= 4.0,
                    "({x}, {y}): {got} against {want}"
                );
            }
        }
    }

    #[test]
    fn a_place_outside_the_picture_gets_the_fill() {
        let img = Gray::blank(40, 40, 100).unwrap();
        let corners = [(-20.0, -20.0), (30.0, -20.0), (30.0, 30.0), (-20.0, 30.0)];
        let out = rectify(&img, &corners, Some((50, 50)), 7, 4096).unwrap();
        assert_eq!(out.at(0, 0), 7);
        assert_eq!(out.at(49, 49), 100);
    }

    #[test]
    fn bad_corners_and_bad_sizes_are_refused() {
        let img = Gray::blank(40, 40, 100).unwrap();
        let bow = [(0.0, 0.0), (30.0, 30.0), (30.0, 0.0), (0.0, 30.0)];
        assert!(rectify(&img, &bow, None, 0, 4096)
            .unwrap_err()
            .starts_with(BAD_CORNERS));
        let nan = [(f64::NAN, 0.0), (30.0, 0.0), (30.0, 30.0), (0.0, 30.0)];
        assert!(rectify(&img, &nan, None, 0, 4096).is_err());
        let ok = [(0.0, 0.0), (30.0, 0.0), (30.0, 30.0), (0.0, 30.0)];
        for size in [(1, 10), (10, 1), (0, 0), (5000, 10)] {
            assert!(
                rectify(&img, &ok, Some(size), 0, 4096)
                    .unwrap_err()
                    .starts_with(BAD_SIZE),
                "{size:?}"
            );
        }
        assert!(rectify(&img, &ok, Some((100, 100)), 0, 64).is_err());
    }

    #[test]
    fn the_natural_size_is_the_mean_of_opposite_sides() {
        assert_eq!(natural_size(&RECT), (60, 40));
        let skew = [(0.0, 0.0), (100.0, 0.0), (90.0, 50.0), (10.0, 50.0)];
        assert_eq!(natural_size(&skew), (91, 52));
        let point = [(1.0, 1.0); 4];
        assert_eq!(natural_size(&point), (MIN_SIDE, MIN_SIDE));
    }

    #[test]
    fn the_corners_are_read_from_text() {
        let c = parse_corners("1,2, 3,4,5,6 ,7,8").unwrap();
        assert_eq!(c[3], (7.0, 8.0));
        assert!(parse_corners("1,2,3").is_err());
        assert!(parse_corners("1,2,3,4,5,6,7,x").is_err());
        assert!(parse_corners("1,2,3,4,5,6,7,inf").is_err());
        assert!(parse_corners("").is_err());
    }
}
