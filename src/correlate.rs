//! Fiducial marks found by correlation with a template: a disc of the wanted diameter is slid over the picture and, at every place, the normalised cross-correlation of the grey
//! levels under it with the disc is worked out (a number from -1 to 1 that does not change when the light is brighter or the contrast is lower). The places where it is high are the
//! marks. This finds a mark that the search by threshold loses (a mark on a board whose light is uneven, a mark that touches a track of nearly the same grey) at the cost of needing the
//! diameter, and it is slower (the work is the pixels of the picture times the pixels of the template), so a picture that would take too long is refused.

use crate::fiducial::Polarity;
use crate::image::Gray;

/// The code of the refusal of a picture smaller than the template.
pub const PICTURE_SMALL: &str = "picture_small";
/// The code of the refusal of a search that would take too long.
pub const TOO_BIG: &str = "correlate_too_big";
/// The code of the refusal of a picture with more marks than the settings allow.
pub const TOO_MANY: &str = "too_many_matches";
/// The most multiply-adds of one search (the pixels of the picture times those of the template).
pub const MAX_WORK: u64 = 3_000_000_000;
/// The smallest and the largest diameter of the template, in pixels.
pub const MIN_DIAMETER: f64 = 4.0;
pub const MAX_DIAMETER: f64 = 120.0;
/// The line of column names of the CSV file.
pub const CSV_HEADER: &str = "x_px,y_px,score";

/// A place where the picture looks like the template. The coordinates are those of the centre of the mark, in pixels, with the centre of the first pixel at (0, 0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Match {
    pub x: f64,
    pub y: f64,
    /// The correlation, from the least that is asked for to 1.
    pub score: f64,
}

/// What is asked of the search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CorrelateSettings {
    /// `Dark` looks for a dark disc on a light board and `Light` for a light disc on a dark one.
    pub polarity: Polarity,
    /// The diameter of the disc, in pixels.
    pub diameter: f64,
    /// The least correlation that counts as a mark (more than 0, at most 1).
    pub min_score: f64,
    /// The most marks in a picture; a picture with more is refused.
    pub max_matches: usize,
}

impl Default for CorrelateSettings {
    fn default() -> Self {
        CorrelateSettings {
            polarity: Polarity::Dark,
            diameter: 20.0,
            min_score: 0.8,
            max_matches: 1000,
        }
    }
}

/// Whether the settings make sense; the reason when they do not.
pub fn check(s: &CorrelateSettings) -> Result<(), String> {
    if !(s.diameter.is_finite() && (MIN_DIAMETER..=MAX_DIAMETER).contains(&s.diameter)) {
        return Err(format!(
            "the diameter of the template must be from {MIN_DIAMETER} to {MAX_DIAMETER} pixels"
        ));
    }
    if !(s.min_score > 0.0 && s.min_score <= 1.0) {
        return Err("the least score of a correlation must be more than 0 and at most 1".into());
    }
    if !(1..=100_000).contains(&s.max_matches) {
        return Err("the most marks of a correlation must be from 1 to 100000".into());
    }
    Ok(())
}

/// The template of a disc: its side (odd, so that the centre of the disc is the centre of a pixel) and the share of each pixel that the disc covers (each pixel is sampled 4 by 4).
pub fn template(diameter: f64) -> (usize, Vec<f64>) {
    let mut n = diameter.ceil() as usize + 2;
    if n.is_multiple_of(2) {
        n += 1;
    }
    let centre = (n - 1) as f64 / 2.0;
    let r = diameter / 2.0;
    let mut data = Vec::with_capacity(n * n);
    for y in 0..n {
        for x in 0..n {
            let mut hit = 0u32;
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = x as f64 - 0.5 + (f64::from(sx) + 0.5) / 4.0 - centre;
                    let py = y as f64 - 0.5 + (f64::from(sy) + 0.5) / 4.0 - centre;
                    if px * px + py * py <= r * r {
                        hit += 1;
                    }
                }
            }
            data.push(f64::from(hit) / 16.0);
        }
    }
    (n, data)
}

/// The sum over a square of side `n` whose top left pixel is (x, y), from an integral image of width `w + 1`.
fn square_sum(ii: &[f64], stride: usize, x: usize, y: usize, n: usize) -> f64 {
    ii[(y + n) * stride + x + n] - ii[y * stride + x + n] - ii[(y + n) * stride + x]
        + ii[y * stride + x]
}

/// The offset of the top of a peak from its middle sample, from the samples on both sides (a parabola through three points; 0 when it is not a peak), within half a pixel.
fn peak_offset(before: f64, middle: f64, after: f64) -> f64 {
    let d = before - 2.0 * middle + after;
    if d >= -1e-12 {
        return 0.0;
    }
    (0.5 * (before - after) / d).clamp(-0.5, 0.5)
}

/// The correlation of the picture with the template at every place where the whole template fits: the map (row after row, `width - n + 1` by `height - n + 1`) and the side of the template.
/// The sign is the one of the polarity, so a high number is always a mark. A place where the picture is flat has 0.
pub fn score_map(
    img: &Gray,
    s: &CorrelateSettings,
) -> Result<(Vec<f64>, usize, usize, usize), String> {
    check(s)?;
    let (n, t) = template(s.diameter);
    if img.w < n || img.h < n || img.data.len() != img.w * img.h {
        return Err(format!(
            "{PICTURE_SMALL}: the picture must be at least {n} by {n} pixels for a mark of {} pixels",
            s.diameter
        ));
    }
    let (ow, oh) = (img.w - n + 1, img.h - n + 1);
    if (ow * oh) as u64 * (n * n) as u64 > MAX_WORK {
        return Err(format!(
            "{TOO_BIG}: the search over {} by {} pixels with a template of {n} by {n} would take too long",
            img.w, img.h
        ));
    }
    let mean = t.iter().sum::<f64>() / t.len() as f64;
    let tz: Vec<f64> = t.iter().map(|v| v - mean).collect();
    let tnorm = tz.iter().map(|v| v * v).sum::<f64>().sqrt();
    let stride = img.w + 1;
    let mut sum = vec![0f64; stride * (img.h + 1)];
    let mut sq = vec![0f64; stride * (img.h + 1)];
    for y in 0..img.h {
        let (mut row, mut row_sq) = (0f64, 0f64);
        for x in 0..img.w {
            let v = f64::from(img.data[y * img.w + x]);
            row += v;
            row_sq += v * v;
            sum[(y + 1) * stride + x + 1] = sum[y * stride + x + 1] + row;
            sq[(y + 1) * stride + x + 1] = sq[y * stride + x + 1] + row_sq;
        }
    }
    let count = (n * n) as f64;
    let sign = match s.polarity {
        Polarity::Dark => -1.0,
        Polarity::Light => 1.0,
    };
    let mut map = vec![0f64; ow * oh];
    for y in 0..oh {
        for x in 0..ow {
            let total = square_sum(&sum, stride, x, y, n);
            let spread = square_sum(&sq, stride, x, y, n) - total * total / count;
            // a window whose grey levels hardly differ has no shape to correlate
            if spread < 1.0 {
                continue;
            }
            let mut cross = 0f64;
            for (ty, trow) in tz.chunks_exact(n).enumerate() {
                let start = (y + ty) * img.w + x;
                for (a, b) in img.data[start..start + n].iter().zip(trow) {
                    cross += f64::from(*a) * b;
                }
            }
            map[y * ow + x] = (sign * cross / (tnorm * spread.sqrt())).clamp(-1.0, 1.0);
        }
    }
    Ok((map, ow, oh, n))
}

/// The marks of a picture: the places where the correlation is the highest in its neighbourhood and at least `min_score`, the best first, none closer to a better one than a diameter. The
/// centre is worked out to a fraction of a pixel from the three correlations around the top. A mark whose template does not fit in the picture (it is nearer to the edge than half
/// the template) is not found. More marks than `max_matches` is the error `too_many_matches`.
pub fn correlate(img: &Gray, s: &CorrelateSettings) -> Result<Vec<Match>, String> {
    let (map, ow, oh, n) = score_map(img, s)?;
    let at = |x: usize, y: usize| map[y * ow + x];
    let mut candidates: Vec<(usize, usize, f64)> = Vec::new();
    for y in 0..oh {
        for x in 0..ow {
            let v = at(x, y);
            if v < s.min_score {
                continue;
            }
            let mut top = true;
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    if nx < 0 || ny < 0 || nx >= ow as i64 || ny >= oh as i64 {
                        continue;
                    }
                    let other = at(nx as usize, ny as usize);
                    // of two neighbours with the same score the first in reading order wins
                    let earlier = dy < 0 || (dy == 0 && dx < 0);
                    if other > v || (other == v && earlier) {
                        top = false;
                    }
                }
            }
            if top {
                candidates.push((x, y, v));
            }
        }
    }
    candidates.sort_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.cmp(&b.1))
            .then(a.0.cmp(&b.0))
    });
    let mut kept: Vec<(usize, usize, f64)> = Vec::new();
    for c in candidates {
        let apart = kept.iter().all(|k| {
            let (dx, dy) = (k.0 as f64 - c.0 as f64, k.1 as f64 - c.1 as f64);
            (dx * dx + dy * dy).sqrt() >= s.diameter
        });
        if apart {
            kept.push(c);
        }
    }
    if kept.len() > s.max_matches {
        return Err(format!(
            "{TOO_MANY}: {} marks are more than the {} that max_matches allows",
            kept.len(),
            s.max_matches
        ));
    }
    let half = (n - 1) as f64 / 2.0;
    Ok(kept
        .into_iter()
        .map(|(x, y, score)| {
            let left = if x > 0 { at(x - 1, y) } else { score };
            let right = if x + 1 < ow { at(x + 1, y) } else { score };
            let up = if y > 0 { at(x, y - 1) } else { score };
            let down = if y + 1 < oh { at(x, y + 1) } else { score };
            Match {
                x: x as f64 + half + peak_offset(left, score, right),
                y: y as f64 + half + peak_offset(up, score, down),
                score,
            }
        })
        .collect())
}

/// The marks as CSV: the line `CSV_HEADER` and one line for each mark (the place in pixels, to three decimals, and the score).
pub fn to_csv(found: &[Match]) -> String {
    let mut out = format!("{CSV_HEADER}\n");
    for m in found {
        out.push_str(&format!("{:.3},{:.3},{:.4}\n", m.x, m.y, m.score));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fiducial::{find, Settings};

    /// A picture of 90 by 70 with discs whose edge is grey where it cuts a pixel, so that a centre falls between pixels as it does with a camera.
    fn picture_of(discs: &[(f64, f64, f64)], board: u8, mark: u8) -> Gray {
        let (w, h) = (90usize, 70usize);
        let mut data = vec![board; w * h];
        for y in 0..h {
            for x in 0..w {
                let mut hit = 0u32;
                for sy in 0..4 {
                    for sx in 0..4 {
                        let px = x as f64 - 0.5 + (f64::from(sx) + 0.5) / 4.0;
                        let py = y as f64 - 0.5 + (f64::from(sy) + 0.5) / 4.0;
                        if discs
                            .iter()
                            .any(|(cx, cy, r)| (px - cx).powi(2) + (py - cy).powi(2) <= r * r)
                        {
                            hit += 1;
                        }
                    }
                }
                let share = f64::from(hit) / 16.0;
                data[y * w + x] =
                    (f64::from(board) + (f64::from(mark) - f64::from(board)) * share).round() as u8;
            }
        }
        Gray::new(w, h, data).unwrap()
    }

    fn near(m: &Match, x: f64, y: f64, tol: f64) -> bool {
        (m.x - x).abs() <= tol && (m.y - y).abs() <= tol
    }

    #[test]
    fn a_disc_is_found_where_it_is_to_a_fraction_of_a_pixel() {
        let img = picture_of(&[(41.3, 33.7, 8.0)], 220, 30);
        let found = correlate(&img, &CorrelateSettings::default().with_diameter(16.0)).unwrap();
        assert_eq!(found.len(), 1);
        assert!(near(&found[0], 41.3, 33.7, 0.25), "{:?}", found[0]);
        assert!(found[0].score > 0.95);
    }

    #[test]
    fn the_polarity_tells_a_light_disc_from_a_dark_one() {
        let dark = picture_of(&[(40.0, 30.0, 8.0)], 220, 30);
        let light = picture_of(&[(40.0, 30.0, 8.0)], 30, 220);
        let mut s = CorrelateSettings::default().with_diameter(16.0);
        assert_eq!(correlate(&dark, &s).unwrap().len(), 1);
        assert!(correlate(&light, &s).unwrap().is_empty());
        s.polarity = Polarity::Light;
        assert_eq!(correlate(&light, &s).unwrap().len(), 1);
        assert!(correlate(&dark, &s).unwrap().is_empty());
    }

    #[test]
    fn two_discs_are_two_marks_the_best_first() {
        let mut img = picture_of(&[(25.0, 30.0, 8.0), (65.0, 35.0, 8.0)], 220, 30);
        // the second mark is a little weaker: a speck of grey on it
        img.data[35 * 90 + 65] = 120;
        let found = correlate(&img, &CorrelateSettings::default().with_diameter(16.0)).unwrap();
        assert_eq!(found.len(), 2);
        assert!(found[0].score >= found[1].score);
        assert!(near(&found[0], 25.0, 30.0, 0.3));
        assert!(near(&found[1], 65.0, 35.0, 0.3));
    }

    #[test]
    fn the_light_can_change_without_the_mark_being_lost() {
        // the same board with half the contrast and brighter: the correlation does not change
        let strong = picture_of(&[(45.0, 35.0, 8.0)], 230, 20);
        let weak = picture_of(&[(45.0, 35.0, 8.0)], 200, 120);
        let s = CorrelateSettings::default().with_diameter(16.0);
        let (a, b) = (
            correlate(&strong, &s).unwrap(),
            correlate(&weak, &s).unwrap(),
        );
        assert_eq!((a.len(), b.len()), (1, 1));
        assert!((a[0].score - b[0].score).abs() < 0.02);
    }

    #[test]
    fn noise_does_not_lose_the_mark() {
        let mut img = picture_of(&[(44.0, 36.0, 8.0)], 220, 30);
        let mut rng = crate::sim::SimRng::new(7);
        for v in img.data.iter_mut() {
            let n = rng.below(41) as i32 - 20;
            *v = (i32::from(*v) + n).clamp(0, 255) as u8;
        }
        let found = correlate(&img, &CorrelateSettings::default().with_diameter(16.0)).unwrap();
        assert_eq!(found.len(), 1);
        assert!(near(&found[0], 44.0, 36.0, 0.6), "{:?}", found[0]);
    }

    #[test]
    fn a_disc_of_another_size_and_other_shapes_score_low() {
        let small = picture_of(&[(40.0, 30.0, 3.0)], 220, 30);
        let s = CorrelateSettings::default().with_diameter(16.0);
        assert!(correlate(&small, &s).unwrap().is_empty());
        // a bar is not a disc
        let mut bar = Gray::blank(90, 70, 220).unwrap();
        for y in 28..34 {
            for x in 20..70 {
                bar.data[y * 90 + x] = 30;
            }
        }
        assert!(correlate(&bar, &s).unwrap().is_empty());
    }

    #[test]
    fn a_flat_picture_has_no_marks() {
        let flat = Gray::blank(90, 70, 128).unwrap();
        let (map, _, _, _) = score_map(&flat, &CorrelateSettings::default()).unwrap();
        assert!(map.iter().all(|v| *v == 0.0));
        assert!(correlate(&flat, &CorrelateSettings::default())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn it_agrees_with_the_search_by_threshold() {
        let img = picture_of(&[(30.4, 25.6, 8.0), (60.2, 40.9, 8.0)], 220, 30);
        let by_threshold = find(&img, &Settings::default()).unwrap();
        let by_template =
            correlate(&img, &CorrelateSettings::default().with_diameter(16.0)).unwrap();
        assert_eq!(by_threshold.len(), 2);
        assert_eq!(by_template.len(), 2);
        for f in &by_threshold {
            assert!(
                by_template.iter().any(|m| near(m, f.x, f.y, 0.5)),
                "{f:?} was not found by the template"
            );
        }
    }

    #[test]
    fn a_mark_cut_by_the_edge_is_not_found_and_one_in_reach_is() {
        let cut = picture_of(&[(3.0, 30.0, 8.0)], 220, 30);
        let s = CorrelateSettings::default().with_diameter(16.0);
        assert!(correlate(&cut, &s).unwrap().is_empty());
        let inside = picture_of(&[(10.0, 30.0, 8.0)], 220, 30);
        assert_eq!(correlate(&inside, &s).unwrap().len(), 1);
    }

    #[test]
    fn too_many_marks_a_small_picture_and_bad_settings_are_refused() {
        let img = picture_of(&[(25.0, 30.0, 8.0), (65.0, 35.0, 8.0)], 220, 30);
        let mut s = CorrelateSettings::default().with_diameter(16.0);
        s.max_matches = 1;
        assert!(correlate(&img, &s).unwrap_err().starts_with(TOO_MANY));
        let tiny = Gray::blank(10, 10, 100).unwrap();
        assert!(correlate(&tiny, &CorrelateSettings::default())
            .unwrap_err()
            .starts_with(PICTURE_SMALL));
        for bad in [
            CorrelateSettings { diameter: 3.0, ..s },
            CorrelateSettings {
                diameter: 121.0,
                ..s
            },
            CorrelateSettings {
                diameter: f64::NAN,
                ..s
            },
            CorrelateSettings {
                min_score: 0.0,
                ..s
            },
            CorrelateSettings {
                min_score: 1.5,
                ..s
            },
            CorrelateSettings {
                max_matches: 0,
                ..s
            },
        ] {
            assert!(check(&bad).is_err(), "{bad:?}");
            assert!(correlate(&img, &bad).is_err());
        }
    }

    #[test]
    fn the_template_is_odd_centred_and_has_the_area_of_a_disc() {
        for d in [4.0, 9.5, 20.0, 33.0] {
            let (n, t) = template(d);
            assert_eq!(n % 2, 1);
            assert_eq!(t.len(), n * n);
            let area: f64 = t.iter().sum();
            let exact = std::f64::consts::PI * d * d / 4.0;
            assert!(
                (area - exact).abs() / exact < 0.05,
                "{d}: {area} against {exact}"
            );
            // the same from the left as from the right, and from the top as from the bottom
            for y in 0..n {
                for x in 0..n {
                    assert_eq!(t[y * n + x], t[y * n + (n - 1 - x)]);
                    assert_eq!(t[y * n + x], t[(n - 1 - y) * n + x]);
                }
            }
        }
    }

    #[test]
    fn the_csv_has_one_line_for_each_mark() {
        let text = to_csv(&[Match {
            x: 1.0,
            y: 2.5,
            score: 0.9,
        }]);
        assert_eq!(text, "x_px,y_px,score\n1.000,2.500,0.9000\n");
        assert_eq!(to_csv(&[]), "x_px,y_px,score\n");
    }

    #[test]
    fn the_top_of_a_peak_is_between_its_neighbours() {
        assert_eq!(peak_offset(0.5, 1.0, 0.5), 0.0);
        assert!(peak_offset(0.8, 1.0, 0.5) < 0.0);
        assert!(peak_offset(0.5, 1.0, 0.8) > 0.0);
        // not a peak: no shift
        assert_eq!(peak_offset(1.0, 0.5, 1.0), 0.0);
        assert!(peak_offset(0.0, 1.0, 0.99999).abs() <= 0.5);
    }

    impl CorrelateSettings {
        fn with_diameter(mut self, d: f64) -> Self {
            self.diameter = d;
            self
        }
    }
}
