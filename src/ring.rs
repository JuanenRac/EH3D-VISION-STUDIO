//! Ring marks: a round mark with a hole in it (a donut), of one size or of two. The spots of the picture are cut as for the other marks; a spot is a ring when its pixels all lie in a band
//! between an inner and an outer radius around its centre, the band is full all the way round (a ring with a gap is not a ring), the hole is wide enough and not too wide, and the
//! outline is round. The two sizes are told apart by a diameter that is given or found from the diameters themselves: when they fall in two groups with a clear gap between them, the
//! small ones and the large ones are named. Each mark has the score of its shape and a confidence, which is that score lowered when the ring stands out only a little from the board.

use crate::fiducial::{otsu, Polarity};
use crate::image::Gray;
use crate::measure::{blobs, Blob};

/// The number of directions around the centre in which the band of a ring is checked.
const SECTORS: usize = 24;

/// What is asked of the search for rings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RingSettings {
    pub polarity: Polarity,
    /// `None` is the threshold of Otsu.
    pub threshold: Option<u8>,
    /// The range of the outer diameter, in pixels.
    pub min_diameter: f64,
    pub max_diameter: f64,
    /// The range of the inner diameter over the outer one: a thin ring has a ratio near 1, a thick one near 0.
    pub min_ratio: f64,
    pub max_ratio: f64,
    /// The least share of the band between the two radii that the pixels fill (0 to 1).
    pub min_fill: f64,
    /// How much longer than wide a ring may be (1 is a circle).
    pub max_elongation: f64,
    /// The most spots that are looked at; a picture with more is refused with `too_many_parts`.
    pub max_spots: usize,
    /// The contrast (0 to 1) from which the confidence of a ring is the score of its shape alone.
    pub full_contrast: f64,
    /// The outer diameter in pixels that tells the small rings from the large ones; 0 finds it from the diameters.
    pub split: f64,
    /// How much larger the smallest of the large rings has to be than the largest of the small ones (1.25 is a quarter larger) for two sizes to be told apart when `split` is 0.
    pub size_gap: f64,
}

impl Default for RingSettings {
    fn default() -> Self {
        RingSettings {
            polarity: Polarity::Dark,
            threshold: None,
            min_diameter: 12.0,
            max_diameter: 200.0,
            min_ratio: 0.25,
            max_ratio: 0.85,
            min_fill: 0.85,
            max_elongation: 1.25,
            max_spots: 1000,
            full_contrast: 0.3,
            split: 0.0,
            size_gap: 1.25,
        }
    }
}

/// Which size a ring has when the rings are of two sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeClass {
    /// All the rings are of one size (or there is only one).
    Single,
    Small,
    Large,
}

impl SizeClass {
    /// The word of the class as it is printed on the command line.
    pub fn word(self) -> &'static str {
        match self {
            SizeClass::Single => "single",
            SizeClass::Small => "small",
            SizeClass::Large => "large",
        }
    }
}

/// A ring mark.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ring {
    pub x: f64,
    pub y: f64,
    /// The outer and the inner diameter in pixels.
    pub outer: f64,
    pub inner: f64,
    /// 0 to 1, how much of its band the ring fills and how round its outline is.
    pub score: f64,
    /// 0 to 1: the score, lowered when the ring stands out only a little from the board.
    pub confidence: f64,
    pub size: SizeClass,
}

fn check(s: &RingSettings) -> Result<(), String> {
    let ok = s.min_diameter.is_finite()
        && s.max_diameter.is_finite()
        && s.min_diameter >= 8.0
        && s.max_diameter >= s.min_diameter
        && s.min_ratio.is_finite()
        && s.max_ratio.is_finite()
        && s.min_ratio > 0.0
        && s.max_ratio < 1.0
        && s.max_ratio >= s.min_ratio
        && s.min_fill.is_finite()
        && s.min_fill > 0.0
        && s.min_fill <= 1.0
        && s.max_elongation.is_finite()
        && s.max_elongation >= 1.0
        && s.max_spots > 0
        && s.full_contrast.is_finite()
        && s.full_contrast > 0.0
        && s.full_contrast <= 1.0
        && s.split.is_finite()
        && s.split >= 0.0
        && s.size_gap.is_finite()
        && s.size_gap > 1.0;
    if ok {
        Ok(())
    } else {
        Err("the settings of the search for rings make no sense (an outer diameter from 8 up, the largest not below the smallest, a ratio of the two diameters above 0 and below 1, a fill and a contrast above 0 and at most 1, an elongation of 1 or more, a size gap above 1)".into())
    }
}

/// The pixels of the spot, reached from the first pixel of its top row (the spot may be a ring, whose centre is not part of it). `None` when they are not as many as were counted.
fn pixels_of(img: &Gray, on: &dyn Fn(u8) -> bool, b: &Blob) -> Option<Vec<(usize, usize)>> {
    let (bw, bh) = (b.x1 - b.x0 + 1, b.y1 - b.y0 + 1);
    let sx = (b.x0..=b.x1).find(|&x| on(img.data[b.y0 * img.w + x]))?;
    let mut seen = vec![false; bw * bh];
    seen[sx - b.x0] = true;
    let mut stack = vec![(sx, b.y0)];
    let mut out = Vec::with_capacity(b.n as usize);
    while let Some((x, y)) = stack.pop() {
        out.push((x, y));
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

/// How far the ring stands out from the board around it, 0 to 1: the grey of the ring against the grey of a band just outside it. `None` when no board lies around it in the picture.
fn contrast_of(
    img: &Gray,
    pixels: &[(usize, usize)],
    centre: (f64, f64),
    outer_r: f64,
) -> Option<f64> {
    let ring_mean = pixels
        .iter()
        .map(|&(x, y)| f64::from(img.data[y * img.w + x]))
        .sum::<f64>()
        / pixels.len() as f64;
    let (lo, hi) = (outer_r + 2.0, outer_r * 1.5 + 2.0);
    let reach = hi.ceil() as i64;
    let (cx, cy) = (centre.0.round() as i64, centre.1.round() as i64);
    let (w, h) = (img.w as i64, img.h as i64);
    let (mut sum, mut n) = (0.0f64, 0u64);
    for y in (cy - reach).max(0)..=(cy + reach).min(h - 1) {
        for x in (cx - reach).max(0)..=(cx + reach).min(w - 1) {
            let d = ((x as f64 - centre.0).powi(2) + (y as f64 - centre.1).powi(2)).sqrt();
            if (lo..=hi).contains(&d) {
                sum += f64::from(img.data[y as usize * img.w + x as usize]);
                n += 1;
            }
        }
    }
    (n > 0).then(|| ((sum / n as f64 - ring_mean).abs() / 255.0).clamp(0.0, 1.0))
}

/// The rings of the picture, the best ones first, with the size class of each left at `Single` (see `classify`; `find_rings` does both). An image with a single grey level has none.
pub fn find_rings_unclassified(img: &Gray, s: &RingSettings) -> Result<Vec<Ring>, String> {
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
    // the area of a ring of the most extreme sizes bounds the number of pixels that a spot of the right size has
    let max_n = (s.max_diameter * s.max_diameter).ceil() as u64;
    let min_n =
        (std::f64::consts::PI * (s.min_diameter / 2.0).powi(2) * (1.0 - s.max_ratio.powi(2)) * 0.5)
            .floor()
            .max(8.0) as u64;
    let mut found = Vec::new();
    for b in blobs(img, &on, min_n, max_n, s.max_spots)? {
        if b.x0 == 0 || b.y0 == 0 || b.x1 + 1 >= img.w || b.y1 + 1 >= img.h {
            continue;
        }
        // the second moments of a ring are the same in every direction; a spot that is longer one way is not a mark
        let half = (b.mxx + b.myy) / 2.0;
        let spread = (((b.mxx - b.myy) / 2.0).powi(2) + b.mxy.powi(2)).sqrt();
        let (l1, l2) = (half + spread, half - spread);
        if l2 <= 0.0 || l1 / l2 > s.max_elongation * s.max_elongation {
            continue;
        }
        let Some(pixels) = pixels_of(img, &on, &b) else {
            continue;
        };
        let mut nearest = f64::INFINITY;
        let mut farthest = 0.0f64;
        let mut count = [0u32; SECTORS];
        let mut reach = [0.0f64; SECTORS];
        for &(x, y) in &pixels {
            let (dx, dy) = (x as f64 - b.cx, y as f64 - b.cy);
            let d = dx.hypot(dy);
            nearest = nearest.min(d);
            farthest = farthest.max(d);
            let angle = dy.atan2(dx) + std::f64::consts::PI;
            let sector =
                ((angle / (2.0 * std::f64::consts::PI) * SECTORS as f64) as usize).min(SECTORS - 1);
            count[sector] += 1;
            reach[sector] = reach[sector].max(d);
        }
        // the pixel centres lie half a pixel inside the edge of the spot
        let outer_r = farthest + 0.5;
        let inner_r = (nearest - 0.5).max(0.0);
        let (outer, inner) = (2.0 * outer_r, 2.0 * inner_r);
        if outer < s.min_diameter || outer > s.max_diameter {
            continue;
        }
        let ratio = inner / outer;
        if ratio < s.min_ratio || ratio > s.max_ratio {
            continue;
        }
        // the band must be full in every direction: a ring with a gap is not a ring
        if count.contains(&0) {
            continue;
        }
        let band = std::f64::consts::PI * (outer_r * outer_r - inner_r * inner_r);
        let fill = (pixels.len() as f64 / band).min(1.0);
        if fill < s.min_fill {
            continue;
        }
        let shortest = reach.iter().copied().fold(f64::INFINITY, f64::min) + 0.5;
        let roundness = (shortest / outer_r).clamp(0.0, 1.0);
        let score = (fill * roundness).clamp(0.0, 1.0);
        let confidence = match contrast_of(img, &pixels, (b.cx, b.cy), outer_r) {
            Some(c) => score * (c / s.full_contrast).min(1.0),
            None => 0.0,
        };
        found.push(Ring {
            x: b.cx,
            y: b.cy,
            outer,
            inner,
            score,
            confidence,
            size: SizeClass::Single,
        });
    }
    found.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(found)
}

/// The size class of each diameter. With `split` above 0, a diameter below it is small and one at or above it is large. With `split` at 0 the diameters are sorted and cut at the widest gap
/// between neighbours when the larger of the two is at least `size_gap` times the smaller (the cut is their geometric mean); otherwise, and with fewer than two diameters, all are `Single`.
pub fn classify(diameters: &[f64], split: f64, size_gap: f64) -> Vec<SizeClass> {
    let cut = if split > 0.0 {
        split
    } else {
        let mut sorted: Vec<f64> = diameters.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut best: Option<(f64, f64)> = None;
        for pair in sorted.windows(2) {
            let ratio = pair[1] / pair[0];
            if ratio >= size_gap && best.is_none_or(|(r, _)| ratio > r) {
                best = Some((ratio, (pair[0] * pair[1]).sqrt()));
            }
        }
        match best {
            Some((_, cut)) => cut,
            None => return vec![SizeClass::Single; diameters.len()],
        }
    };
    diameters
        .iter()
        .map(|&d| {
            if d < cut {
                SizeClass::Small
            } else {
                SizeClass::Large
            }
        })
        .collect()
}

/// The rings of the picture, the best ones first, each with its size class.
pub fn find_rings(img: &Gray, s: &RingSettings) -> Result<Vec<Ring>, String> {
    let mut rings = find_rings_unclassified(img, s)?;
    let diameters: Vec<f64> = rings.iter().map(|r| r.outer).collect();
    for (ring, class) in rings
        .iter_mut()
        .zip(classify(&diameters, s.split, s.size_gap))
    {
        ring.size = class;
    }
    Ok(rings)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ring of outer radius `outer` and inner radius `inner` at (cx, cy), dark on the board.
    fn ring(g: &mut Gray, cx: f64, cy: f64, outer: f64, inner: f64) {
        g.disc(cx, cy, outer, 30);
        g.disc(cx, cy, inner, 220);
    }

    fn board() -> Gray {
        Gray::blank(300, 160, 220).unwrap()
    }

    #[test]
    fn a_ring_is_found_with_its_centre_and_its_two_diameters() {
        let mut g = board();
        ring(&mut g, 80.3, 70.6, 20.0, 11.0);
        let found = find_rings(&g, &RingSettings::default()).unwrap();
        assert_eq!(found.len(), 1);
        let r = found[0];
        assert!(
            (r.x - 80.3).abs() < 0.3 && (r.y - 70.6).abs() < 0.3,
            "{r:?}"
        );
        assert!(
            (r.outer - 40.0).abs() < 2.0 && (r.inner - 22.0).abs() < 2.0,
            "{r:?}"
        );
        assert!(r.score > 0.8 && r.confidence > 0.8, "{r:?}");
        assert_eq!(r.size, SizeClass::Single);
    }

    #[test]
    fn a_disc_a_cut_ring_a_ring_with_a_gap_and_a_bad_ratio_are_not_rings() {
        let mut g = board();
        // a filled disc
        g.disc(40.0, 40.0, 18.0, 30);
        // a ring with a gap of 8 pixels in its side
        ring(&mut g, 120.0, 50.0, 20.0, 12.0);
        for y in 46..=54 {
            for x in 138..=142 {
                g.data[y * 300 + x] = 220;
            }
        }
        // a ring cut by the edge of the picture
        ring(&mut g, 4.0, 120.0, 20.0, 11.0);
        // a ring whose hole is tiny (a ratio of 0.1) and a ring that is nearly all hole (0.95)
        ring(&mut g, 200.0, 50.0, 20.0, 2.0);
        ring(&mut g, 260.0, 110.0, 20.0, 19.0);
        // a good one among them
        ring(&mut g, 150.0, 120.0, 18.0, 10.0);
        let found = find_rings(&g, &RingSettings::default()).unwrap();
        assert_eq!(found.len(), 1, "{found:?}");
        assert!((found[0].x - 150.0).abs() < 0.5);
        // wider limits for the ratio take in the thick and the thin ones
        let wide = RingSettings {
            min_ratio: 0.05,
            max_ratio: 0.97,
            min_fill: 0.5,
            ..RingSettings::default()
        };
        assert!(find_rings(&g, &wide).unwrap().len() >= 3);
    }

    #[test]
    fn rings_of_two_sizes_are_told_apart_and_one_size_is_single() {
        let mut g = board();
        ring(&mut g, 40.0, 40.0, 14.0, 7.0);
        ring(&mut g, 100.0, 40.0, 15.0, 8.0);
        ring(&mut g, 160.0, 90.0, 28.0, 15.0);
        ring(&mut g, 240.0, 90.0, 29.0, 16.0);
        let found = find_rings(&g, &RingSettings::default()).unwrap();
        assert_eq!(found.len(), 4);
        for r in &found {
            let expected = if r.outer < 40.0 {
                SizeClass::Small
            } else {
                SizeClass::Large
            };
            assert_eq!(r.size, expected, "{r:?}");
        }
        // a split that is given wins over the one that was found
        let by_hand = RingSettings {
            split: 40.0,
            ..RingSettings::default()
        };
        let sizes: Vec<SizeClass> = find_rings(&g, &by_hand)
            .unwrap()
            .iter()
            .map(|r| r.size)
            .collect();
        assert_eq!(sizes.iter().filter(|s| **s == SizeClass::Small).count(), 2);
        // a gap that is asked to be wider than there is leaves them single
        let strict = RingSettings {
            size_gap: 3.0,
            ..RingSettings::default()
        };
        assert!(find_rings(&g, &strict)
            .unwrap()
            .iter()
            .all(|r| r.size == SizeClass::Single));
        // rings of one size
        let mut one = board();
        ring(&mut one, 60.0, 60.0, 20.0, 11.0);
        ring(&mut one, 160.0, 60.0, 21.0, 11.0);
        assert!(find_rings(&one, &RingSettings::default())
            .unwrap()
            .iter()
            .all(|r| r.size == SizeClass::Single));
    }

    #[test]
    fn the_classes_follow_the_widest_gap_between_the_diameters() {
        use SizeClass::{Large, Single, Small};
        assert_eq!(classify(&[], 0.0, 1.25), Vec::<SizeClass>::new());
        assert_eq!(classify(&[30.0], 0.0, 1.25), vec![Single]);
        assert_eq!(classify(&[30.0, 31.0, 32.0], 0.0, 1.25), vec![Single; 3]);
        assert_eq!(
            classify(&[60.0, 30.0, 31.0, 62.0], 0.0, 1.25),
            vec![Large, Small, Small, Large]
        );
        // three groups: the widest gap decides
        assert_eq!(
            classify(&[20.0, 30.0, 90.0], 0.0, 1.25),
            vec![Small, Small, Large]
        );
        assert_eq!(
            classify(&[20.0, 30.0, 90.0], 25.0, 1.25),
            vec![Small, Large, Large]
        );
        assert_eq!(Small.word(), "small");
        assert_eq!(Large.word(), "large");
        assert_eq!(Single.word(), "single");
    }

    #[test]
    fn light_rings_on_a_dark_board_are_found_with_the_polarity() {
        let mut g = Gray::blank(200, 120, 30).unwrap();
        g.disc(100.0, 60.0, 20.0, 220);
        g.disc(100.0, 60.0, 11.0, 30);
        assert!(find_rings(&g, &RingSettings::default()).unwrap().is_empty());
        let light = RingSettings {
            polarity: Polarity::Light,
            ..RingSettings::default()
        };
        let found = find_rings(&g, &light).unwrap();
        assert_eq!(found.len(), 1);
        assert!((found[0].x - 100.0).abs() < 0.3);
    }

    #[test]
    fn a_ring_that_stands_out_little_is_trusted_less() {
        let make = |ring_value: u8| {
            let mut g = Gray::blank(200, 120, 220).unwrap();
            g.disc(100.0, 60.0, 20.0, ring_value);
            g.disc(100.0, 60.0, 11.0, 220);
            // a fixed threshold so that the weak ring is still cut out
            let s = RingSettings {
                threshold: Some(ring_value + (220 - ring_value) / 2),
                ..RingSettings::default()
            };
            find_rings(&g, &s).unwrap()[0]
        };
        let strong = make(30);
        let weak = make(205);
        assert!((strong.score - weak.score).abs() < 0.05);
        assert!(strong.confidence > 0.8, "{strong:?}");
        assert!(weak.confidence < 0.25, "{weak:?}");
    }

    #[test]
    fn what_makes_no_sense_or_is_too_much_is_refused() {
        let g = board();
        for bad in [
            RingSettings {
                min_diameter: 4.0,
                ..RingSettings::default()
            },
            RingSettings {
                min_ratio: 0.0,
                ..RingSettings::default()
            },
            RingSettings {
                max_ratio: 1.0,
                ..RingSettings::default()
            },
            RingSettings {
                min_ratio: 0.9,
                max_ratio: 0.5,
                ..RingSettings::default()
            },
            RingSettings {
                size_gap: 1.0,
                ..RingSettings::default()
            },
            RingSettings {
                split: -1.0,
                ..RingSettings::default()
            },
            RingSettings {
                max_spots: 0,
                ..RingSettings::default()
            },
        ] {
            assert!(find_rings(&g, &bad).is_err(), "{bad:?}");
        }
        // a flat picture has no rings
        assert!(find_rings(&board(), &RingSettings::default())
            .unwrap()
            .is_empty());
        // more spots of the size of a ring than are allowed
        let mut many = board();
        for i in 0..6 {
            ring(&mut many, 25.0 + 45.0 * i as f64, 40.0, 14.0, 7.0);
        }
        let few = RingSettings {
            max_spots: 4,
            ..RingSettings::default()
        };
        assert!(find_rings(&many, &few)
            .unwrap_err()
            .starts_with("too_many_parts"));
        assert_eq!(
            find_rings(&many, &RingSettings::default()).unwrap().len(),
            6
        );
    }

    #[test]
    fn a_ring_that_is_not_round_is_left_out() {
        let mut g = board();
        // an ellipse of 60 by 30 pixels with a hole
        for y in 20..110usize {
            for x in 20..140usize {
                let (dx, dy) = (x as f64 - 80.0, y as f64 - 65.0);
                let r = (dx / 30.0).powi(2) + (dy / 15.0).powi(2);
                if (0.25..=1.0).contains(&r) {
                    g.data[y * 300 + x] = 30;
                }
            }
        }
        assert!(find_rings(&g, &RingSettings::default()).unwrap().is_empty());
    }
}
