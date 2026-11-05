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
