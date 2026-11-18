//! Uneven light: a ring light that is brighter on one side, a shadow or a lamp that fades across the board make one threshold for the whole picture fail (the dark side is all "mark").
//! The picture is divided by the light around each pixel (the mean of a square window, from an integral image), so the board comes out about the same grey everywhere and the marks stand out from it.

use crate::image::Gray;

/// The smallest window that is accepted, in pixels (a mark must be much smaller than the window).
pub const MIN_WINDOW: usize = 3;
/// The largest window that is accepted, in pixels.
pub const MAX_WINDOW: usize = 4001;
/// The grey the board comes out as.
const BOARD_LEVEL: f64 = 190.0;

/// The picture with the uneven light taken out. `window` is the side of the square, in pixels, over which the light is averaged: about three times the largest mark or more. Pixels
/// near the edge use the part of the window that is inside the picture.
pub fn flatten(img: &Gray, window: usize) -> Result<Gray, String> {
    if !(MIN_WINDOW..=MAX_WINDOW).contains(&window) {
        return Err(format!(
            "the window of the light must be from {MIN_WINDOW} to {MAX_WINDOW} pixels, not {window}"
        ));
    }
    let (w, h) = (img.w, img.h);
    // sums[(y * (w + 1)) + x] is the sum of the pixels above and to the left of (x, y)
    let mut sums = vec![0u64; (w + 1) * (h + 1)];
    for y in 0..h {
        let mut row = 0u64;
        for x in 0..w {
            row += u64::from(img.data[y * w + x]);
            sums[(y + 1) * (w + 1) + x + 1] = sums[y * (w + 1) + x + 1] + row;
        }
    }
    let r = window / 2;
    let mut data = Vec::with_capacity(w * h);
    for y in 0..h {
        let (y0, y1) = (y.saturating_sub(r), (y + r + 1).min(h));
        for x in 0..w {
            let (x0, x1) = (x.saturating_sub(r), (x + r + 1).min(w));
            let total = sums[y1 * (w + 1) + x1] + sums[y0 * (w + 1) + x0]
                - sums[y0 * (w + 1) + x1]
                - sums[y1 * (w + 1) + x0];
            let area = ((x1 - x0) * (y1 - y0)) as f64;
            let light = (total as f64 / area).max(1.0);
            let level = f64::from(img.data[y * w + x]) * BOARD_LEVEL / light;
            data.push(level.round().clamp(0.0, 255.0) as u8);
        }
    }
    Gray::new(w, h, data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fiducial::{find, Polarity, Settings};

    /// A board whose light fades from 250 at the left to about 60 at the right, with dark marks that are 40 % of the light where they lie.
    fn faded_board(marks: &[(f64, f64)]) -> Gray {
        let (w, h) = (400usize, 120usize);
        let light = |x: f64| 250.0 - x * 190.0 / w as f64;
        let mut data = Vec::with_capacity(w * h);
        for _ in 0..h {
            for x in 0..w {
                data.push(light(x as f64) as u8);
            }
        }
        let mut g = Gray::new(w, h, data).unwrap();
        for &(x, y) in marks {
            g.disc(x, y, 10.0, (0.4 * light(x)) as u8);
        }
        g
    }

    #[test]
    fn a_fade_of_the_light_defeats_one_threshold_and_the_flat_picture_finds_every_mark() {
        let marks = [(60.0, 60.0), (190.0, 60.0), (330.0, 60.0)];
        let img = faded_board(&marks);
        let s = Settings::default();
        let plain = find(&img, &s).unwrap();
        assert!(plain.len() < marks.len(), "{plain:?}");
        let flat = flatten(&img, 61).unwrap();
        let found = find(&flat, &s).unwrap();
        assert_eq!(found.len(), marks.len(), "{found:?}");
        for (x, y) in marks {
            assert!(
                found
                    .iter()
                    .any(|f| (f.x - x).abs() < 0.6 && (f.y - y).abs() < 0.6),
                "no mark at {x},{y}: {found:?}"
            );
        }
    }

    #[test]
    fn light_marks_on_a_dark_board_are_found_after_flattening() {
        let (w, h) = (300usize, 100usize);
        let mut data = Vec::new();
        for _ in 0..h {
            for x in 0..w {
                data.push((20.0 + x as f64 * 0.2) as u8);
            }
        }
        let mut g = Gray::new(w, h, data).unwrap();
        g.disc(60.0, 50.0, 9.0, 200);
        g.disc(240.0, 50.0, 9.0, 250);
        let s = Settings {
            polarity: Polarity::Light,
            ..Settings::default()
        };
        let found = find(&flatten(&g, 61).unwrap(), &s).unwrap();
        assert_eq!(found.len(), 2, "{found:?}");
    }

    #[test]
    fn a_flat_board_stays_flat_and_the_marks_keep_their_place() {
        let mut g = Gray::blank(100, 80, 200).unwrap();
        g.disc(50.0, 40.0, 9.0, 30);
        let flat = flatten(&g, 41).unwrap();
        // far from the mark the board comes out at the same grey everywhere
        assert_eq!(flat.at(5, 5), flat.at(95, 75));
        assert!(flat.at(50, 40) < 60);
        let found = find(&flat, &Settings::default()).unwrap();
        assert_eq!(found.len(), 1);
        assert!((found[0].x - 50.0).abs() < 0.3 && (found[0].y - 40.0).abs() < 0.3);
    }

    #[test]
    fn a_window_that_makes_no_sense_is_refused_and_small_pictures_do_not_panic() {
        let g = Gray::blank(10, 10, 100).unwrap();
        for bad in [0, 1, 2, MAX_WINDOW + 1] {
            let why = flatten(&g, bad).unwrap_err();
            assert!(why.contains(&bad.to_string()), "{why}");
        }
        // a window larger than the picture is the mean of the whole picture
        let flat = flatten(&g, 4001).unwrap();
        assert!(flat.data.iter().all(|&v| v == 190));
        let one = Gray::blank(1, 1, 0).unwrap();
        assert_eq!(flatten(&one, 3).unwrap().data, vec![0]);
    }
}
