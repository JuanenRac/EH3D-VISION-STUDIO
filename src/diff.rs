//! Two pictures of the same place compared: the one before and the one after (a pad before the paste and after it, a tray before the part was picked and after it, a board before and after
//! a part was placed). Every pixel whose grey level differs by more than a threshold is changed; the result says how many, how much, where (the box around them) and in how many
//! separate regions, and can give the picture of the changes. The pictures must be the same size and are expected to be taken without the camera or the board having moved.

use crate::image::Gray;
use crate::measure::{blobs, fixed};

/// The code of the refusal of two pictures of different sizes.
pub const SIZE_MISMATCH: &str = "size_mismatch";
/// The line of column names of the CSV file.
pub const CSV_HEADER: &str =
    "changed_px,total_px,changed_share,mean_abs,max_abs,regions,x0,y0,x1,y1,verdict";
/// The name of the format and its version, in the JSON file.
pub const FORMAT: &str = "eh3d-vision-diff";
pub const FORMAT_VERSION: u32 = 1;
/// The most separate regions that are counted; a picture with more is refused.
pub const MAX_REGIONS: usize = 100_000;

/// What is asked of the comparison.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffSettings {
    /// A pixel is changed when its grey level differs by more than this (0 to 254).
    pub threshold: u8,
    /// The fewest pixels a region of changes has to be counted.
    pub min_area: u64,
    /// The largest share of the pixels that may be changed for the pictures to count as the same (0 to 1).
    pub max_changed: f64,
}

impl Default for DiffSettings {
    fn default() -> Self {
        DiffSettings {
            threshold: 25,
            min_area: 4,
            max_changed: 0.01,
        }
    }
}

/// What differs between two pictures.
#[derive(Debug, Clone, PartialEq)]
pub struct Diff {
    pub changed_px: u64,
    pub total_px: u64,
    /// `changed_px` over `total_px`.
    pub changed_share: f64,
    /// The mean and the largest absolute difference of the grey levels, over all the pixels.
    pub mean_abs: f64,
    pub max_abs: u8,
    /// How many regions of at least `min_area` changed pixels (four neighbours) there are.
    pub regions: usize,
    /// The box around every changed pixel, `(x0, y0, x1, y1)` with both corners included; `None` when nothing changed.
    pub bbox: Option<(usize, usize, usize, usize)>,
    /// Whether the share of changed pixels is within `max_changed`.
    pub same: bool,
}

/// Whether the settings make sense; the reason when they do not.
pub fn check(s: &DiffSettings) -> Result<(), String> {
    if s.threshold == 255 {
        return Err("the threshold of a difference must be from 0 to 254".into());
    }
    if s.min_area == 0 {
        return Err("the least area of a region of changes must be 1 or more".into());
    }
    if !(0.0..=1.0).contains(&s.max_changed) {
        return Err("the largest share of changed pixels must be from 0 to 1".into());
    }
    Ok(())
}

/// The picture of the changes (255 where a pixel changed, 0 where it did not) with the same size as the two pictures.
pub fn mask(a: &Gray, b: &Gray, s: &DiffSettings) -> Result<Gray, String> {
    check(s)?;
    if (a.w, a.h) != (b.w, b.h) || a.data.len() != b.data.len() {
        return Err(format!(
            "{SIZE_MISMATCH}: the pictures are {} by {} and {} by {} pixels",
            a.w, a.h, b.w, b.h
        ));
    }
    let data = a
        .data
        .iter()
        .zip(&b.data)
        .map(|(p, q)| if p.abs_diff(*q) > s.threshold { 255 } else { 0 })
        .collect();
    Gray::new(a.w, a.h, data)
}

/// The comparison of two pictures of the same size.
pub fn compare(a: &Gray, b: &Gray, s: &DiffSettings) -> Result<Diff, String> {
    let changes = mask(a, b, s)?;
    let total_px = (a.w * a.h) as u64;
    let mut sum = 0u64;
    let mut max_abs = 0u8;
    for (p, q) in a.data.iter().zip(&b.data) {
        let d = p.abs_diff(*q);
        sum += u64::from(d);
        max_abs = max_abs.max(d);
    }
    let mut changed_px = 0u64;
    let mut bbox: Option<(usize, usize, usize, usize)> = None;
    for (i, v) in changes.data.iter().enumerate() {
        if *v == 0 {
            continue;
        }
        changed_px += 1;
        let (x, y) = (i % a.w, i / a.w);
        bbox = Some(match bbox {
            None => (x, y, x, y),
            Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
        });
    }
    let regions = if changed_px == 0 {
        0
    } else {
        blobs(&changes, &|v| v > 0, s.min_area, u64::MAX, MAX_REGIONS)?.len()
    };
    let changed_share = changed_px as f64 / total_px as f64;
    Ok(Diff {
        changed_px,
        total_px,
        changed_share,
        mean_abs: sum as f64 / total_px as f64,
        max_abs,
        regions,
        bbox,
        same: changed_share <= s.max_changed,
    })
}

fn verdict(d: &Diff) -> &'static str {
    if d.same {
        "same"
    } else {
        "changed"
    }
}

/// The result as CSV: the line `CSV_HEADER` and one line with the numbers and the verdict (`same` or `changed`); the box is empty when nothing changed.
pub fn to_csv(d: &Diff) -> String {
    let b = d.bbox.map_or(",,,".to_string(), |(x0, y0, x1, y1)| {
        format!("{x0},{y0},{x1},{y1}")
    });
    format!(
        "{CSV_HEADER}\n{},{},{},{},{},{},{},{}\n",
        d.changed_px,
        d.total_px,
        fixed(d.changed_share, 6),
        fixed(d.mean_abs, 4),
        d.max_abs,
        d.regions,
        b,
        verdict(d)
    )
}

/// The result as JSON: an object with `format`, `version`, the numbers, the `bbox` (an object, or `null` when nothing changed) and the `verdict`.
pub fn to_json(d: &Diff) -> String {
    let b = d.bbox.map_or("null".to_string(), |(x0, y0, x1, y1)| {
        format!("{{\"x0\": {x0}, \"y0\": {y0}, \"x1\": {x1}, \"y1\": {y1}}}")
    });
    format!(
        "{{\n  \"format\": \"{FORMAT}\",\n  \"version\": {FORMAT_VERSION},\n  \"changed_px\": {},\n  \"total_px\": {},\n  \"changed_share\": {},\n  \"mean_abs\": {},\n  \"max_abs\": {},\n  \"regions\": {},\n  \"bbox\": {},\n  \"verdict\": \"{}\"\n}}\n",
        d.changed_px,
        d.total_px,
        fixed(d.changed_share, 6),
        fixed(d.mean_abs, 4),
        d.max_abs,
        d.regions,
        b,
        verdict(d)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board() -> Gray {
        Gray::blank(40, 30, 200).unwrap()
    }

    fn with_square(mut g: Gray, x0: usize, y0: usize, side: usize, v: u8) -> Gray {
        for y in y0..y0 + side {
            for x in x0..x0 + side {
                g.data[y * 40 + x] = v;
            }
        }
        g
    }

    #[test]
    fn the_same_picture_has_no_changes() {
        let a = with_square(board(), 5, 5, 6, 40);
        let d = compare(&a, &a.clone(), &DiffSettings::default()).unwrap();
        assert_eq!((d.changed_px, d.regions, d.max_abs), (0, 0, 0));
        assert_eq!(d.bbox, None);
        assert!(d.same);
        assert_eq!(d.mean_abs, 0.0);
    }

    #[test]
    fn a_part_that_appears_is_a_region_with_its_box() {
        let before = board();
        let after = with_square(board(), 10, 8, 6, 40);
        let d = compare(&before, &after, &DiffSettings::default()).unwrap();
        assert_eq!(d.changed_px, 36);
        assert_eq!(d.regions, 1);
        assert_eq!(d.bbox, Some((10, 8, 15, 13)));
        assert_eq!(d.max_abs, 160);
        assert!((d.changed_share - 36.0 / 1200.0).abs() < 1e-12);
        assert!(!d.same);
        // the other way round is the same difference
        assert_eq!(
            compare(&after, &before, &DiffSettings::default()).unwrap(),
            d
        );
    }

    #[test]
    fn two_parts_are_two_regions_and_a_speck_is_left_out() {
        let after = with_square(
            with_square(with_square(board(), 3, 3, 5, 40), 25, 15, 6, 40),
            35,
            2,
            1,
            0,
        );
        let d = compare(&board(), &after, &DiffSettings::default()).unwrap();
        assert_eq!(d.changed_px, 25 + 36 + 1);
        assert_eq!(d.regions, 2);
        let strict = DiffSettings {
            min_area: 1,
            ..DiffSettings::default()
        };
        assert_eq!(compare(&board(), &after, &strict).unwrap().regions, 3);
    }

    #[test]
    fn a_small_change_of_light_is_below_the_threshold() {
        let mut lit = board();
        for v in lit.data.iter_mut() {
            *v = 215;
        }
        let d = compare(&board(), &lit, &DiffSettings::default()).unwrap();
        assert_eq!(d.changed_px, 0);
        assert_eq!(d.max_abs, 15);
        assert!((d.mean_abs - 15.0).abs() < 1e-12);
        let sharp = DiffSettings {
            threshold: 10,
            ..DiffSettings::default()
        };
        assert_eq!(compare(&board(), &lit, &sharp).unwrap().changed_px, 1200);
    }

    #[test]
    fn the_share_that_is_allowed_decides_the_verdict() {
        let after = with_square(board(), 10, 8, 6, 40);
        let loose = DiffSettings {
            max_changed: 0.05,
            ..DiffSettings::default()
        };
        assert!(compare(&board(), &after, &loose).unwrap().same);
        let none = DiffSettings {
            max_changed: 0.0,
            ..DiffSettings::default()
        };
        assert!(!compare(&board(), &after, &none).unwrap().same);
        assert!(compare(&board(), &board(), &none).unwrap().same);
    }

    #[test]
    fn the_mask_is_the_picture_of_the_changes() {
        let after = with_square(board(), 10, 8, 6, 40);
        let m = mask(&board(), &after, &DiffSettings::default()).unwrap();
        assert_eq!((m.w, m.h), (40, 30));
        assert_eq!(m.data.iter().filter(|v| **v == 255).count(), 36);
        assert_eq!(m.at(10, 8), 255);
        assert_eq!(m.at(9, 8), 0);
    }

    #[test]
    fn pictures_of_different_sizes_and_bad_settings_are_refused() {
        let small = Gray::blank(10, 10, 0).unwrap();
        assert!(compare(&board(), &small, &DiffSettings::default())
            .unwrap_err()
            .starts_with(SIZE_MISMATCH));
        // the same number of pixels in another shape is also different
        let tall = Gray::blank(30, 40, 200).unwrap();
        assert!(mask(&board(), &tall, &DiffSettings::default()).is_err());
        for bad in [
            DiffSettings {
                threshold: 255,
                ..DiffSettings::default()
            },
            DiffSettings {
                min_area: 0,
                ..DiffSettings::default()
            },
            DiffSettings {
                max_changed: 1.5,
                ..DiffSettings::default()
            },
            DiffSettings {
                max_changed: -0.1,
                ..DiffSettings::default()
            },
            DiffSettings {
                max_changed: f64::NAN,
                ..DiffSettings::default()
            },
        ] {
            assert!(check(&bad).is_err(), "{bad:?}");
            assert!(compare(&board(), &board(), &bad).is_err());
        }
    }

    #[test]
    fn the_files_have_the_numbers_the_box_and_the_verdict() {
        let after = with_square(board(), 10, 8, 6, 40);
        let d = compare(&board(), &after, &DiffSettings::default()).unwrap();
        let csv = to_csv(&d);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], CSV_HEADER);
        assert_eq!(lines[1].split(',').count(), CSV_HEADER.split(',').count());
        assert!(lines[1].starts_with("36,1200,0.030000,"));
        assert!(lines[1].ends_with(",1,10,8,15,13,changed"));
        let json = to_json(&d);
        assert!(json.contains("\"format\": \"eh3d-vision-diff\""));
        assert!(json.contains("\"bbox\": {\"x0\": 10, \"y0\": 8, \"x1\": 15, \"y1\": 13}"));
        let same = compare(&board(), &board(), &DiffSettings::default()).unwrap();
        let csv = to_csv(&same);
        let line = csv.lines().nth(1).unwrap();
        assert_eq!(line.split(',').count(), CSV_HEADER.split(',').count());
        assert!(line.ends_with(",0,,,,,same"));
        assert!(to_json(&same).contains("\"bbox\": null"));
    }
}
