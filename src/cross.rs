//! Cross-shaped fiducial marks (a plus sign, with its arms along the axes of the picture): the picture is cut in dark and light, the connected spots are measured, and the ones whose
//! shape is a cross are kept. A spot is a cross when its box is square, its area is what two arms of one width give, the middle row and the middle column of the box are full, and the four
//! corners of the box are empty. The centre of a cross is the centre of its pixels, so it is as good as the centre of a disc.

use crate::fiducial::{otsu, Polarity};
use crate::image::Gray;
use crate::measure::blobs;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrossSettings {
    pub polarity: Polarity,
    /// `None` is the threshold of Otsu.
    pub threshold: Option<u8>,
    /// The range of the side of the box of a cross, in pixels.
    pub min_size: f64,
    pub max_size: f64,
    /// How much longer than wide the box of a cross may be (1 is a square).
    pub max_elongation: f64,
    /// The most spots that are looked at; a picture with more is refused.
    pub max_spots: usize,
}

impl Default for CrossSettings {
    fn default() -> Self {
        CrossSettings {
            polarity: Polarity::Dark,
            threshold: None,
            min_size: 12.0,
            max_size: 200.0,
            max_elongation: 1.2,
            max_spots: 1000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cross {
    pub x: f64,
    pub y: f64,
    /// The side of the box that holds the cross.
    pub size: f64,
    /// The width of an arm, worked out from the area.
    pub arm: f64,
    /// 0 to 1, how much it looks like a cross.
    pub score: f64,
}

/// The crosses of a picture, the best first. A spot cut by the edge of the picture is left out, a picture with a single grey level has none, and more than `max_spots` spots of the size
/// of a cross is an error with the code `too_many_parts`.
pub fn find_crosses(img: &Gray, s: &CrossSettings) -> Result<Vec<Cross>, String> {
    if !(s.min_size.is_finite()
        && s.max_size.is_finite()
        && s.min_size >= 7.0
        && s.max_size >= s.min_size
        && s.max_elongation.is_finite()
        && s.max_elongation >= 1.0
        && s.max_spots > 0)
    {
        return Err("the settings of the search for crosses make no sense (a size from 7 up, the largest not below the smallest, an elongation of 1 or more)".into());
    }
    if img.data.iter().all(|&v| v == img.data[0]) {
        return Ok(Vec::new());
    }
    let t = s.threshold.unwrap_or_else(|| otsu(img));
    let polarity = s.polarity;
    let on = move |v: u8| match polarity {
        Polarity::Dark => v <= t,
        Polarity::Light => v > t,
    };
    let (w, h) = (img.w, img.h);
    // a cross of side L with an arm of one pixel has 2L - 1 pixels; a square of side L has L * L
    let min_n = (2.0 * s.min_size - 1.0).floor() as u64;
    let max_n = (s.max_size * s.max_size).ceil() as u64;
    let mut found = Vec::new();
    for b in blobs(img, &on, min_n, max_n, s.max_spots)? {
        let (bw, bh) = ((b.x1 - b.x0 + 1) as f64, (b.y1 - b.y0 + 1) as f64);
        if b.x0 == 0 || b.y0 == 0 || b.x1 + 1 >= w || b.y1 + 1 >= h {
            continue;
        }
        if bw.min(bh) < s.min_size || bw.max(bh) > s.max_size {
            continue;
        }
        let elongation = bw.max(bh) / bw.min(bh);
        if elongation > s.max_elongation {
            continue;
        }
        // the width of an arm: a cross of side L and arm a has 2 * L * a - a * a pixels
        let side = (bw + bh) / 2.0;
        let rest = side * side - b.n as f64;
        if rest < 0.0 {
            continue;
        }
        let arm = side - rest.sqrt();
        if arm < 1.0 || arm > 0.48 * side {
            continue;
        }
        let (mx, my) = (b.cx.round() as usize, b.cy.round() as usize);
        if mx < b.x0 || mx > b.x1 || my < b.y0 || my > b.y1 {
            continue;
        }
        let row = (b.x0..=b.x1).filter(|&x| on(img.data[my * w + x])).count() as f64 / bw;
        let column = (b.y0..=b.y1).filter(|&y| on(img.data[y * w + mx])).count() as f64 / bh;
        if row < 0.9 || column < 0.9 {
            continue;
        }
        // the four corners of the box, a little smaller than the space between the arms, are empty
        let corner = ((side - arm) / 2.0).floor() as usize;
        if corner < 2 {
            continue;
        }
        let side_px = corner - 1;
        let mut empty = 1.0f64;
        for (cx0, cy0) in [
            (b.x0, b.y0),
            (b.x1 + 1 - side_px, b.y0),
            (b.x0, b.y1 + 1 - side_px),
            (b.x1 + 1 - side_px, b.y1 + 1 - side_px),
        ] {
            let mut off = 0usize;
            for y in cy0..cy0 + side_px {
                for x in cx0..cx0 + side_px {
                    if !on(img.data[y * w + x]) {
                        off += 1;
                    }
                }
            }
            empty = empty.min(off as f64 / (side_px * side_px) as f64);
        }
        if empty < 0.95 {
            continue;
        }
        // a disc works out to an arm about half the side and has pixels between the arms, so the arm may be at most 0.48 of the side (above) and nothing may lie beyond the arms
        let beyond = arm / 2.0 + 1.0;
        let mut stray = 0usize;
        for y in b.y0..=b.y1 {
            for x in b.x0..=b.x1 {
                if on(img.data[y * w + x])
                    && (x as f64 - b.cx).abs() > beyond
                    && (y as f64 - b.cy).abs() > beyond
                {
                    stray += 1;
                }
            }
        }
        if stray as f64 > 1.0 + 0.005 * b.n as f64 {
            continue;
        }
        found.push(Cross {
            x: b.cx,
            y: b.cy,
            size: side,
            arm,
            score: (row.min(column) * empty / elongation).clamp(0.0, 1.0),
        });
    }
    found.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fiducial::{find, Settings};

    /// Paints a cross of side `2 * half + 1` with arms of width `2 * arm_half + 1` centred at the pixel (cx, cy).
    fn cross(g: &mut Gray, cx: usize, cy: usize, half: usize, arm_half: usize, value: u8) {
        for dy in 0..=2 * half {
            for dx in 0..=2 * half {
                let (ox, oy) = (dx.abs_diff(half), dy.abs_diff(half));
                if ox <= arm_half || oy <= arm_half {
                    g.data[(cy + dy - half) * g.w + cx + dx - half] = value;
                }
            }
        }
    }

    #[test]
    fn crosses_are_found_with_their_centre_size_and_arm() {
        let mut g = Gray::blank(240, 140, 220).unwrap();
        cross(&mut g, 60, 50, 15, 2, 30);
        cross(&mut g, 160, 90, 20, 3, 30);
        let found = find_crosses(&g, &CrossSettings::default()).unwrap();
        assert_eq!(found.len(), 2, "{found:?}");
        for c in &found {
            let (ex, ey, size, arm) = if c.x < 100.0 {
                (60.0, 50.0, 31.0, 5.0)
            } else {
                (160.0, 90.0, 41.0, 7.0)
            };
            assert!((c.x - ex).abs() < 1e-6 && (c.y - ey).abs() < 1e-6, "{c:?}");
            assert!(
                (c.size - size).abs() < 1e-9 && (c.arm - arm).abs() < 0.01,
                "{c:?}"
            );
            assert!(c.score > 0.95, "{c:?}");
        }
    }

    #[test]
    fn light_crosses_and_a_shifted_centre_are_found_too() {
        let mut g = Gray::blank(200, 120, 30).unwrap();
        cross(&mut g, 70, 40, 14, 2, 230);
        cross(&mut g, 150, 80, 14, 2, 230);
        // a pixel of the arm is missing: the centre moves a little and the cross is still found
        g.data[40 * 200 + 70 - 14] = 30;
        let s = CrossSettings {
            polarity: Polarity::Light,
            ..CrossSettings::default()
        };
        let found = find_crosses(&g, &s).unwrap();
        assert_eq!(found.len(), 2, "{found:?}");
        // the same picture has no dark crosses
        assert!(find_crosses(&g, &CrossSettings::default())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn a_disc_a_square_a_line_a_turned_cross_and_a_cut_cross_are_not_crosses() {
        let mut g = Gray::blank(300, 200, 220).unwrap();
        g.disc(40.0, 40.0, 15.0, 30);
        for y in 80..110 {
            for x in 20..50 {
                g.data[y * 300 + x] = 30;
            }
        }
        for x in 100..140 {
            g.data[50 * 300 + x] = 30;
        }
        // a cross turned by 45 degrees: two diagonal bands
        for d in 0..31usize {
            for k in 0..3usize {
                g.data[(40 + d) * 300 + 200 + d + k] = 30;
                g.data[(40 + d) * 300 + 230 - d + k] = 30;
            }
        }
        // a cross that touches the edge of the picture
        cross(&mut g, 100, 100, 15, 2, 30);
        for y in 85..116 {
            g.data[y * 300 + 85] = 220;
        }
        cross(&mut g, 15, 150, 15, 2, 30);
        let found = find_crosses(&g, &CrossSettings::default()).unwrap();
        // the only cross that stays is the one at (100, 100), where one column of the left arm was taken away: it is still a cross
        assert_eq!(found.len(), 1, "{found:?}");
        assert!((found[0].x - 100.0).abs() < 0.6 && (found[0].y - 100.0).abs() < 0.6);
    }

    #[test]
    fn a_disc_of_the_size_of_a_cross_is_not_a_cross_whatever_its_size() {
        for (i, radius) in [8.0, 10.0, 12.0, 14.5, 18.0, 25.0].iter().enumerate() {
            let mut g = Gray::blank(200, 200, 220).unwrap();
            g.disc(100.0 + i as f64 * 0.4, 100.0, *radius, 30);
            assert!(
                find_crosses(&g, &CrossSettings::default())
                    .unwrap()
                    .is_empty(),
                "radius {radius}"
            );
        }
    }

    #[test]
    fn a_cross_is_not_a_round_mark_and_a_disc_is_not_a_cross() {
        let mut g = Gray::blank(200, 100, 220).unwrap();
        cross(&mut g, 50, 50, 15, 2, 30);
        g.disc(140.0, 50.0, 15.0, 30);
        let crosses = find_crosses(&g, &CrossSettings::default()).unwrap();
        let discs = find(&g, &Settings::default()).unwrap();
        assert_eq!(crosses.len(), 1);
        assert_eq!(discs.len(), 1);
        assert!(crosses[0].x < 100.0 && discs[0].x > 100.0);
    }

    #[test]
    fn the_size_range_and_the_settings_are_checked() {
        let mut g = Gray::blank(240, 140, 220).unwrap();
        cross(&mut g, 60, 50, 15, 2, 30);
        let big = CrossSettings {
            min_size: 40.0,
            ..CrossSettings::default()
        };
        assert!(find_crosses(&g, &big).unwrap().is_empty());
        let small = CrossSettings {
            max_size: 20.0,
            ..CrossSettings::default()
        };
        assert!(find_crosses(&g, &small).unwrap().is_empty());
        for bad in [
            CrossSettings {
                min_size: 3.0,
                ..CrossSettings::default()
            },
            CrossSettings {
                max_size: 5.0,
                ..CrossSettings::default()
            },
            CrossSettings {
                max_elongation: 0.5,
                ..CrossSettings::default()
            },
            CrossSettings {
                max_spots: 0,
                ..CrossSettings::default()
            },
            CrossSettings {
                min_size: f64::NAN,
                ..CrossSettings::default()
            },
        ] {
            assert!(find_crosses(&g, &bad).is_err(), "{bad:?}");
        }
        let flat = Gray::blank(50, 50, 100).unwrap();
        assert!(find_crosses(&flat, &CrossSettings::default())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn too_many_spots_of_the_size_of_a_cross_are_refused() {
        let mut g = Gray::blank(300, 60, 220).unwrap();
        for i in 0..6 {
            cross(&mut g, 30 + i * 45, 30, 14, 2, 30);
        }
        let s = CrossSettings {
            max_spots: 4,
            ..CrossSettings::default()
        };
        assert!(find_crosses(&g, &s)
            .unwrap_err()
            .starts_with("too_many_parts"));
        assert_eq!(
            find_crosses(&g, &CrossSettings::default()).unwrap().len(),
            6
        );
    }
}
