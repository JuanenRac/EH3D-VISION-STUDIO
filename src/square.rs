//! Square marks and cross marks turned by any angle. The spots of the picture are cut as for the other marks; for each spot the direction of its four-fold symmetry is worked out from the
//! fourth moment of its pixels (a square and a cross both repeat every 90 degrees, so that is all there is to find), and the shape is then judged in that frame: a square fills its box and
//! its box is square, a cross has two arms of one width, the middle bands full and the four corners of its box empty. A disc, a ring, a rectangle that is not a square and a spot cut by
//! the edge of the picture are left out. The centre of a mark is the centre of its pixels, as good as the centre of a disc.

use crate::fiducial::{otsu, Polarity};
use crate::image::Gray;
use crate::measure::{blobs, Blob};

/// What is asked of the search for squares and turned crosses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeSettings {
    pub polarity: Polarity,
    /// `None` is the threshold of Otsu.
    pub threshold: Option<u8>,
    /// The range of the side of a mark (the side of the box of a cross), in pixels.
    pub min_size: f64,
    pub max_size: f64,
    /// How much longer than wide the box of a mark may be (1 is a square).
    pub max_elongation: f64,
    /// The least share of its box that a square fills (0 to 1); a disc fills about 0.785, so the default leaves it out.
    pub min_fill: f64,
    /// The least score of a cross (0 to 1).
    pub min_score: f64,
    /// The most spots that are looked at; a picture with more is refused.
    pub max_spots: usize,
}

impl Default for ShapeSettings {
    fn default() -> Self {
        ShapeSettings {
            polarity: Polarity::Dark,
            threshold: None,
            min_size: 12.0,
            max_size: 200.0,
            max_elongation: 1.2,
            min_fill: 0.88,
            min_score: 0.85,
            max_spots: 1000,
        }
    }
}

/// A square mark.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Square {
    pub x: f64,
    pub y: f64,
    /// The side of the square in pixels.
    pub side: f64,
    /// The turn of its sides, counter-clockwise as the machine sees it, from -45 (not included) to 45 degrees.
    pub angle_deg: f64,
    /// 0 to 1, how much of its box it fills, lowered when the box is not square.
    pub score: f64,
}

/// A cross mark turned by any angle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurnedCross {
    pub x: f64,
    pub y: f64,
    /// The side of the box that holds the cross.
    pub size: f64,
    /// The width of an arm, worked out from the area.
    pub arm: f64,
    /// The turn of its arms, counter-clockwise as the machine sees it, from -45 (not included) to 45 degrees.
    pub angle_deg: f64,
    pub score: f64,
}

/// A spot with all its pixels, taken from the centre of the spot.
struct Spot {
    blob: Blob,
    /// The pixels of the spot, as the distance from its centre.
    pixels: Vec<(f64, f64)>,
}

fn check(s: &ShapeSettings) -> Result<(), String> {
    let ok = s.min_size.is_finite()
        && s.max_size.is_finite()
        && s.min_size >= 7.0
        && s.max_size >= s.min_size
        && s.max_elongation.is_finite()
        && s.max_elongation >= 1.0
        && s.min_fill.is_finite()
        && s.min_fill > 0.0
        && s.min_fill <= 1.0
        && s.min_score.is_finite()
        && s.min_score > 0.0
        && s.min_score <= 1.0
        && s.max_spots > 0;
    if ok {
        Ok(())
    } else {
        Err("the settings of the search for squares and crosses make no sense (a size from 7 up, the largest not below the smallest, an elongation of 1 or more, a fill and a score above 0 and at most 1)".into())
    }
}

/// The pixels of the spot, reached from its centre. `None` when the centre is not part of the spot (a ring) or the spot is not the one that was counted.
fn pixels_of(img: &Gray, on: &dyn Fn(u8) -> bool, b: &Blob) -> Option<Vec<(f64, f64)>> {
    let (bw, bh) = (b.x1 - b.x0 + 1, b.y1 - b.y0 + 1);
    let (sx, sy) = (b.cx.round() as usize, b.cy.round() as usize);
    if sx < b.x0 || sx > b.x1 || sy < b.y0 || sy > b.y1 || !on(img.data[sy * img.w + sx]) {
        return None;
    }
    let mut seen = vec![false; bw * bh];
    seen[(sy - b.y0) * bw + (sx - b.x0)] = true;
    let mut stack = vec![(sx, sy)];
    let mut out = Vec::with_capacity(b.n as usize);
    while let Some((x, y)) = stack.pop() {
        out.push((x as f64 - b.cx, y as f64 - b.cy));
        // the spot does not touch the edge of the picture, so x and y are above 0
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if nx < b.x0 || nx > b.x1 || ny < b.y0 || ny > b.y1 {
                continue;
            }
            let k = (ny - b.y0) * bw + (nx - b.x0);
            if !seen[k] && on(img.data[ny * img.w + nx]) {
                seen[k] = true;
                stack.push((nx, ny));
            }
        }
    }
    (out.len() as u64 == b.n).then_some(out)
}

/// The spots of the picture that can be marks: not cut by the edge, of a size in range and at least `min_n` pixels.
fn spots(img: &Gray, s: &ShapeSettings, min_n: u64) -> Result<Vec<Spot>, String> {
    check(s)?;
    if img.data.iter().all(|&v| v == img.data[0]) {
        return Ok(Vec::new());
    }
    let t = s.threshold.unwrap_or_else(|| otsu(img));
    let polarity = s.polarity;
    let on = move |v: u8| match polarity {
        Polarity::Dark => v <= t,
        Polarity::Light => v > t,
    };
    let max_n = (s.max_size * s.max_size).ceil() as u64;
    let mut out = Vec::new();
    for b in blobs(img, &on, min_n, max_n, s.max_spots)? {
        if b.x0 == 0 || b.y0 == 0 || b.x1 + 1 >= img.w || b.y1 + 1 >= img.h {
            continue;
        }
        // the second moments of a square or a cross are the same in every direction; a spot that is longer one way is not a mark
        let half = (b.mxx + b.myy) / 2.0;
        let spread = (((b.mxx - b.myy) / 2.0).powi(2) + b.mxy.powi(2)).sqrt();
        let (l1, l2) = (half + spread, half - spread);
        if l2 <= 0.0 || l1 / l2 > s.max_elongation * s.max_elongation {
            continue;
        }
        if let Some(pixels) = pixels_of(img, &on, &b) {
            out.push(Spot { blob: b, pixels });
        }
    }
    Ok(out)
}

/// The direction, in radians in the frame of the picture (y down), in which the pixels of a shape that repeats every 90 degrees stand out most (the arms of a cross; for a square the
/// corners are 45 degrees from it).
fn direction(pixels: &[(f64, f64)]) -> f64 {
    let (mut re, mut im) = (0.0f64, 0.0f64);
    for p in pixels {
        let (zr, zi) = (p.0 * p.0 - p.1 * p.1, 2.0 * p.0 * p.1);
        re += zr * zr - zi * zi;
        im += 2.0 * zr * zi;
    }
    im.atan2(re) / 4.0
}

/// The length of the pixels along the direction `angle` and across it, in pixels (each pixel counts one).
fn extent(pixels: &[(f64, f64)], angle: f64) -> (f64, f64) {
    let (sin, cos) = angle.sin_cos();
    let (mut u0, mut u1) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut v0, mut v1) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in pixels {
        let (u, v) = (p.0 * cos + p.1 * sin, -p.0 * sin + p.1 * cos);
        u0 = u0.min(u);
        u1 = u1.max(u);
        v0 = v0.min(v);
        v1 = v1.max(v);
    }
    (u1 - u0 + 1.0, v1 - v0 + 1.0)
}

/// The turn as the machine sees it (y up), brought to -45 (not included) to 45 degrees.
fn machine_degrees(image_angle: f64) -> f64 {
    let d = (-image_angle.to_degrees()).rem_euclid(90.0);
    if d > 45.0 {
        d - 90.0
    } else {
        d
    }
}

fn by_score<T>(items: &mut [T], score: impl Fn(&T) -> f64) {
    items.sort_by(|a, b| {
        score(b)
            .partial_cmp(&score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// The square marks of a picture, the best first. The sides may be turned by any angle.
pub fn find_squares(img: &Gray, s: &ShapeSettings) -> Result<Vec<Square>, String> {
    let min_n = (s.min_size * s.min_size).floor() as u64;
    let mut found = Vec::new();
    for spot in spots(img, s, min_n.max(1))? {
        let n = spot.blob.n as f64;
        let d = direction(&spot.pixels);
        // the sides are along the direction of the arms or 45 degrees from it, whichever fills its box better
        let mut best: Option<(f64, f64, f64, f64)> = None;
        for a in [d, d + std::f64::consts::FRAC_PI_4] {
            let (w, h) = extent(&spot.pixels, a);
            let fill = n / (w * h);
            if best.is_none_or(|b| fill > b.0) {
                best = Some((fill, w, h, a));
            }
        }
        let Some((fill, w, h, a)) = best else {
            continue;
        };
        let side = (w + h) / 2.0;
        let elongation = w.max(h) / w.min(h);
        if side < s.min_size || side > s.max_size || elongation > s.max_elongation {
            continue;
        }
        if fill < s.min_fill {
            continue;
        }
        found.push(Square {
            x: spot.blob.cx,
            y: spot.blob.cy,
            side,
            angle_deg: machine_degrees(a),
            score: (fill.min(1.0) / elongation).clamp(0.0, 1.0),
        });
    }
    by_score(&mut found, |q| q.score);
    Ok(found)
}

/// How much a spot of `n` pixels looks like a cross whose arms are along `angle`: `(score, side, arm)`.
fn cross_in_frame(pixels: &[(f64, f64)], n: f64, angle: f64) -> Option<(f64, f64, f64)> {
    let (w, h) = extent(pixels, angle);
    let side = (w + h) / 2.0;
    let elongation = w.max(h) / w.min(h);
    // a cross of side L and arm a has 2 * L * a - a * a pixels
    let rest = side * side - n;
    if rest < 0.0 {
        return None;
    }
    let arm = side - rest.sqrt();
    if arm < 2.0 || arm > 0.48 * side {
        return None;
    }
    let (sin, cos) = angle.sin_cos();
    // the bands of the arms, half a pixel wider than the arm, and the four corners of the box, a pixel away from the arms
    let band = arm / 2.0 + 0.5;
    let corner = (side - arm) / 2.0 - 1.5;
    if corner < 1.0 {
        return None;
    }
    let edge = side / 2.0 - corner;
    // the tip of each arm, a little short of its end and as wide as the arm: an arm is as wide there as anywhere (a square turned by 45 degrees narrows to a point)
    let tip_hi = side / 2.0 - 2.0;
    let tip_lo = (tip_hi - arm / 2.0 - 0.5).max(0.0);
    let tip_width = (arm / 2.0).max(0.5);
    let (mut inside, mut filled, mut tips) = (0usize, 0usize, 0usize);
    for p in pixels {
        let (u, v) = (
            (p.0 * cos + p.1 * sin).abs(),
            (-p.0 * sin + p.1 * cos).abs(),
        );
        if u <= band || v <= band {
            inside += 1;
        }
        if u >= edge && v >= edge {
            filled += 1;
        }
        if (u >= tip_lo && u <= tip_hi && v <= tip_width)
            || (v >= tip_lo && v <= tip_hi && u <= tip_width)
        {
            tips += 1;
        }
    }
    // a cross has every pixel in its bands and nothing in its corners; a disc of the same area has a tenth of its pixels outside, so a share outside counts five times
    let in_band = 1.0 - 5.0 * (1.0 - inside as f64 / n);
    let empty = 1.0 - 5.0 * filled as f64 / (4.0 * corner * corner);
    let tip_area = 8.0 * (tip_hi - tip_lo) * tip_width;
    // drawn arms are thin and the pixels fall on them unevenly: seven tenths of the tip is full
    let tip = (tips as f64 / (0.7 * tip_area)).min(1.0);
    Some((
        in_band.min(empty).min(tip).clamp(0.0, 1.0) / elongation,
        side,
        arm,
    ))
}

/// The cross marks of a picture whose arms are turned by any angle, the best first. A cross whose arms are along the axes of the picture is found too (see also `cross::find_crosses`).
pub fn find_turned_crosses(img: &Gray, s: &ShapeSettings) -> Result<Vec<TurnedCross>, String> {
    let min_n = (2.0 * s.min_size - 1.0).floor() as u64;
    let mut found = Vec::new();
    for spot in spots(img, s, min_n.max(1))? {
        let n = spot.blob.n as f64;
        let d = direction(&spot.pixels);
        // a thin cross stands out along its arms and a fat one along its corners: both frames are tried
        let mut best: Option<(f64, f64, f64, f64)> = None;
        for a in [d, d + std::f64::consts::FRAC_PI_4] {
            if let Some((score, side, arm)) = cross_in_frame(&spot.pixels, n, a) {
                if best.is_none_or(|b| score > b.0) {
                    best = Some((score, side, arm, a));
                }
            }
        }
        let Some((score, side, arm, a)) = best else {
            continue;
        };
        if score < s.min_score || side < s.min_size || side > s.max_size {
            continue;
        }
        found.push(TurnedCross {
            x: spot.blob.cx,
            y: spot.blob.cy,
            size: side,
            arm,
            angle_deg: machine_degrees(a),
            score,
        });
    }
    by_score(&mut found, |c| c.score);
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fiducial::{find, Settings};

    /// Paints a square (`arm_half` of `None`) or a cross of side `2 * half + 1` turned by `phi` degrees counter-clockwise (y up), centred at the pixel (cx, cy).
    fn paint(
        g: &mut Gray,
        cx: usize,
        cy: usize,
        half: f64,
        arm_half: Option<f64>,
        phi: f64,
        value: u8,
    ) {
        let (sin, cos) = phi.to_radians().sin_cos();
        let reach = (half * 1.5) as isize + 2;
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let (x, y) = (dx as f64, -dy as f64);
                let (u, v) = ((x * cos + y * sin).abs(), (-x * sin + y * cos).abs());
                let inside = match arm_half {
                    None => u <= half && v <= half,
                    Some(a) => (u <= half && v <= a) || (v <= half && u <= a),
                };
                let (px, py) = (cx as isize + dx, cy as isize + dy);
                if inside && px >= 0 && py >= 0 && (px as usize) < g.w && (py as usize) < g.h {
                    g.data[py as usize * g.w + px as usize] = value;
                }
            }
        }
    }

    /// The difference between two turns, as a turn of the square (they repeat every 90 degrees).
    fn off(a: f64, b: f64) -> f64 {
        let d = (a - b).rem_euclid(90.0);
        d.min(90.0 - d)
    }

    #[test]
    fn squares_are_found_at_any_turn_with_centre_side_and_angle() {
        for phi in [0.0, 12.0, 30.0, 45.0, 63.0, 80.0] {
            let mut g = Gray::blank(240, 140, 220).unwrap();
            paint(&mut g, 60, 60, 12.0, None, phi, 30);
            paint(&mut g, 170, 70, 18.0, None, 0.0, 30);
            let found = find_squares(&g, &ShapeSettings::default()).unwrap();
            assert_eq!(found.len(), 2, "turn {phi}: {found:?}");
            let turned = found.iter().find(|q| q.x < 100.0).unwrap();
            assert!(
                (turned.x - 60.0).abs() < 0.2 && (turned.y - 60.0).abs() < 0.2,
                "{turned:?}"
            );
            assert!((turned.side - 25.0).abs() < 2.0, "turn {phi}: {turned:?}");
            assert!(off(turned.angle_deg, phi) < 1.5, "turn {phi}: {turned:?}");
            assert!(turned.angle_deg > -45.0 && turned.angle_deg <= 45.0);
            assert!(turned.score > 0.85, "{turned:?}");
            let plain = found.iter().find(|q| q.x > 100.0).unwrap();
            assert!(
                plain.angle_deg.abs() < 0.5 && (plain.side - 37.0).abs() < 1.0,
                "{plain:?}"
            );
        }
    }

    #[test]
    fn the_turn_of_a_square_is_counter_clockwise_as_the_machine_sees_it() {
        let mut g = Gray::blank(120, 120, 220).unwrap();
        paint(&mut g, 60, 60, 15.0, None, 20.0, 30);
        let q = find_squares(&g, &ShapeSettings::default()).unwrap();
        assert!((q[0].angle_deg - 20.0).abs() < 1.5, "{q:?}");
        let mut h = Gray::blank(120, 120, 220).unwrap();
        paint(&mut h, 60, 60, 15.0, None, -20.0, 30);
        let q = find_squares(&h, &ShapeSettings::default()).unwrap();
        assert!((q[0].angle_deg + 20.0).abs() < 1.5, "{q:?}");
    }

    #[test]
    fn crosses_turned_by_any_angle_are_found_with_their_arm() {
        for phi in [0.0, 15.0, 30.0, 45.0, 70.0] {
            for arm_half in [1.5, 4.0] {
                let mut g = Gray::blank(240, 140, 220).unwrap();
                paint(&mut g, 70, 70, 16.0, Some(arm_half), phi, 30);
                let found = find_turned_crosses(&g, &ShapeSettings::default()).unwrap();
                assert_eq!(found.len(), 1, "turn {phi}, arm {arm_half}: {found:?}");
                let c = &found[0];
                assert!(
                    (c.x - 70.0).abs() < 0.3 && (c.y - 70.0).abs() < 0.3,
                    "{c:?}"
                );
                assert!(
                    off(c.angle_deg, phi) < 2.0,
                    "turn {phi}, arm {arm_half}: {c:?}"
                );
                assert!((c.size - 33.0).abs() < 2.5, "{c:?}");
                assert!((c.arm - (2.0 * arm_half + 1.0)).abs() < 1.5, "{c:?}");
                assert!(c.score > 0.9, "{c:?}");
            }
        }
    }

    #[test]
    fn light_marks_are_found_with_the_light_polarity_only() {
        let mut g = Gray::blank(200, 120, 30).unwrap();
        paint(&mut g, 60, 60, 14.0, None, 25.0, 230);
        paint(&mut g, 140, 60, 14.0, Some(2.0), 25.0, 230);
        let light = ShapeSettings {
            polarity: Polarity::Light,
            ..ShapeSettings::default()
        };
        assert_eq!(find_squares(&g, &light).unwrap().len(), 1);
        assert_eq!(find_turned_crosses(&g, &light).unwrap().len(), 1);
        assert!(find_squares(&g, &ShapeSettings::default())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn a_square_is_not_a_cross_and_a_cross_is_not_a_square() {
        let mut g = Gray::blank(240, 120, 220).unwrap();
        paint(&mut g, 60, 60, 15.0, None, 33.0, 30);
        paint(&mut g, 170, 60, 15.0, Some(2.0), 33.0, 30);
        let squares = find_squares(&g, &ShapeSettings::default()).unwrap();
        let crosses = find_turned_crosses(&g, &ShapeSettings::default()).unwrap();
        assert_eq!((squares.len(), crosses.len()), (1, 1));
        assert!(squares[0].x < 100.0 && crosses[0].x > 100.0);
    }

    #[test]
    fn discs_rings_rectangles_and_cut_marks_are_not_marks() {
        let mut g = Gray::blank(320, 200, 220).unwrap();
        g.disc(40.0, 40.0, 16.0, 30);
        // a ring
        g.disc(120.0, 40.0, 16.0, 30);
        g.disc(120.0, 40.0, 10.0, 220);
        // a rectangle twice as long as wide, turned
        let (sin, cos) = 25f64.to_radians().sin_cos();
        for y in 0..200usize {
            for x in 180..260usize {
                let (dx, dy) = (x as f64 - 220.0, 60.0 - y as f64);
                let (u, v) = ((dx * cos + dy * sin).abs(), (-dx * sin + dy * cos).abs());
                if u <= 24.0 && v <= 12.0 {
                    g.data[y * 320 + x] = 30;
                }
            }
        }
        // a square and a cross that touch the edge of the picture
        paint(&mut g, 8, 150, 12.0, None, 0.0, 30);
        paint(&mut g, 150, 188, 12.0, Some(2.0), 0.0, 30);
        let squares = find_squares(&g, &ShapeSettings::default()).unwrap();
        let crosses = find_turned_crosses(&g, &ShapeSettings::default()).unwrap();
        assert!(squares.is_empty(), "{squares:?}");
        assert!(crosses.is_empty(), "{crosses:?}");
    }

    #[test]
    fn discs_of_every_size_are_neither_squares_nor_crosses() {
        for (i, radius) in [7.0, 9.5, 13.0, 21.0, 34.0, 60.0].iter().enumerate() {
            let mut g = Gray::blank(240, 240, 220).unwrap();
            g.disc(120.0 + 0.3 * i as f64, 120.0 + 0.45 * i as f64, *radius, 30);
            let s = ShapeSettings {
                min_size: 7.0,
                ..ShapeSettings::default()
            };
            assert!(find_squares(&g, &s).unwrap().is_empty(), "radius {radius}");
            assert!(
                find_turned_crosses(&g, &s).unwrap().is_empty(),
                "radius {radius}"
            );
        }
    }

    #[test]
    fn a_square_is_not_a_round_mark_and_a_disc_is_not_a_square() {
        let mut g = Gray::blank(200, 100, 220).unwrap();
        paint(&mut g, 50, 50, 15.0, None, 18.0, 30);
        g.disc(140.0, 50.0, 15.0, 30);
        let squares = find_squares(&g, &ShapeSettings::default()).unwrap();
        let discs = find(&g, &Settings::default()).unwrap();
        assert_eq!((squares.len(), discs.len()), (1, 1));
        assert!(squares[0].x < 100.0 && discs[0].x > 100.0);
    }

    #[test]
    fn the_tolerances_can_be_tightened_and_the_settings_are_checked() {
        let mut g = Gray::blank(200, 120, 220).unwrap();
        paint(&mut g, 60, 60, 14.0, None, 25.0, 30);
        // a rotated square never fills its box to the last pixel: a fill of 1 leaves it out
        let strict = ShapeSettings {
            min_fill: 1.0,
            ..ShapeSettings::default()
        };
        assert!(find_squares(&g, &strict).unwrap().is_empty());
        let big = ShapeSettings {
            min_size: 40.0,
            ..ShapeSettings::default()
        };
        assert!(find_squares(&g, &big).unwrap().is_empty());
        for bad in [
            ShapeSettings {
                min_size: 3.0,
                ..ShapeSettings::default()
            },
            ShapeSettings {
                max_size: 5.0,
                ..ShapeSettings::default()
            },
            ShapeSettings {
                max_elongation: 0.5,
                ..ShapeSettings::default()
            },
            ShapeSettings {
                min_fill: 0.0,
                ..ShapeSettings::default()
            },
            ShapeSettings {
                min_score: 1.5,
                ..ShapeSettings::default()
            },
            ShapeSettings {
                max_spots: 0,
                ..ShapeSettings::default()
            },
            ShapeSettings {
                min_size: f64::NAN,
                ..ShapeSettings::default()
            },
        ] {
            assert!(find_squares(&g, &bad).is_err(), "{bad:?}");
            assert!(find_turned_crosses(&g, &bad).is_err(), "{bad:?}");
        }
        let flat = Gray::blank(50, 50, 100).unwrap();
        assert!(find_squares(&flat, &ShapeSettings::default())
            .unwrap()
            .is_empty());
        assert!(find_turned_crosses(&flat, &ShapeSettings::default())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn too_many_spots_of_the_size_of_a_mark_are_refused() {
        let mut g = Gray::blank(300, 60, 220).unwrap();
        for i in 0..6 {
            paint(&mut g, 30 + i * 45, 30, 12.0, None, 10.0, 30);
        }
        let s = ShapeSettings {
            max_spots: 4,
            ..ShapeSettings::default()
        };
        assert!(find_squares(&g, &s)
            .unwrap_err()
            .starts_with("too_many_parts"));
        assert_eq!(
            find_squares(&g, &ShapeSettings::default()).unwrap().len(),
            6
        );
    }
}
