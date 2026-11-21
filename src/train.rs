//! Training a mark from examples: a person cuts one or more rectangles around the same mark out of a picture, and the program keeps their average as the template of that mark. The
//! template is then slid over a picture (and, when asked, turned a step at a time) and the places where the normalised correlation of the grey levels is high are the marks, each with
//! its place to a fraction of a pixel, its angle and its score. This is for a mark that is none of the shapes the library knows (a logo, a letter, a pattern of the maker of the board).
//!
//! What is refused, with a code that names the reason: examples that are too small or too large for the limits of the settings, outside the picture, flat (no contrast), of different
//! sizes, or that do not look like each other; and a template that would be found as well a few pixels away as where it is (a stripe, a plain edge), because it would give a place
//! that cannot be trusted. Everything here works on grey pictures and is tried on pictures drawn by the tests with a known truth; no photograph of a real board has been used.
//!
//! The angle follows the picture: positive turns the mark clockwise as it is seen on the screen (the y axis points down), and the place is that of the centre of the template, with the
//! centre of the first pixel at (0, 0). A search that turns the template looks only inside the circle that fits the template, so that the corners do not matter at any angle.

use crate::image::Gray;

/// The code of the refusal of a request for no example, or for more than [`MAX_SAMPLES`].
pub const SAMPLE_COUNT: &str = "sample_count";
/// The code of the refusal of an example smaller than the settings allow.
pub const SAMPLE_SMALL: &str = "sample_small";
/// The code of the refusal of an example larger than the settings allow.
pub const SAMPLE_LARGE: &str = "sample_large";
/// The code of the refusal of an example that is not entirely inside the picture.
pub const SAMPLE_OUTSIDE: &str = "sample_outside";
/// The code of the refusal of an example (or a template) with too little contrast.
pub const SAMPLE_FLAT: &str = "sample_flat";
/// The code of the refusal of examples of different sizes.
pub const SAMPLES_DIFFER: &str = "samples_differ";
/// The code of the refusal of examples that do not look like each other.
pub const SAMPLES_DISAGREE: &str = "samples_disagree";
/// The code of the refusal of a template that looks as much like itself a few pixels away.
pub const MARK_AMBIGUOUS: &str = "mark_ambiguous";
/// The code of the refusal of a picture smaller than the template.
pub const PICTURE_SMALL: &str = "picture_small";
/// The code of the refusal of a search that would take too long.
pub const TOO_BIG: &str = "train_too_big";
/// The code of the refusal of a picture with more marks than the settings allow.
pub const TOO_MANY: &str = "too_many_matches";
/// The code of the refusal of a file that is not a trained mark.
pub const BAD_FILE: &str = "bad_mark_file";

/// The most examples of one training.
pub const MAX_SAMPLES: usize = 64;
/// The largest side of a template that any setting or file may ask for, in pixels.
pub const SIDE_LIMIT: usize = 256;
/// The most multiply-adds of one search (the places of the picture times the pixels of the template times the angles).
pub const MAX_WORK: u64 = 1_500_000_000;
/// The first line of the file of a trained mark.
pub const MAGIC: &str = "EH3DMARK 1";
/// The most bytes of the file of a trained mark that are read.
pub const MAX_FILE: usize = 70_000;

/// A rectangle of a picture, in pixels: the top left corner and the size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

/// What is asked of the training and of the search with the template.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrainSettings {
    /// The smallest and the largest side of an example, in pixels.
    pub min_side: usize,
    pub max_side: usize,
    /// The least standard deviation of the grey levels of an example (0 to 255).
    pub min_contrast: f64,
    /// The least correlation of each example with the first one when there are several (more than 0, at most 1).
    pub agree_score: f64,
    /// The most that the template may look like itself a few pixels away (more than 0, less than 1).
    pub max_ambiguity: f64,
    /// The least correlation that counts as a mark in the search (more than 0, at most 1).
    pub min_score: f64,
    /// The step of the angles of the search in degrees (1 to 45), and how far from upright it goes in each direction (0 does not turn the template, 180 is every angle).
    pub turn_step_deg: f64,
    pub turn_max_deg: f64,
    /// The most marks in a picture; a picture with more is refused.
    pub max_matches: usize,
}

impl Default for TrainSettings {
    fn default() -> Self {
        TrainSettings {
            min_side: 8,
            max_side: 96,
            min_contrast: 20.0,
            agree_score: 0.8,
            max_ambiguity: 0.9,
            min_score: 0.8,
            turn_step_deg: 5.0,
            turn_max_deg: 0.0,
            max_matches: 1000,
        }
    }
}

/// Whether the settings make sense; the reason when they do not.
pub fn check(s: &TrainSettings) -> Result<(), String> {
    if !(4..=SIDE_LIMIT).contains(&s.min_side) || !(s.min_side..=SIDE_LIMIT).contains(&s.max_side) {
        return Err(format!("the sides of an example must be from 4 to {SIDE_LIMIT} pixels and the largest not below the smallest"));
    }
    if !(s.min_contrast.is_finite() && (0.0..=255.0).contains(&s.min_contrast)) {
        return Err("the least contrast of an example must be from 0 to 255".into());
    }
    if !(s.agree_score > 0.0 && s.agree_score <= 1.0) {
        return Err("the agreement of the examples must be more than 0 and at most 1".into());
    }
    if !(s.max_ambiguity > 0.0 && s.max_ambiguity < 1.0) {
        return Err(
            "the most that a mark may look like itself must be more than 0 and less than 1".into(),
        );
    }
    if !(s.min_score > 0.0 && s.min_score <= 1.0) {
        return Err("the least score of a trained mark must be more than 0 and at most 1".into());
    }
    if !(s.turn_step_deg.is_finite() && (1.0..=45.0).contains(&s.turn_step_deg)) {
        return Err("the step of the turn must be from 1 to 45 degrees".into());
    }
    if !(s.turn_max_deg.is_finite() && (0.0..=180.0).contains(&s.turn_max_deg)) {
        return Err("the turn of the search must be from 0 (upright only) to 180 degrees".into());
    }
    if !(1..=100_000).contains(&s.max_matches) {
        return Err("the most marks of a search must be from 1 to 100000".into());
    }
    Ok(())
}

/// A trained mark: the grey levels of the average of its examples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub w: usize,
    pub h: usize,
    /// Row after row.
    pub data: Vec<u8>,
    /// How many examples it is the average of.
    pub samples: usize,
}

/// A place where the picture looks like the template.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Found {
    /// The centre of the mark in pixels.
    pub x: f64,
    pub y: f64,
    /// How much the mark is turned from the template, in degrees, clockwise on the screen, from -180 (not included) to 180.
    pub angle_deg: f64,
    /// The correlation, from the least that is asked for to 1.
    pub score: f64,
}

/// The standard deviation of the grey levels.
fn spread_of(v: &[u8]) -> f64 {
    let n = v.len() as f64;
    let mean = v.iter().map(|&a| f64::from(a)).sum::<f64>() / n;
    (v.iter()
        .map(|&a| (f64::from(a) - mean).powi(2))
        .sum::<f64>()
        / n)
        .sqrt()
}

/// The normalised correlation of two lists of the same length; 0 when either has no spread.
fn ncc(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let (mut c, mut va, mut vb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        c += (x - ma) * (y - mb);
        va += (x - ma) * (x - ma);
        vb += (y - mb) * (y - mb);
    }
    if va < 1e-9 || vb < 1e-9 {
        return 0.0;
    }
    (c / (va * vb).sqrt()).clamp(-1.0, 1.0)
}

/// The grey levels of a rectangle of the picture, or `None` when it is not entirely inside it.
fn cut(img: &Gray, r: &Rect) -> Option<Vec<u8>> {
    let (x1, y1) = (r.x.checked_add(r.w)?, r.y.checked_add(r.h)?);
    if r.w == 0 || r.h == 0 || x1 > img.w || y1 > img.h || img.data.len() != img.w * img.h {
        return None;
    }
    let mut out = Vec::with_capacity(r.w * r.h);
    for y in r.y..y1 {
        out.extend_from_slice(&img.data[y * img.w + r.x..y * img.w + x1]);
    }
    Some(out)
}

/// How much a template looks like itself shifted by a quarter of its side (at least 2 pixels) along the rows, the columns and both diagonals: the highest correlation of the part where
/// the two copies overlap. Near 1 the template is a stripe, an edge or a smooth ramp; a mark with a shape of its own stays well below.
pub fn ambiguity(data: &[u8], w: usize, h: usize) -> f64 {
    if w == 0 || h == 0 || data.len() != w * h {
        return 1.0;
    }
    let k = (w.min(h) / 4).max(2) as i64;
    let mut worst = 0.0f64;
    for (dx, dy) in [(1i64, 0i64), (0, 1), (1, 1), (1, -1)] {
        let (sx, sy) = (dx * k, dy * k);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for y in 0..h as i64 {
            for x in 0..w as i64 {
                let (x2, y2) = (x + sx, y + sy);
                if x2 < 0 || y2 < 0 || x2 >= w as i64 || y2 >= h as i64 {
                    continue;
                }
                a.push(f64::from(data[(y as usize) * w + x as usize]));
                b.push(f64::from(data[(y2 as usize) * w + x2 as usize]));
            }
        }
        if a.len() >= 16 {
            worst = worst.max(ncc(&a, &b));
        }
    }
    worst
}

/// The template that the examples make: the rectangles `rects` of the picture, all of the same size, whose average is kept. One example is enough; with several, each must correlate with
/// the first one at least `agree_score`, so that a rectangle put on the wrong place is found out instead of blurring the template.
pub fn train(img: &Gray, rects: &[Rect], s: &TrainSettings) -> Result<Template, String> {
    check(s)?;
    if rects.is_empty() || rects.len() > MAX_SAMPLES {
        return Err(format!(
            "{SAMPLE_COUNT}: {} examples; from 1 to {MAX_SAMPLES} are needed",
            rects.len()
        ));
    }
    let (w, h) = (rects[0].w, rects[0].h);
    if rects.iter().any(|r| r.w != w || r.h != h) {
        return Err(format!(
            "{SAMPLES_DIFFER}: the examples must all have the same size"
        ));
    }
    if w < s.min_side || h < s.min_side {
        return Err(format!(
            "{SAMPLE_SMALL}: an example of {w} by {h} pixels is smaller than {} by {}",
            s.min_side, s.min_side
        ));
    }
    if w > s.max_side || h > s.max_side {
        return Err(format!(
            "{SAMPLE_LARGE}: an example of {w} by {h} pixels is larger than {} by {}",
            s.max_side, s.max_side
        ));
    }
    let mut cuts: Vec<Vec<u8>> = Vec::with_capacity(rects.len());
    for (i, r) in rects.iter().enumerate() {
        let c = cut(img, r).ok_or_else(|| {
            format!(
                "{SAMPLE_OUTSIDE}: the example {} is not entirely inside the picture of {} by {} pixels",
                i + 1,
                img.w,
                img.h
            )
        })?;
        let contrast = spread_of(&c);
        if contrast < s.min_contrast || contrast < 1.0 {
            return Err(format!(
                "{SAMPLE_FLAT}: the example {} has a contrast of {contrast:.1}, less than the {} that is asked for",
                i + 1,
                s.min_contrast.max(1.0)
            ));
        }
        cuts.push(c);
    }
    let first: Vec<f64> = cuts[0].iter().map(|&v| f64::from(v)).collect();
    for (i, c) in cuts.iter().enumerate().skip(1) {
        let other: Vec<f64> = c.iter().map(|&v| f64::from(v)).collect();
        let score = ncc(&first, &other);
        if score < s.agree_score {
            return Err(format!(
                "{SAMPLES_DISAGREE}: the example {} looks like the first one by {score:.2}, less than the {} that is asked for",
                i + 1,
                s.agree_score
            ));
        }
    }
    let n = cuts.len() as u32;
    let data: Vec<u8> = (0..w * h)
        .map(|i| {
            let total: u32 = cuts.iter().map(|c| u32::from(c[i])).sum();
            ((total + n / 2) / n) as u8
        })
        .collect();
    let amb = ambiguity(&data, w, h);
    if amb > s.max_ambiguity {
        return Err(format!(
            "{MARK_AMBIGUOUS}: the mark looks like itself {amb:.2} when it is moved a few pixels, more than the {} that is allowed",
            s.max_ambiguity
        ));
    }
    Ok(Template {
        w,
        h,
        data,
        samples: cuts.len(),
    })
}

impl Template {
    /// The file of the template: the line [`MAGIC`], the line `WIDTH HEIGHT SAMPLES` and the grey levels row after row.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = format!("{MAGIC}\n{} {} {}\n", self.w, self.h, self.samples).into_bytes();
        out.extend_from_slice(&self.data);
        out
    }

    /// Reads the file of a template. Refused, with [`BAD_FILE`] and the reason, when it is bigger than [`MAX_FILE`], does not begin with the first line, has a size that is not a
    /// number from 1 to [`SIDE_LIMIT`] (or more pixels than the limit lets), a count of examples outside 1 to [`MAX_SAMPLES`], a body of another length, or no contrast at all.
    pub fn from_bytes(bytes: &[u8]) -> Result<Template, String> {
        let bad = |why: &str| format!("{BAD_FILE}: {why}");
        if bytes.len() > MAX_FILE {
            return Err(bad("the file is larger than the limit"));
        }
        let mut lines = bytes.splitn(3, |&b| b == b'\n');
        let first = lines.next().unwrap_or(&[]);
        if first != MAGIC.as_bytes() {
            return Err(bad(
                "it does not begin with the first line of a trained mark",
            ));
        }
        let sizes = lines
            .next()
            .ok_or_else(|| bad("the size line is missing"))?;
        let body = lines
            .next()
            .ok_or_else(|| bad("there are no grey levels"))?;
        let text = std::str::from_utf8(sizes).map_err(|_| bad("the size line is not text"))?;
        let nums: Vec<&str> = text.split(' ').collect();
        if nums.len() != 3 {
            return Err(bad("the size line is WIDTH HEIGHT SAMPLES"));
        }
        let mut v = [0usize; 3];
        for (slot, n) in v.iter_mut().zip(&nums) {
            if n.is_empty() || n.len() > 5 || !n.bytes().all(|b| b.is_ascii_digit()) {
                return Err(bad("a number of the size line is not a number"));
            }
            *slot = n
                .parse()
                .map_err(|_| bad("a number of the size line is too large"))?;
        }
        let (w, h, samples) = (v[0], v[1], v[2]);
        if !(1..=SIDE_LIMIT).contains(&w) || !(1..=SIDE_LIMIT).contains(&h) {
            return Err(bad("the sides must be from 1 to the limit"));
        }
        if !(1..=MAX_SAMPLES).contains(&samples) {
            return Err(bad("the count of examples is outside the limit"));
        }
        if body.len() != w * h {
            return Err(bad(&format!(
                "{w} by {h} pixels need {} grey levels, not {}",
                w * h,
                body.len()
            )));
        }
        if spread_of(body) < 1.0 {
            return Err(format!("{SAMPLE_FLAT}: the template has no contrast"));
        }
        Ok(Template {
            w,
            h,
            data: body.to_vec(),
            samples,
        })
    }
}

/// The template turned by an angle, ready to be slid over a picture of a given width: the offsets of its pixels from the top left corner of the window, the grey levels without their
/// mean, and the length of that vector.
struct Prepared {
    offsets: Vec<usize>,
    tz: Vec<f64>,
    norm: f64,
}

/// The value of the template at a point that is not on a pixel, by bilinear interpolation; `None` when the point is outside the template.
fn sample(t: &Template, x: f64, y: f64) -> Option<f64> {
    if !(x >= 0.0 && y >= 0.0 && x <= (t.w - 1) as f64 && y <= (t.h - 1) as f64) {
        return None;
    }
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(t.w - 1), (y0 + 1).min(t.h - 1));
    let (fx, fy) = (x - x0 as f64, y - y0 as f64);
    let at = |xx: usize, yy: usize| f64::from(t.data[yy * t.w + xx]);
    Some(
        at(x0, y0) * (1.0 - fx) * (1.0 - fy)
            + at(x1, y0) * fx * (1.0 - fy)
            + at(x0, y1) * (1.0 - fx) * fy
            + at(x1, y1) * fx * fy,
    )
}

/// The template as it looks when the mark is turned by `angle_deg` (clockwise on the screen): each pixel of the window looks at the template turned back by the angle. With `circle` only
/// the pixels inside the circle that fits the template count, so that every angle compares the same pixels.
fn prepare(t: &Template, angle_deg: f64, circle: bool, stride: usize) -> Option<Prepared> {
    let (s, c) = angle_deg.to_radians().sin_cos();
    let (cx, cy) = ((t.w - 1) as f64 / 2.0, (t.h - 1) as f64 / 2.0);
    let radius = t.w.min(t.h) as f64 / 2.0;
    let (mut offsets, mut values) = (Vec::new(), Vec::new());
    for y in 0..t.h {
        for x in 0..t.w {
            let (u, v) = (x as f64 - cx, y as f64 - cy);
            if circle && u * u + v * v > radius * radius {
                continue;
            }
            // the template turned back by the angle
            let (sx, sy) = (cx + u * c + v * s, cy - u * s + v * c);
            if let Some(value) = sample(t, sx, sy) {
                offsets.push(y * stride + x);
                values.push(value);
            }
        }
    }
    if values.len() < 16 {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let tz: Vec<f64> = values.iter().map(|v| v - mean).collect();
    let norm = tz.iter().map(|v| v * v).sum::<f64>().sqrt();
    if norm < 1e-6 {
        return None;
    }
    Some(Prepared { offsets, tz, norm })
}

/// The correlation of the picture with a prepared template whose window has its top left corner at (x, y).
fn score_at(img: &Gray, p: &Prepared, x: usize, y: usize) -> f64 {
    let base = y * img.w + x;
    let (mut sum, mut sq, mut cross) = (0f64, 0f64, 0f64);
    for (o, t) in p.offsets.iter().zip(&p.tz) {
        let v = f64::from(img.data[base + o]);
        sum += v;
        sq += v * v;
        cross += v * t;
    }
    let spread = sq - sum * sum / p.tz.len() as f64;
    if spread < 1.0 {
        return 0.0;
    }
    (cross / (p.norm * spread.sqrt())).clamp(-1.0, 1.0)
}

/// The offset of the top of a peak from its middle sample, from the samples on both sides (a parabola through three points; 0 when it is not a peak), within half a sample.
fn peak_offset(before: f64, middle: f64, after: f64) -> f64 {
    let d = before - 2.0 * middle + after;
    if d >= -1e-12 {
        return 0.0;
    }
    (0.5 * (before - after) / d).clamp(-0.5, 0.5)
}

/// The angles of the search: 0 alone, or every step from `-turn_max_deg` to `turn_max_deg` (the full circle once).
fn angles(s: &TrainSettings) -> Vec<f64> {
    if s.turn_max_deg <= 0.0 {
        return vec![0.0];
    }
    let n = (s.turn_max_deg / s.turn_step_deg).floor() as i64;
    let mut out: Vec<f64> = (-n..=n).map(|k| k as f64 * s.turn_step_deg).collect();
    // -180 and 180 are the same angle
    if out.len() > 1 && (out[out.len() - 1] - out[0] - 360.0).abs() < 1e-9 {
        out.pop();
    }
    out
}

/// The angle brought into the range from -180 (not included) to 180.
fn wrap(mut a: f64) -> f64 {
    while a > 180.0 {
        a -= 360.0;
    }
    while a <= -180.0 {
        a += 360.0;
    }
    a
}

/// The places of the picture that look like the template, the best first, none closer to a better one than half the side of the template. Without a turn (`turn_max_deg` 0) the template is
/// the rectangle as it is; with a turn it is tried at every angle of the search and the angle of each mark is refined to a fraction of the step from the scores at the angles on both
/// sides. A mark whose window does not fit in the picture is not found. More marks than `max_matches` is the error `too_many_matches`, and a search that would take more than
/// [`MAX_WORK`] is the error `train_too_big`.
pub fn locate(img: &Gray, t: &Template, s: &TrainSettings) -> Result<Vec<Found>, String> {
    check(s)?;
    if t.w == 0 || t.h == 0 || t.data.len() != t.w * t.h || t.w > SIDE_LIMIT || t.h > SIDE_LIMIT {
        return Err(format!("{BAD_FILE}: the template is not a picture"));
    }
    if img.w < t.w || img.h < t.h || img.data.len() != img.w * img.h {
        return Err(format!(
            "{PICTURE_SMALL}: the picture must be at least {} by {} pixels for this mark",
            t.w, t.h
        ));
    }
    let steps = angles(s);
    let circle = s.turn_max_deg > 0.0;
    let (ow, oh) = (img.w - t.w + 1, img.h - t.h + 1);
    let work = (ow as u64) * (oh as u64) * (t.w as u64) * (t.h as u64) * steps.len() as u64;
    if work > MAX_WORK {
        return Err(format!(
            "{TOO_BIG}: the search over {} by {} pixels with a mark of {} by {} at {} angles would take too long",
            img.w,
            img.h,
            t.w,
            t.h,
            steps.len()
        ));
    }
    let mut candidates: Vec<Found> = Vec::new();
    for &angle in &steps {
        let Some(p) = prepare(t, angle, circle, img.w) else {
            return Err(format!("{SAMPLE_FLAT}: the template has no contrast"));
        };
        let mut map = vec![0f64; ow * oh];
        for y in 0..oh {
            for x in 0..ow {
                map[y * ow + x] = score_at(img, &p, x, y);
            }
        }
        for y in 0..oh {
            for x in 0..ow {
                let v = map[y * ow + x];
                if v < s.min_score {
                    continue;
                }
                let mut top = true;
                'around: for dy in -1i64..=1 {
                    for dx in -1i64..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                        if nx < 0 || ny < 0 || nx >= ow as i64 || ny >= oh as i64 {
                            continue;
                        }
                        let o = map[ny as usize * ow + nx as usize];
                        // a plateau gives its mark to the first of its places only
                        if o > v || (o == v && (dy < 0 || (dy == 0 && dx < 0))) {
                            top = false;
                            break 'around;
                        }
                    }
                }
                if !top {
                    continue;
                }
                let at = |xx: usize, yy: usize| map[yy * ow + xx];
                let sub_x = if x > 0 && x + 1 < ow {
                    peak_offset(at(x - 1, y), v, at(x + 1, y))
                } else {
                    0.0
                };
                let sub_y = if y > 0 && y + 1 < oh {
                    peak_offset(at(x, y - 1), v, at(x, y + 1))
                } else {
                    0.0
                };
                candidates.push(Found {
                    x: x as f64 + (t.w - 1) as f64 / 2.0 + sub_x,
                    y: y as f64 + (t.h - 1) as f64 / 2.0 + sub_y,
                    angle_deg: wrap(angle),
                    score: v,
                });
            }
        }
        if candidates.len() > s.max_matches.saturating_mul(64) {
            return Err(format!(
                "{TOO_MANY}: more than {} candidate places",
                s.max_matches.saturating_mul(64)
            ));
        }
    }
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                a.angle_deg
                    .partial_cmp(&b.angle_deg)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
            .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });
    let apart = t.w.max(t.h) as f64 / 2.0;
    let mut kept: Vec<Found> = Vec::new();
    for c in candidates {
        if kept.iter().all(|k| (k.x - c.x).hypot(k.y - c.y) >= apart) {
            kept.push(c);
            if kept.len() > s.max_matches {
                return Err(format!(
                    "{TOO_MANY}: more than {} marks in the picture",
                    s.max_matches
                ));
            }
        }
    }
    if circle {
        for f in &mut kept {
            let (px, py) = (
                (f.x - (t.w - 1) as f64 / 2.0).round().max(0.0) as usize,
                (f.y - (t.h - 1) as f64 / 2.0).round().max(0.0) as usize,
            );
            if px >= ow || py >= oh {
                continue;
            }
            let at_angle = |a: f64| prepare(t, a, true, img.w).map(|p| score_at(img, &p, px, py));
            let (b, m, a) = (
                at_angle(f.angle_deg - s.turn_step_deg),
                at_angle(f.angle_deg),
                at_angle(f.angle_deg + s.turn_step_deg),
            );
            if let (Some(b), Some(m), Some(a)) = (b, m, a) {
                f.angle_deg = wrap(f.angle_deg + s.turn_step_deg * peak_offset(b, m, a));
            }
        }
    }
    Ok(kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mark with no symmetry: a ring with a bar from the middle to one side, drawn at the centre (cx, cy) turned by `turn_deg` clockwise on the screen, over a light board.
    fn mark_picture(w: usize, h: usize, marks: &[(f64, f64, f64)]) -> Gray {
        let mut data = vec![215u8; w * h];
        for (i, v) in data.iter_mut().enumerate() {
            let (x, y) = ((i % w) as f64, (i / w) as f64);
            for &(cx, cy, turn) in marks {
                // the pixel as seen from the mark, turned back
                let (s, c) = turn.to_radians().sin_cos();
                let (u, vv) = (x - cx, y - cy);
                let (a, b) = (u * c + vv * s, -u * s + vv * c);
                let r = a.hypot(b);
                let ring = (5.0..8.0).contains(&r);
                let bar = (0.0..=8.0).contains(&a) && b.abs() <= 1.2;
                if ring || bar {
                    *v = 40;
                }
            }
        }
        Gray::new(w, h, data).unwrap()
    }

    fn rect_around(cx: usize, cy: usize, half: usize) -> Rect {
        Rect {
            x: cx - half,
            y: cy - half,
            w: 2 * half + 1,
            h: 2 * half + 1,
        }
    }

    #[test]
    fn a_new_mark_is_trained_from_an_example() {
        let pic = mark_picture(80, 60, &[(30.0, 30.0, 0.0)]);
        let t = train(&pic, &[rect_around(30, 30, 11)], &TrainSettings::default()).unwrap();
        assert_eq!((t.w, t.h, t.samples), (23, 23, 1));
        // the template is found where it was cut
        let found = locate(&pic, &t, &TrainSettings::default()).unwrap();
        assert_eq!(found.len(), 1);
        assert!((found[0].x - 30.0).abs() < 0.2 && (found[0].y - 30.0).abs() < 0.2);
        assert!(found[0].score > 0.99);
    }

    #[test]
    fn examples_that_agree_are_averaged_and_one_in_the_wrong_place_is_refused() {
        let pic = mark_picture(120, 50, &[(25.0, 25.0, 0.0), (70.0, 25.0, 0.0)]);
        let s = TrainSettings::default();
        let both = [rect_around(25, 25, 11), rect_around(70, 25, 11)];
        assert_eq!(train(&pic, &both, &s).unwrap().samples, 2);
        // a rectangle of the board with a bit of the ring does not look like the first one
        let off = [rect_around(25, 25, 11), rect_around(46, 25, 11)];
        let e = train(&pic, &off, &s).unwrap_err();
        assert!(
            e.starts_with(SAMPLES_DISAGREE) || e.starts_with(SAMPLE_FLAT),
            "{e}"
        );
    }

    #[test]
    fn every_refusal_of_a_training_has_its_code() {
        let pic = mark_picture(60, 60, &[(30.0, 30.0, 0.0)]);
        let s = TrainSettings::default();
        let code = |rects: &[Rect], s: &TrainSettings| train(&pic, rects, s).unwrap_err();
        assert!(code(&[], &s).starts_with(SAMPLE_COUNT));
        assert!(code(&[rect_around(30, 30, 3)], &s).starts_with(SAMPLE_SMALL));
        let narrow = TrainSettings { max_side: 40, ..s };
        assert!(code(&[rect_around(30, 30, 20)], &narrow).starts_with(SAMPLE_LARGE));
        let big = TrainSettings { max_side: 100, ..s };
        assert!(code(
            &[Rect {
                x: 40,
                y: 40,
                w: 30,
                h: 30
            }],
            &big
        )
        .starts_with(SAMPLE_OUTSIDE));
        assert!(code(
            &[Rect {
                x: usize::MAX,
                y: 0,
                w: 9,
                h: 9
            }],
            &s
        )
        .starts_with(SAMPLE_OUTSIDE));
        assert!(code(&[rect_around(8, 8, 5)], &s).starts_with(SAMPLE_FLAT));
        let mixed = [rect_around(30, 30, 11), rect_around(30, 30, 10)];
        assert!(code(&mixed, &s).starts_with(SAMPLES_DIFFER));
        let many = vec![rect_around(30, 30, 11); MAX_SAMPLES + 1];
        assert!(code(&many, &s).starts_with(SAMPLE_COUNT));
    }

    #[test]
    fn a_stripe_is_refused_because_it_cannot_say_where_it_is() {
        let mut data = vec![200u8; 64 * 64];
        for y in 0..64 {
            for x in 0..64 {
                if (20..28).contains(&x) {
                    data[y * 64 + x] = 30;
                }
            }
        }
        let pic = Gray::new(64, 64, data).unwrap();
        let e = train(&pic, &[rect_around(24, 32, 12)], &TrainSettings::default()).unwrap_err();
        assert!(e.starts_with(MARK_AMBIGUOUS), "{e}");
    }

    #[test]
    fn the_settings_with_no_sense_are_refused_with_the_name_of_what_is_wrong() {
        let d = TrainSettings::default();
        assert!(check(&d).is_ok());
        for bad in [
            TrainSettings { min_side: 3, ..d },
            TrainSettings { max_side: 7, ..d },
            TrainSettings {
                max_side: SIDE_LIMIT + 1,
                ..d
            },
            TrainSettings {
                min_contrast: f64::NAN,
                ..d
            },
            TrainSettings {
                agree_score: 0.0,
                ..d
            },
            TrainSettings {
                max_ambiguity: 1.0,
                ..d
            },
            TrainSettings {
                min_score: 1.5,
                ..d
            },
            TrainSettings {
                turn_step_deg: 0.5,
                ..d
            },
            TrainSettings {
                turn_max_deg: 181.0,
                ..d
            },
            TrainSettings {
                max_matches: 0,
                ..d
            },
        ] {
            assert!(check(&bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_angles_of_the_search_cover_the_circle_once() {
        let s = TrainSettings::default();
        assert_eq!(angles(&s), vec![0.0]);
        let full = angles(&TrainSettings {
            turn_max_deg: 180.0,
            turn_step_deg: 30.0,
            ..s
        });
        assert_eq!(full.len(), 12);
        assert!(full.contains(&-180.0) && !full.contains(&180.0));
        let some = angles(&TrainSettings {
            turn_max_deg: 20.0,
            turn_step_deg: 10.0,
            ..s
        });
        assert_eq!(some, vec![-20.0, -10.0, 0.0, 10.0, 20.0]);
        assert_eq!(wrap(190.0), -170.0);
        assert_eq!(wrap(-180.0), 180.0);
    }

    #[test]
    fn the_file_of_a_mark_comes_back_the_same_and_a_bad_one_is_refused_with_its_code() {
        let pic = mark_picture(60, 60, &[(30.0, 30.0, 0.0)]);
        let t = train(&pic, &[rect_around(30, 30, 11)], &TrainSettings::default()).unwrap();
        assert_eq!(Template::from_bytes(&t.to_bytes()).unwrap(), t);
        let good = t.to_bytes();
        let cases: Vec<Vec<u8>> = vec![
            Vec::new(),
            b"EH3DMARK 2\n23 23 1\n".to_vec(),
            good[..good.len() - 1].to_vec(),
            [good.clone(), vec![0]].concat(),
            b"EH3DMARK 1\n23 23\n".to_vec(),
            b"EH3DMARK 1\n0 23 1\n".to_vec(),
            b"EH3DMARK 1\n99999999999999999999 23 1\n".to_vec(),
            b"EH3DMARK 1\n23 23 0\n".to_vec(),
            b"EH3DMARK 1\n-3 23 1\n".to_vec(),
            vec![b'x'; MAX_FILE + 1],
        ];
        for bytes in cases {
            let e = Template::from_bytes(&bytes).unwrap_err();
            assert!(e.starts_with(BAD_FILE), "{e}");
        }
        let mut flat = b"EH3DMARK 1\n10 10 1\n".to_vec();
        flat.extend(vec![7u8; 100]);
        assert!(Template::from_bytes(&flat)
            .unwrap_err()
            .starts_with(SAMPLE_FLAT));
    }

    #[test]
    fn a_picture_smaller_than_the_mark_and_a_search_too_big_are_refused() {
        let pic = mark_picture(60, 60, &[(30.0, 30.0, 0.0)]);
        let t = train(&pic, &[rect_around(30, 30, 11)], &TrainSettings::default()).unwrap();
        let small = Gray::blank(10, 10, 100).unwrap();
        assert!(locate(&small, &t, &TrainSettings::default())
            .unwrap_err()
            .starts_with(PICTURE_SMALL));
        let all = TrainSettings {
            turn_max_deg: 180.0,
            turn_step_deg: 1.0,
            ..TrainSettings::default()
        };
        let wide = Gray::blank(3000, 3000, 100).unwrap();
        assert!(locate(&wide, &t, &all).unwrap_err().starts_with(TOO_BIG));
    }
}
