//! A picture compared with a reference picture of the same size through a mask of tolerance: where the picture may differ from the reference by more than the general tolerance (a label that
//! changes from one part to the next, a bright reflection, the place of a hand) the mask says so, and where it must not be looked at the mask says that too. This is the check of a board
//! or a part against a good one; `diff` compares two pictures with a single threshold everywhere.
//!
//! The mask is a picture of the same size, read like this: a pixel of the mask at 0 is held to the general tolerance (`diff_threshold`), a pixel from 1 to 254 is allowed that many grey
//! levels more, and a pixel at 255 is not looked at. Without a mask every pixel is held to the general tolerance.

use crate::diff::DiffSettings;
use crate::image::Gray;
use crate::measure::{blobs, fixed};

/// The name of the format and its version, in the JSON file.
pub const FORMAT: &str = "eh3d-vision-reference";
pub const FORMAT_VERSION: u32 = 1;
pub const CSV_HEADER: &str =
    "checked_px,ignored_px,outside_px,outside_share,mean_excess,max_excess,regions,x0,y0,x1,y1,verdict";
/// The code of two pictures (or a picture and its mask) of different sizes.
pub const SIZE_MISMATCH: &str = "size_mismatch";
/// The code of a mask that leaves nothing to look at.
pub const NOTHING_CHECKED: &str = "reference_nothing_checked";
/// The value of a pixel of the mask that is not looked at.
pub const IGNORED: u8 = 255;
/// The most separate regions that are counted.
pub const MAX_REGIONS: usize = 100_000;

/// What the comparison found.
#[derive(Debug, Clone, PartialEq)]
pub struct RefCheck {
    /// How many pixels were looked at and how many the mask left out.
    pub checked_px: u64,
    pub ignored_px: u64,
    /// How many of the pixels looked at differ from the reference by more than they are allowed, and their share of the pixels looked at.
    pub outside_px: u64,
    pub outside_share: f64,
    /// The mean and the largest of how much those pixels pass their tolerance, in grey levels.
    pub mean_excess: f64,
    pub max_excess: u8,
    /// How many regions of at least `min_area` pixels outside the tolerance there are (four neighbours).
    pub regions: usize,
    /// The box around every pixel outside the tolerance, `(x0, y0, x1, y1)` with both corners included; `None` when there is none.
    pub bbox: Option<(usize, usize, usize, usize)>,
    /// Whether the share outside the tolerance is within `max_changed` of the settings.
    pub same: bool,
}

fn same_size(a: &Gray, b: &Gray, what: &str) -> Result<(), String> {
    if a.w != b.w || a.h != b.h {
        return Err(format!(
            "{SIZE_MISMATCH}: {what} are {} x {} and {} x {} pixels",
            a.w, a.h, b.w, b.h
        ));
    }
    Ok(())
}

/// The map of the pixels outside the tolerance (255 there, 0 elsewhere) and the numbers about it.
pub fn map(
    reference: &Gray,
    picture: &Gray,
    tolerance: Option<&Gray>,
    s: &DiffSettings,
) -> Result<(Gray, RefCheck), String> {
    crate::diff::check(s)?;
    same_size(reference, picture, "the reference and the picture")?;
    if let Some(m) = tolerance {
        same_size(reference, m, "the picture and its mask")?;
    }
    let n = reference.w * reference.h;
    if n == 0 || reference.data.len() != n || picture.data.len() != n {
        return Err(format!("{NOTHING_CHECKED}: the pictures have no pixels"));
    }
    let mut out = vec![0u8; n];
    let (mut checked, mut ignored, mut outside) = (0u64, 0u64, 0u64);
    let (mut excess_sum, mut max_excess) = (0u64, 0u8);
    let mut bbox: Option<(usize, usize, usize, usize)> = None;
    for (i, ((slot, &was), &is)) in out
        .iter_mut()
        .zip(&reference.data)
        .zip(&picture.data)
        .enumerate()
    {
        let extra = tolerance.map_or(0, |m| m.data[i]);
        if extra == IGNORED {
            ignored += 1;
            continue;
        }
        checked += 1;
        let allowed = u16::from(s.threshold) + u16::from(extra);
        let diff = u16::from(was.abs_diff(is));
        if diff > allowed {
            outside += 1;
            *slot = 255;
            let over = (diff - allowed) as u8;
            excess_sum += u64::from(over);
            max_excess = max_excess.max(over);
            let (x, y) = (i % reference.w, i / reference.w);
            bbox = Some(match bbox {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    if checked == 0 {
        return Err(format!(
            "{NOTHING_CHECKED}: the mask leaves no pixel to look at"
        ));
    }
    let map = Gray::new(reference.w, reference.h, out)?;
    let regions = if outside == 0 {
        0
    } else {
        blobs(&map, &|v| v > 0, s.min_area, u64::MAX, MAX_REGIONS)?.len()
    };
    let outside_share = outside as f64 / checked as f64;
    Ok((
        map,
        RefCheck {
            checked_px: checked,
            ignored_px: ignored,
            outside_px: outside,
            outside_share,
            mean_excess: if outside == 0 {
                0.0
            } else {
                excess_sum as f64 / outside as f64
            },
            max_excess,
            regions,
            bbox,
            same: outside_share <= s.max_changed,
        },
    ))
}

/// The numbers of the comparison.
pub fn compare(
    reference: &Gray,
    picture: &Gray,
    tolerance: Option<&Gray>,
    s: &DiffSettings,
) -> Result<RefCheck, String> {
    map(reference, picture, tolerance, s).map(|(_, c)| c)
}

fn verdict(c: &RefCheck) -> &'static str {
    if c.same {
        "same"
    } else {
        "different"
    }
}

/// The result as CSV: the line `CSV_HEADER` and one line with the numbers and the verdict (`same` or `different`); the box is empty when nothing is outside the tolerance.
pub fn to_csv(c: &RefCheck) -> String {
    let b = c.bbox.map_or(",,,".to_string(), |(x0, y0, x1, y1)| {
        format!("{x0},{y0},{x1},{y1}")
    });
    format!(
        "{CSV_HEADER}\n{},{},{},{},{},{},{},{},{}\n",
        c.checked_px,
        c.ignored_px,
        c.outside_px,
        fixed(c.outside_share, 6),
        fixed(c.mean_excess, 3),
        c.max_excess,
        c.regions,
        b,
        verdict(c)
    )
}

/// The result as JSON: an object with `format`, `version`, the numbers, the `bbox` (an object, or `null`) and the `verdict`.
pub fn to_json(c: &RefCheck) -> String {
    let b = c.bbox.map_or("null".to_string(), |(x0, y0, x1, y1)| {
        format!("{{\"x0\": {x0}, \"y0\": {y0}, \"x1\": {x1}, \"y1\": {y1}}}")
    });
    format!(
        "{{\n  \"format\": \"{FORMAT}\",\n  \"version\": {FORMAT_VERSION},\n  \"checked_px\": {},\n  \"ignored_px\": {},\n  \"outside_px\": {},\n  \"outside_share\": {},\n  \"mean_excess\": {},\n  \"max_excess\": {},\n  \"regions\": {},\n  \"bbox\": {b},\n  \"verdict\": \"{}\"\n}}\n",
        c.checked_px,
        c.ignored_px,
        c.outside_px,
        fixed(c.outside_share, 6),
        fixed(c.mean_excess, 3),
        c.max_excess,
        c.regions,
        verdict(c)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference() -> Gray {
        let mut g = Gray::blank(40, 30, 120).unwrap();
        for y in 8..16 {
            for x in 10..22 {
                g.data[y * 40 + x] = 60;
            }
        }
        g
    }

    fn with_patch(mut g: Gray, x0: usize, y0: usize, w: usize, h: usize, v: u8) -> Gray {
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                g.data[y * 40 + x] = v;
            }
        }
        g
    }

    fn settings() -> DiffSettings {
        DiffSettings {
            threshold: 20,
            min_area: 4,
            max_changed: 0.01,
        }
    }

    #[test]
    fn the_same_picture_is_the_same_and_a_small_change_within_the_tolerance_is_too() {
        let r = reference();
        let c = compare(&r, &r.clone(), None, &settings()).unwrap();
        assert_eq!(
            (c.checked_px, c.ignored_px, c.outside_px, c.regions),
            (1200, 0, 0, 0)
        );
        assert!(c.same && c.bbox.is_none());
        // a difference of 20 levels is within the tolerance of 20, one of 21 is not
        let near = with_patch(r.clone(), 2, 18, 10, 10, 140);
        assert!(compare(&r, &near, None, &settings()).unwrap().same);
        let far = with_patch(r.clone(), 2, 18, 10, 10, 141);
        let c = compare(&r, &far, None, &settings()).unwrap();
        assert_eq!((c.outside_px, c.regions, c.max_excess), (100, 1, 1));
        assert_eq!(c.bbox, Some((2, 18, 11, 27)));
        assert!(!c.same);
        assert!((c.mean_excess - 1.0).abs() < 1e-9);
    }

    #[test]
    fn the_mask_widens_the_tolerance_where_it_says_and_leaves_the_rest_out() {
        let r = reference();
        let p = with_patch(r.clone(), 2, 18, 10, 10, 170);
        // 50 levels of difference: 30 over the tolerance
        let c = compare(&r, &p, None, &settings()).unwrap();
        assert_eq!((c.outside_px, c.max_excess), (100, 30));
        // a mask that allows 30 more where the patch is: now within
        let mask = with_patch(Gray::blank(40, 30, 0).unwrap(), 2, 18, 10, 10, 30);
        let c = compare(&r, &p, Some(&mask), &settings()).unwrap();
        assert_eq!(c.outside_px, 0);
        // 29 more is not enough, by one level
        let mask = with_patch(Gray::blank(40, 30, 0).unwrap(), 2, 18, 10, 10, 29);
        let c = compare(&r, &p, Some(&mask), &settings()).unwrap();
        assert_eq!((c.outside_px, c.max_excess), (100, 1));
        // a mask that leaves the patch out does not count its pixels at all
        let blind = with_patch(Gray::blank(40, 30, 0).unwrap(), 2, 18, 10, 10, IGNORED);
        let c = compare(&r, &p, Some(&blind), &settings()).unwrap();
        assert_eq!((c.checked_px, c.ignored_px, c.outside_px), (1100, 100, 0));
        assert!(c.same);
        // and the tolerance never passes the top of the scale
        let wide = Gray::blank(40, 30, 254).unwrap();
        let c = compare(
            &r,
            &with_patch(r.clone(), 0, 0, 40, 30, 0),
            Some(&wide),
            &settings(),
        )
        .unwrap();
        assert_eq!(c.outside_px, 0);
    }

    #[test]
    fn the_map_marks_what_is_outside_and_the_regions_are_counted_by_area() {
        let r = reference();
        let p = with_patch(with_patch(r.clone(), 2, 18, 5, 5, 250), 30, 20, 1, 2, 250);
        let (m, c) = map(&r, &p, None, &settings()).unwrap();
        assert_eq!(c.outside_px, 27);
        // the speck of two pixels is below the smallest region
        assert_eq!(c.regions, 1);
        assert_eq!(m.data[18 * 40 + 2], 255);
        assert_eq!(m.data[20 * 40 + 30], 255);
        assert_eq!(m.data[0], 0);
        assert_eq!(m.data.iter().filter(|&&v| v == 255).count(), 27);
        assert_eq!(c.bbox, Some((2, 18, 30, 22)));
    }

    #[test]
    fn pictures_of_different_sizes_and_a_mask_with_nothing_to_look_at_are_refused() {
        let r = reference();
        let small = Gray::blank(20, 30, 120).unwrap();
        assert!(compare(&r, &small, None, &settings())
            .unwrap_err()
            .starts_with(SIZE_MISMATCH));
        assert!(compare(&small, &r, None, &settings())
            .unwrap_err()
            .starts_with(SIZE_MISMATCH));
        let bad_mask = Gray::blank(40, 29, 0).unwrap();
        assert!(compare(&r, &r, Some(&bad_mask), &settings())
            .unwrap_err()
            .starts_with(SIZE_MISMATCH));
        let all = Gray::blank(40, 30, IGNORED).unwrap();
        assert!(compare(&r, &r, Some(&all), &settings())
            .unwrap_err()
            .starts_with(NOTHING_CHECKED));
        let bad = DiffSettings {
            threshold: 255,
            ..settings()
        };
        assert!(compare(&r, &r, None, &bad).is_err());
    }

    #[test]
    fn the_files_have_the_numbers_of_the_comparison() {
        let r = reference();
        let c = compare(
            &r,
            &with_patch(r.clone(), 2, 18, 10, 10, 250),
            None,
            &settings(),
        )
        .unwrap();
        let csv = to_csv(&c);
        let mut lines = csv.lines();
        assert_eq!(lines.next(), Some(CSV_HEADER));
        let row: Vec<&str> = lines.next().unwrap().split(',').collect();
        assert_eq!(row.len(), CSV_HEADER.split(',').count());
        assert_eq!(&row[2..3], ["100"]);
        assert_eq!(row[11], "different");
        let json = to_json(&c);
        assert!(json.contains("\"format\": \"eh3d-vision-reference\""));
        assert!(json.contains("\"bbox\": {\"x0\": 2, \"y0\": 18, \"x1\": 11, \"y1\": 27}"));
        let same = compare(&r, &r, None, &settings()).unwrap();
        assert!(to_json(&same).contains("\"bbox\": null"));
        assert!(to_csv(&same).contains(",,,,same"));
    }
}
