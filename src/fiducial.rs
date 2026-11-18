//! Round fiducial marks: the image is cut in dark and light with the threshold of Otsu, the connected spots are measured, and the ones that look like a disc of the expected size are the marks.

use crate::image::Gray;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// The marks are dark on a light board.
    Dark,
    /// The marks are light on a dark board (a bare copper pad under a ring light, for instance).
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub polarity: Polarity,
    /// The range of the diameter of a mark, in pixels.
    pub min_diameter: f64,
    pub max_diameter: f64,
    /// How much of the circle that holds the spot the spot fills (a perfect disc fills all of it): marks below this are not discs.
    pub min_fill: f64,
    /// How much longer than wide a mark may be (1 is a circle).
    pub max_elongation: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            polarity: Polarity::Dark,
            min_diameter: 8.0,
            max_diameter: 200.0,
            min_fill: 0.85,
            max_elongation: 1.25,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fiducial {
    pub x: f64,
    pub y: f64,
    /// The diameter of the circle that has the area of the spot.
    pub diameter: f64,
    /// 0 to 1, how much it looks like a disc.
    pub score: f64,
}

/// The threshold that best separates the dark from the light pixels (Otsu).
pub fn otsu(img: &Gray) -> u8 {
    let mut hist = [0u64; 256];
    for &v in &img.data {
        hist[usize::from(v)] += 1;
    }
    let total = img.data.len() as f64;
    let sum: f64 = hist
        .iter()
        .enumerate()
        .map(|(i, &c)| i as f64 * c as f64)
        .sum();
    let (mut sum_b, mut w_b, mut best, mut threshold) = (0.0, 0.0, -1.0, 0u8);
    for (t, &c) in hist.iter().enumerate() {
        w_b += c as f64;
        if w_b == 0.0 {
            continue;
        }
        let w_f = total - w_b;
        if w_f == 0.0 {
            break;
        }
        sum_b += t as f64 * c as f64;
        let (m_b, m_f) = (sum_b / w_b, (sum - sum_b) / w_f);
        let between = w_b * w_f * (m_b - m_f).powi(2);
        if between > best {
            best = between;
            threshold = t as u8;
        }
    }
    threshold
}

/// The marks of the image, the best ones first. An image with a single grey level has none.
pub fn find(img: &Gray, s: &Settings) -> Result<Vec<Fiducial>, String> {
    find_with(img, s, None)
}

/// Like `find`, with the threshold between dark and light given (0 to 255) instead of the one of Otsu; `None` is the threshold of Otsu.
pub fn find_with(img: &Gray, s: &Settings, threshold: Option<u8>) -> Result<Vec<Fiducial>, String> {
    if !(s.min_diameter.is_finite()
        && s.max_diameter.is_finite()
        && s.min_diameter > 1.0
        && s.max_diameter >= s.min_diameter
        && s.min_fill > 0.0
        && s.min_fill <= 1.0
        && s.max_elongation >= 1.0
        && s.max_elongation.is_finite())
    {
        return Err("the settings of the search make no sense (diameters from 2 up, a fill from 0 to 1, an elongation of 1 or more)".into());
    }
    let t = threshold.unwrap_or_else(|| otsu(img));
    let on = |v: u8| match s.polarity {
        Polarity::Dark => v <= t,
        Polarity::Light => v > t,
    };
    let (w, h) = (img.w, img.h);
    if img.data.iter().all(|&v| v == img.data[0]) {
        return Ok(Vec::new());
    }
    let mut seen = vec![false; w * h];
    let mut found = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for start in 0..w * h {
        if seen[start] || !on(img.data[start]) {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let (mut n, mut sx, mut sy) = (0u64, 0.0f64, 0.0f64);
        let (mut x0, mut x1, mut y0, mut y1) = (usize::MAX, 0usize, usize::MAX, 0usize);
        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            n += 1;
            sx += x as f64;
            sy += y as f64;
            (x0, x1, y0, y1) = (x0.min(x), x1.max(x), y0.min(y), y1.max(y));
            for (nx, ny) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if nx < w && ny < h && !seen[ny * w + nx] && on(img.data[ny * w + nx]) {
                    seen[ny * w + nx] = true;
                    stack.push(ny * w + nx);
                }
            }
        }
        let (bw, bh) = ((x1 - x0 + 1) as f64, (y1 - y0 + 1) as f64);
        let diameter = 2.0 * (n as f64 / std::f64::consts::PI).sqrt();
        if diameter < s.min_diameter || diameter > s.max_diameter {
            continue;
        }
        let elongation = bw.max(bh) / bw.min(bh);
        let fill = n as f64 / (std::f64::consts::PI * (bw.max(bh) / 2.0).powi(2));
        // a spot that touches the edge of the picture is cut and its centre cannot be trusted
        if x0 == 0
            || y0 == 0
            || x1 == w - 1
            || y1 == h - 1
            || elongation > s.max_elongation
            || fill < s.min_fill
        {
            continue;
        }
        let score = (fill.min(1.0) * (1.0 / elongation)).clamp(0.0, 1.0);
        found.push(Fiducial {
            x: sx / n as f64,
            y: sy / n as f64,
            diameter,
            score,
        });
    }
    found.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(found)
}

/// A mark whose centre was worked out again from the grey levels around it, with how clearly it stands out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Refined {
    pub x: f64,
    pub y: f64,
    /// The difference between the board and the inside of the mark, from 0 to 1 (1 is black against white).
    pub contrast: f64,
    /// 0 to 1: the score of the shape, lowered when the mark stands out only a little from the board.
    pub confidence: f64,
}

/// The centre of a mark to a fraction of a pixel. The first centre is the middle of the pixels that passed the threshold, which moves by up to half a pixel with the way the edge falls
/// on the pixels; here each pixel near the mark weighs by how much darker (or lighter) than the board it is, so the grey pixels of the edge count for what they hold. `full_contrast`
/// is the contrast (0 to 1) from which the confidence is the score of the shape alone. A mark that does not stand out (no contrast, or no board around it in the picture) keeps its
/// first centre and has no confidence.
pub fn refine(img: &Gray, f: &Fiducial, polarity: Polarity, full_contrast: f64) -> Refined {
    let unchanged = Refined {
        x: f.x,
        y: f.y,
        contrast: 0.0,
        confidence: 0.0,
    };
    let r = f.diameter / 2.0;
    let reach = (1.7 * r).ceil() as i64 + 1;
    let (cx, cy) = (f.x.round() as i64, f.y.round() as i64);
    let (w, h) = (img.w as i64, img.h as i64);
    // (x, y, distance from the first centre, grey level) of the pixels around the mark
    let mut near: Vec<(f64, f64, f64, f64)> = Vec::new();
    for y in (cy - reach).max(0)..=(cy + reach).min(h - 1) {
        for x in (cx - reach).max(0)..=(cx + reach).min(w - 1) {
            let d = ((x as f64 - f.x).powi(2) + (y as f64 - f.y).powi(2)).sqrt();
            let v = f64::from(img.data[y as usize * img.w + x as usize]);
            near.push((x as f64, y as f64, d, v));
        }
    }
    let mean = |from: f64, to: f64| {
        let (sum, n) = near
            .iter()
            .filter(|p| p.2 >= from && p.2 <= to)
            .fold((0.0, 0u32), |(s, n), p| (s + p.3, n + 1));
        (n > 0).then(|| sum / f64::from(n))
    };
    let (Some(inside), Some(board)) = (mean(0.0, 0.5 * r), mean(1.3 * r, 1.7 * r)) else {
        return unchanged;
    };
    let diff = match polarity {
        Polarity::Dark => board - inside,
        Polarity::Light => inside - board,
    };
    if diff < 1.0 {
        return unchanged;
    }
    let (mut sum, mut sx, mut sy) = (0.0, 0.0, 0.0);
    for &(x, y, _, v) in near.iter().filter(|p| p.2 <= 1.3 * r) {
        let weight = match polarity {
            Polarity::Dark => (board - v) / diff,
            Polarity::Light => (v - board) / diff,
        }
        .clamp(0.0, 1.0);
        sum += weight;
        sx += weight * x;
        sy += weight * y;
    }
    if sum <= 0.0 {
        return unchanged;
    }
    let contrast = (diff / 255.0).clamp(0.0, 1.0);
    let confidence = (f.score * (contrast / full_contrast.max(1e-9)).min(1.0)).clamp(0.0, 1.0);
    Refined {
        x: sx / sum,
        y: sy / sum,
        contrast,
        confidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture with a disc whose edge is grey where it cuts a pixel (each pixel is sampled 4 by 4), so the centre falls between pixels as it does with a camera.
    fn soft_disc(cx: f64, cy: f64, r: f64, board: u8, mark: u8) -> Gray {
        let (w, h) = (80usize, 60usize);
        let mut data = vec![board; w * h];
        for y in 0..h {
            for x in 0..w {
                let mut hit = 0u32;
                for sy in 0..4 {
                    for sx in 0..4 {
                        let px = x as f64 - 0.5 + (f64::from(sx) + 0.5) / 4.0;
                        let py = y as f64 - 0.5 + (f64::from(sy) + 0.5) / 4.0;
                        if (px - cx).powi(2) + (py - cy).powi(2) <= r * r {
                            hit += 1;
                        }
                    }
                }
                let t = f64::from(hit) / 16.0;
                data[y * w + x] =
                    (f64::from(board) + (f64::from(mark) - f64::from(board)) * t).round() as u8;
            }
        }
        Gray::new(w, h, data).unwrap()
    }

    #[test]
    fn the_centre_of_a_soft_edged_mark_comes_to_a_fraction_of_a_pixel_in_both_polarities() {
        for (polarity, board, mark) in [(Polarity::Dark, 220, 30), (Polarity::Light, 30, 220)] {
            for (cx, cy) in [(40.3, 30.7), (25.5, 20.25), (51.8, 33.1)] {
                let img = soft_disc(cx, cy, 9.0, board, mark);
                let s = Settings {
                    polarity,
                    ..Settings::default()
                };
                let found = find(&img, &s).unwrap();
                assert_eq!(found.len(), 1, "{polarity:?} {cx},{cy}");
                let r = refine(&img, &found[0], polarity, 0.3);
                assert!(
                    (r.x - cx).abs() < 0.06 && (r.y - cy).abs() < 0.06,
                    "{polarity:?} {cx},{cy}: {r:?}"
                );
                assert!(r.contrast > 0.7 && r.confidence > 0.75, "{r:?}");
            }
        }
    }

    #[test]
    fn a_mark_that_hardly_stands_out_has_a_low_confidence() {
        let img = soft_disc(40.0, 30.0, 9.0, 120, 100);
        let found = find(&img, &Settings::default()).unwrap();
        assert_eq!(found.len(), 1);
        let r = refine(&img, &found[0], Polarity::Dark, 0.3);
        assert!(r.contrast < 0.1 && r.confidence < 0.3, "{r:?}");
        // the contrast that counts as full confidence is a value that is given
        let lenient = refine(&img, &found[0], Polarity::Dark, 0.05);
        assert!(lenient.confidence > r.confidence);
    }

    #[test]
    fn a_mark_with_no_board_around_it_keeps_its_first_centre() {
        let img = Gray::blank(30, 30, 100).unwrap();
        let f = Fiducial {
            x: 10.0,
            y: 12.0,
            diameter: 8.0,
            score: 1.0,
        };
        let r = refine(&img, &f, Polarity::Dark, 0.3);
        assert_eq!((r.x, r.y, r.confidence), (10.0, 12.0, 0.0));
        // a mark at the corner of the picture, with the ring of board outside it, does not panic
        let corner = Fiducial {
            x: 1.0,
            y: 1.0,
            diameter: 20.0,
            score: 1.0,
        };
        let c = refine(&img, &corner, Polarity::Light, 0.3);
        assert_eq!(c.confidence, 0.0);
    }
}
