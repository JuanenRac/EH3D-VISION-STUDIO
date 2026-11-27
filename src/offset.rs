//! Where a picked part is held, from a picture of it taken from below: the part is measured as for `measure`, its size is judged against the body of its package as for `inspect`, and its
//! offset from the axis of the nozzle and its turn from the way it should be held are worked out, with the correction that puts it where the nozzle would have it. The picture must hold
//! one part (the one under the nozzle); a part held too far off or turned too much is refused with the reason. Only the geometry is here: the camera and the nozzle are the business of
//! whatever takes the picture.

use crate::inspect::{judge, Judged, Package};
use crate::measure::{fixed, Part};

/// The reasons that a part fails besides the ones of its size (`length` and `width`): it is held too far from the axis of the nozzle, or turned too much.
pub const REASON_OFFSET: &str = "offset";
pub const REASON_TURN: &str = "turn";
/// The line of column names of the CSV file.
pub const CSV_HEADER: &str = "x_offset_mm,y_offset_mm,rot_offset_deg,x_correction_mm,y_correction_mm,rot_correction_deg,length_mm,width_mm,package,delta_long_mm,delta_short_mm,verdict,reason";
/// The name of the format and its version, in the JSON file.
pub const FORMAT: &str = "eh3d-vision-offset";
pub const FORMAT_VERSION: u32 = 1;

/// What is asked of the offset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetSettings {
    /// Where the axis of the nozzle is in the picture, in millimetres from the centre of the picture, x to the right and y up.
    pub nozzle_x_mm: f64,
    pub nozzle_y_mm: f64,
    /// The turn of the long side of the part when it is held as it should be, in degrees counter-clockwise from the x axis.
    pub nominal_angle_deg: f64,
    /// The most that the length or the width of the part may differ from the body of the package, in millimetres (the tolerance of the package wins).
    pub tolerance_mm: f64,
    /// The farthest from the axis of the nozzle that the part may be held, in millimetres; 0 does not check it.
    pub max_offset_mm: f64,
    /// The most that the part may be turned from the way it should be held, in degrees (up to 90); 0 does not check it.
    pub max_turn_deg: f64,
}

impl Default for OffsetSettings {
    fn default() -> Self {
        OffsetSettings {
            nozzle_x_mm: 0.0,
            nozzle_y_mm: 0.0,
            nominal_angle_deg: 0.0,
            tolerance_mm: 0.2,
            max_offset_mm: 0.0,
            max_turn_deg: 0.0,
        }
    }
}

/// How a part is held, and whether it may be placed.
#[derive(Debug, Clone, PartialEq)]
pub struct Offset {
    pub part: Part,
    /// Where the part is from the axis of the nozzle, in millimetres (x to the right, y up).
    pub x_mm: f64,
    pub y_mm: f64,
    /// How far its long side is from the way it should be held, in degrees counter-clockwise, from -90 (not included) to 90; for a part as long as wide (within the tolerance), from -45 (not
    /// included) to 45, because it looks the same turned a quarter of a turn.
    pub rot_deg: f64,
    /// The distance of the part from the axis of the nozzle, in millimetres.
    pub distance_mm: f64,
    /// The size of the part against the body of the package.
    pub size: Judged,
    /// Why the part fails: `length`, `width`, `offset` and `turn`; empty when it passes.
    pub reasons: Vec<&'static str>,
}

impl Offset {
    pub fn passes(&self) -> bool {
        self.reasons.is_empty()
    }

    /// The correction that puts the part where the nozzle would have it: the same distances and turn the other way.
    pub fn correction(&self) -> (f64, f64, f64) {
        (-self.x_mm, -self.y_mm, -self.rot_deg)
    }
}

/// `degrees` brought into the half turn that ends at `period / 2` (not included at the bottom): from `-period / 2` (not included) to `period / 2`.
fn wrap(degrees: f64, period: f64) -> f64 {
    let r = degrees.rem_euclid(period);
    if r > period / 2.0 {
        r - period
    } else {
        r
    }
}

fn check(s: &OffsetSettings) -> Result<(), String> {
    let ok = [
        s.nozzle_x_mm,
        s.nozzle_y_mm,
        s.nominal_angle_deg,
        s.tolerance_mm,
        s.max_offset_mm,
        s.max_turn_deg,
    ]
    .iter()
    .all(|v| v.is_finite())
        && s.tolerance_mm > 0.0
        && s.max_offset_mm >= 0.0
        && (0.0..=90.0).contains(&s.max_turn_deg);
    if ok {
        Ok(())
    } else {
        Err("the settings of the offset make no sense (numbers, a tolerance above 0, a distance of 0 or more and a turn from 0 to 90)".into())
    }
}

/// How the part of the picture is held. The picture must have exactly one part: none is the error `no_parts`, and more than one is `parts_count` (the part under the nozzle is alone in the
/// picture; two are a sign of dirt or of a second part picked up).
pub fn pick_offset(
    parts: &[Part],
    package: &Package,
    s: &OffsetSettings,
) -> Result<Offset, String> {
    check(s)?;
    let part = match parts {
        [] => return Err("no_parts: no part was found in the picture".into()),
        [one] => *one,
        many => {
            return Err(format!(
                "parts_count: {} parts were found in the picture and one was expected (the part under the nozzle)",
                many.len()
            ))
        }
    };
    held_part(part, package, s)
}

/// How each part of the picture is held, one after the other and each on its own (the parts are not told apart from the part under the nozzle: this is the table of corrections that the
/// window and `offset --each` give for a picture with several parts, such as a tray). A picture with no part is the error `no_parts`.
pub fn pick_offsets(
    parts: &[Part],
    package: &Package,
    s: &OffsetSettings,
) -> Result<Vec<Offset>, String> {
    check(s)?;
    if parts.is_empty() {
        return Err("no_parts: no part was found in the picture".into());
    }
    parts.iter().map(|p| held_part(*p, package, s)).collect()
}

/// The offset of one part (the settings are already checked).
fn held_part(part: Part, package: &Package, s: &OffsetSettings) -> Result<Offset, String> {
    let size = judge(&[part], package, s.tolerance_mm, 0.0)?
        .into_iter()
        .next()
        .ok_or("no verdict was given")?;
    let tolerance = package.tolerance_mm.unwrap_or(s.tolerance_mm);
    // a part as long as wide has no long side to turn: a quarter of a turn looks the same
    let period = if (package.long_mm - package.short_mm).abs() <= tolerance {
        90.0
    } else {
        180.0
    };
    let rot_deg = wrap(part.angle_deg - s.nominal_angle_deg, period);
    let (x_mm, y_mm) = (part.x_mm - s.nozzle_x_mm, part.y_mm - s.nozzle_y_mm);
    let distance_mm = x_mm.hypot(y_mm);
    let mut reasons = size.reasons.clone();
    if s.max_offset_mm > 0.0 && distance_mm > s.max_offset_mm {
        reasons.push(REASON_OFFSET);
    }
    if s.max_turn_deg > 0.0 && rot_deg.abs() > s.max_turn_deg {
        reasons.push(REASON_TURN);
    }
    Ok(Offset {
        part,
        x_mm,
        y_mm,
        rot_deg,
        distance_mm,
        size,
        reasons,
    })
}

/// The offset as CSV: the line `CSV_HEADER` and one line with the offset, the correction, the size of the part, the package, the deviation from its body, the verdict (`pass` or `fail`) and the
/// reasons joined with `+`.
pub fn to_csv(o: &Offset, package: &Package) -> String {
    format!("{CSV_HEADER}\n{}\n", csv_row(o, package))
}

/// The line of the CSV file for one part, without the end of the line.
fn csv_row(o: &Offset, package: &Package) -> String {
    let (cx, cy, cr) = o.correction();
    format!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{}",
        fixed(o.x_mm, 4),
        fixed(o.y_mm, 4),
        fixed(o.rot_deg, 3),
        fixed(cx, 4),
        fixed(cy, 4),
        fixed(cr, 3),
        fixed(o.part.length_mm, 4),
        fixed(o.part.width_mm, 4),
        crate::batch::csv_field(&package.name),
        fixed(o.size.delta_long_mm, 4),
        fixed(o.size.delta_short_mm, 4),
        if o.passes() { "pass" } else { "fail" },
        o.reasons.join("+")
    )
}

/// The table of corrections of a picture with several parts, as CSV: the line `TABLE_HEADER` (the column `part`, the number of the part from 1 in the order of the picture, then the columns
/// of `CSV_HEADER`) and one line for each part.
pub fn to_table_csv(offsets: &[Offset], package: &Package) -> String {
    let mut out = format!("{TABLE_HEADER}\n");
    for (i, o) in offsets.iter().enumerate() {
        out.push_str(&format!("{},{}\n", i + 1, csv_row(o, package)));
    }
    out
}

/// The line of column names of the table of corrections.
pub const TABLE_HEADER: &str = "part,x_offset_mm,y_offset_mm,rot_offset_deg,x_correction_mm,y_correction_mm,rot_correction_deg,length_mm,width_mm,package,delta_long_mm,delta_short_mm,verdict,reason";

/// The parts as they are held when the camera sees them turned over left to right (a camera under the nozzle that looks up, or a mirror in the way): the x of every part changes sign and its
/// turn changes sign (kept above -90 and up to 90). The parts are turned, not the pixels, so that no half pixel is lost to the side of the centre of the picture.
pub fn mirror_parts(parts: &[Part]) -> Vec<Part> {
    parts
        .iter()
        .map(|p| {
            let turned = -p.angle_deg;
            Part {
                x_mm: -p.x_mm,
                angle_deg: if turned <= -90.0 {
                    turned + 180.0
                } else {
                    turned
                },
                ..*p
            }
        })
        .collect()
}

/// Where the axis of the nozzle is in the picture, worked out from the parts that were picked with it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NozzleEstimate {
    /// How many parts it was worked out from.
    pub count: usize,
    /// The mean place of the parts in the picture, in millimetres from its centre (x to the right, y up): the best axis of the nozzle when the parts are picked at their centre.
    pub x_mm: f64,
    pub y_mm: f64,
    /// How far the parts are spread about it: the standard deviation in x and in y, and the distance of the farthest part from the mean place.
    pub spread_x_mm: f64,
    pub spread_y_mm: f64,
    pub farthest_mm: f64,
    /// The mean turn of the parts from the way they should be held, in degrees.
    pub mean_turn_deg: f64,
}

/// The axis of the nozzle from the offsets of several parts picked with it: the mean place of the parts (it does not depend on the axis that was in use when they were measured) with how
/// spread they are. Fewer than 2 parts are the error `few_pickups`, because one part says nothing about the axis. A part whose length or width is not that of the package is the error
/// `part_size`: it is not the part that was picked (a speck of dirt, a neighbour), and the axis it would move is the one the machine is then corrected with.
pub fn estimate_nozzle(offsets: &[Offset]) -> Result<NozzleEstimate, String> {
    if let Some((i, o)) = offsets
        .iter()
        .enumerate()
        .find(|(_, o)| !o.size.reasons.is_empty())
    {
        return Err(format!(
            "part_size: the part of picture {} measures {} x {} mm, which is not the size of the package ({}): the axis of the nozzle is not worked out from it",
            i + 1,
            fixed(o.part.length_mm, 3),
            fixed(o.part.width_mm, 3),
            o.size.reasons.join(", ")
        ));
    }
    let n = offsets.len();
    if n < 2 {
        return Err(format!(
            "few_pickups: {n} part was given and at least 2 are needed to tell where the axis of the nozzle is"
        ));
    }
    let count = n as f64;
    let mean = |f: fn(&Offset) -> f64| offsets.iter().map(f).sum::<f64>() / count;
    let x_mm = mean(|o| o.part.x_mm);
    let y_mm = mean(|o| o.part.y_mm);
    let spread = |f: fn(&Offset) -> f64, m: f64| {
        (offsets.iter().map(|o| (f(o) - m).powi(2)).sum::<f64>() / (count - 1.0)).sqrt()
    };
    Ok(NozzleEstimate {
        count: n,
        x_mm,
        y_mm,
        spread_x_mm: spread(|o| o.part.x_mm, x_mm),
        spread_y_mm: spread(|o| o.part.y_mm, y_mm),
        farthest_mm: offsets
            .iter()
            .map(|o| (o.part.x_mm - x_mm).hypot(o.part.y_mm - y_mm))
            .fold(0.0, f64::max),
        mean_turn_deg: mean(|o| o.rot_deg),
    })
}

/// The estimate as the two lines of `vision.cfg` that set the axis of the nozzle (`nozzle_x_mm` and `nozzle_y_mm`).
pub fn nozzle_cfg(e: &NozzleEstimate) -> String {
    format!(
        "nozzle_x_mm = {}\nnozzle_y_mm = {}\n",
        fixed(e.x_mm, 4),
        fixed(e.y_mm, 4)
    )
}

/// The offset as JSON: an object with `format`, `version`, `unit` ("mm"), the `package`, the `offset` (`x_mm`, `y_mm`, `rot_deg`, `distance_mm`), the `correction` (`x_mm`, `y_mm`, `rot_deg`),
/// the `size` of the part (`length_mm`, `width_mm`, `delta_long_mm`, `delta_short_mm`), the `verdict` and the list of `reasons`.
pub fn to_json(o: &Offset, package: &Package) -> String {
    let (cx, cy, cr) = o.correction();
    let reasons: Vec<String> = o
        .reasons
        .iter()
        .map(|r| crate::inspect::json_string(r))
        .collect();
    format!(
        "{{\n  \"format\": \"{FORMAT}\",\n  \"version\": {FORMAT_VERSION},\n  \"unit\": \"mm\",\n  \"package\": {},\n  \"offset\": {{\"x_mm\": {}, \"y_mm\": {}, \"rot_deg\": {}, \"distance_mm\": {}}},\n  \"correction\": {{\"x_mm\": {}, \"y_mm\": {}, \"rot_deg\": {}}},\n  \"size\": {{\"length_mm\": {}, \"width_mm\": {}, \"delta_long_mm\": {}, \"delta_short_mm\": {}}},\n  \"verdict\": \"{}\",\n  \"reasons\": [{}]\n}}\n",
        crate::inspect::json_string(&package.name),
        fixed(o.x_mm, 4),
        fixed(o.y_mm, 4),
        fixed(o.rot_deg, 3),
        fixed(o.distance_mm, 4),
        fixed(cx, 4),
        fixed(cy, 4),
        fixed(cr, 3),
        fixed(o.part.length_mm, 4),
        fixed(o.part.width_mm, 4),
        fixed(o.size.delta_long_mm, 4),
        fixed(o.size.delta_short_mm, 4),
        if o.passes() { "pass" } else { "fail" },
        reasons.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::Gray;

    fn package(long: f64, short: f64) -> Package {
        Package {
            name: "0805".into(),
            long_mm: long,
            short_mm: short,
            tolerance_mm: None,
        }
    }

    fn part(x: f64, y: f64, angle: f64, length: f64, width: f64) -> Part {
        Part {
            x_mm: x,
            y_mm: y,
            angle_deg: angle,
            length_mm: length,
            width_mm: width,
            area_mm2: length * width,
        }
    }

    #[test]
    fn the_offset_is_the_place_of_the_part_from_the_nozzle_and_the_correction_is_the_opposite() {
        let s = OffsetSettings {
            nozzle_x_mm: 0.1,
            nozzle_y_mm: -0.2,
            ..OffsetSettings::default()
        };
        let o = pick_offset(&[part(0.4, 0.1, 12.0, 2.0, 1.25)], &package(2.0, 1.25), &s).unwrap();
        assert!((o.x_mm - 0.3).abs() < 1e-12 && (o.y_mm - 0.3).abs() < 1e-12);
        assert!((o.distance_mm - 0.3f64.hypot(0.3)).abs() < 1e-12);
        assert!((o.rot_deg - 12.0).abs() < 1e-12);
        assert_eq!(o.correction(), (-o.x_mm, -o.y_mm, -12.0));
        assert!(o.passes());
        // held as it should be, with the nominal turn: no turn to correct
        let held = OffsetSettings {
            nominal_angle_deg: 90.0,
            ..OffsetSettings::default()
        };
        let o = pick_offset(
            &[part(0.0, 0.0, 90.0, 2.0, 1.25)],
            &package(2.0, 1.25),
            &held,
        )
        .unwrap();
        assert_eq!(o.rot_deg, 0.0);
    }

    #[test]
    fn the_turn_is_the_short_way_round_for_a_part_that_looks_the_same_turned_half_a_turn() {
        let p = package(2.0, 1.25);
        let turn = |angle: f64, nominal: f64| {
            let s = OffsetSettings {
                nominal_angle_deg: nominal,
                ..OffsetSettings::default()
            };
            pick_offset(&[part(0.0, 0.0, angle, 2.0, 1.25)], &p, &s)
                .unwrap()
                .rot_deg
        };
        // 85 against a nominal of -85 is 170 degrees apart one way and 10 the other
        assert!((turn(85.0, -85.0) - -10.0).abs() < 1e-9);
        assert!((turn(-85.0, 85.0) - 10.0).abs() < 1e-9);
        assert_eq!(turn(90.0, 0.0), 90.0);
        assert!((turn(10.0, 180.0) - 10.0).abs() < 1e-9);
        // a square part looks the same turned a quarter of a turn
        let square = package(2.0, 2.0);
        let s = OffsetSettings::default();
        let o = pick_offset(&[part(0.0, 0.0, 80.0, 2.0, 2.0)], &square, &s).unwrap();
        assert!((o.rot_deg - -10.0).abs() < 1e-9, "{}", o.rot_deg);
        let o = pick_offset(&[part(0.0, 0.0, -30.0, 2.0, 2.0)], &square, &s).unwrap();
        assert!((o.rot_deg - -30.0).abs() < 1e-9);
        let o = pick_offset(&[part(0.0, 0.0, 45.0, 2.0, 2.0)], &square, &s).unwrap();
        assert_eq!(o.rot_deg, 45.0);
    }

    #[test]
    fn a_part_of_the_wrong_size_held_too_far_or_turned_too_much_fails_with_each_reason() {
        let p = package(2.0, 1.25);
        let strict = OffsetSettings {
            max_offset_mm: 0.5,
            max_turn_deg: 10.0,
            ..OffsetSettings::default()
        };
        let good = pick_offset(&[part(0.1, 0.1, 5.0, 2.0, 1.25)], &p, &strict).unwrap();
        assert!(good.passes());
        let bad = pick_offset(&[part(0.6, 0.0, 20.0, 2.5, 1.0)], &p, &strict).unwrap();
        assert_eq!(bad.reasons, vec!["length", "width", "offset", "turn"]);
        // the limits that are 0 are not checked, and the tolerance of the package wins
        let loose = pick_offset(
            &[part(9.0, 9.0, 80.0, 2.0, 1.25)],
            &p,
            &OffsetSettings::default(),
        )
        .unwrap();
        assert!(loose.passes());
        let own = Package {
            tolerance_mm: Some(0.01),
            ..p.clone()
        };
        let o = pick_offset(
            &[part(0.0, 0.0, 0.0, 2.05, 1.25)],
            &own,
            &OffsetSettings::default(),
        )
        .unwrap();
        assert_eq!(o.reasons, vec!["length"]);
    }

    #[test]
    fn one_part_is_needed_and_settings_that_make_no_sense_are_refused() {
        let p = package(2.0, 1.25);
        let s = OffsetSettings::default();
        let none = pick_offset(&[], &p, &s).unwrap_err();
        assert!(none.starts_with("no_parts"), "{none}");
        let two = [
            part(0.0, 0.0, 0.0, 2.0, 1.25),
            part(3.0, 0.0, 0.0, 2.0, 1.25),
        ];
        let many = pick_offset(&two, &p, &s).unwrap_err();
        assert!(many.starts_with("parts_count: 2 parts"), "{many}");
        let one = [part(0.0, 0.0, 0.0, 2.0, 1.25)];
        for bad in [
            OffsetSettings {
                tolerance_mm: 0.0,
                ..s
            },
            OffsetSettings {
                max_offset_mm: -1.0,
                ..s
            },
            OffsetSettings {
                max_turn_deg: 91.0,
                ..s
            },
            OffsetSettings {
                nozzle_x_mm: f64::NAN,
                ..s
            },
            OffsetSettings {
                nominal_angle_deg: f64::INFINITY,
                ..s
            },
        ] {
            assert!(pick_offset(&one, &p, &bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn each_part_of_a_tray_gets_its_own_offset_and_its_row_in_the_table_of_corrections() {
        let p = package(2.0, 1.25);
        let s = OffsetSettings {
            nozzle_x_mm: 1.0,
            max_offset_mm: 2.5,
            ..OffsetSettings::default()
        };
        let parts = [
            part(1.5, 0.0, 3.0, 2.0, 1.25),
            part(-2.0, 1.0, -4.0, 2.0, 1.25),
            part(4.0, 0.0, 0.0, 2.0, 1.25),
        ];
        // the single-part function refuses a tray; the table takes every part, each on its own
        assert!(pick_offset(&parts, &p, &s).is_err());
        let all = pick_offsets(&parts, &p, &s).unwrap();
        assert_eq!(all.len(), 3);
        for (one, part) in all.iter().zip(&parts) {
            assert_eq!(*one, pick_offset(&[*part], &p, &s).unwrap());
        }
        assert!(all[0].passes() && !all[1].passes() && !all[2].passes());
        assert_eq!(all[1].reasons, vec!["offset"]);
        let table = to_table_csv(&all, &p);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines[0], TABLE_HEADER);
        assert_eq!(lines.len(), 4);
        assert_eq!(
            lines[1],
            "1,0.5000,0.0000,3.000,-0.5000,0.0000,-3.000,2.0000,1.2500,0805,0.0000,0.0000,pass,"
        );
        assert!(lines[2].starts_with("2,-3.0000,1.0000,-4.000,3.0000,-1.0000,4.000,"));
        assert!(lines[2].ends_with(",fail,offset") && lines[3].starts_with("3,3.0000,0.0000,"));
        // the table has the columns of the file of one part, with the number of the part first
        assert_eq!(TABLE_HEADER, format!("part,{CSV_HEADER}"));
        assert_eq!(lines[1].split(',').count(), TABLE_HEADER.split(',').count());
        // none is the error of the single part, and settings that make no sense are refused
        assert!(pick_offsets(&[], &p, &s)
            .unwrap_err()
            .starts_with("no_parts"));
        let bad = OffsetSettings {
            tolerance_mm: 0.0,
            ..s
        };
        assert!(pick_offsets(&parts, &p, &bad).is_err());
        assert_eq!(to_table_csv(&[], &p), format!("{TABLE_HEADER}\n"));
    }

    /// A picture of 120 by 80 with a dark rectangle of 40 by 16 pixels turned by `angle` degrees about (80, 30).
    fn turned_rectangle(angle: f64) -> Gray {
        let (w, h) = (120usize, 80usize);
        let (sin, cos) = angle.to_radians().sin_cos();
        let mut g = Gray::blank(w, h, 220).unwrap();
        for y in 0..h {
            for x in 0..w {
                let (dx, up) = (x as f64 - 80.0, 30.0 - y as f64);
                let (u, v) = (dx * cos + up * sin, -dx * sin + up * cos);
                if u.abs() <= 20.0 && v.abs() <= 8.0 {
                    g.data[y * w + x] = 30;
                }
            }
        }
        g
    }

    #[test]
    fn a_part_seen_turned_over_left_to_right_changes_the_sign_of_its_x_and_of_its_turn() {
        let g = turned_rectangle(30.0);
        let scale = crate::scale::Scale { mm_per_px: 0.1 };
        let settings = crate::measure::PartSettings::default();
        let direct = crate::measure::measure_parts(&g, &settings, scale).unwrap();
        assert!((direct[0].angle_deg - 30.0).abs() < 1.0, "{direct:?}");
        let seen = mirror_parts(&direct);
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].x_mm, -direct[0].x_mm);
        assert_eq!(seen[0].angle_deg, -direct[0].angle_deg);
        // the y and the size do not change
        assert_eq!(seen[0].y_mm, direct[0].y_mm);
        assert_eq!(seen[0].length_mm, direct[0].length_mm);
        assert_eq!(seen[0].area_mm2, direct[0].area_mm2);
        // the turn stays above -90 and up to 90, and turning twice gives the part back
        let edge = [
            part(1.0, 2.0, 90.0, 2.0, 1.0),
            part(-1.0, 2.0, 45.0, 2.0, 1.0),
            part(0.0, 0.0, -89.5, 2.0, 1.0),
            part(0.0, 0.0, 0.0, 2.0, 1.0),
        ];
        let once = mirror_parts(&edge);
        assert_eq!(once[0].angle_deg, 90.0);
        assert_eq!(once[1].angle_deg, -45.0);
        assert_eq!(once[2].angle_deg, 89.5);
        assert_eq!(once[0].x_mm, -1.0);
        assert!(once
            .iter()
            .all(|p| p.angle_deg > -90.0 && p.angle_deg <= 90.0));
        assert_eq!(mirror_parts(&once), edge.to_vec());
        assert!(mirror_parts(&[]).is_empty());
        // a part seen in a mirror, held a little to the right of the nozzle, is to the left in the picture
        let pack = package(2.0, 1.0);
        let held = pick_offset(
            &mirror_parts(&[part(-0.4, 0.0, -3.0, 2.0, 1.0)]),
            &pack,
            &OffsetSettings::default(),
        )
        .unwrap();
        assert_eq!((held.x_mm, held.rot_deg), (0.4, 3.0));
    }

    #[test]
    fn the_axis_of_the_nozzle_is_the_mean_place_of_the_parts_picked_with_it() {
        let p = package(2.0, 1.25);
        let places = [(0.30, 0.10), (0.34, 0.14), (0.26, 0.06)];
        let held = |nozzle_x: f64, nozzle_y: f64| {
            let s = OffsetSettings {
                nozzle_x_mm: nozzle_x,
                nozzle_y_mm: nozzle_y,
                ..OffsetSettings::default()
            };
            let parts: Vec<Part> = places
                .iter()
                .enumerate()
                .map(|(i, (x, y))| part(*x, *y, i as f64, 2.0, 1.25))
                .collect();
            pick_offsets(&parts, &p, &s).unwrap()
        };
        let e = estimate_nozzle(&held(0.0, 0.0)).unwrap();
        assert_eq!(e.count, 3);
        assert!(
            (e.x_mm - 0.30).abs() < 1e-12 && (e.y_mm - 0.10).abs() < 1e-12,
            "{e:?}"
        );
        assert!((e.spread_x_mm - 0.04).abs() < 1e-12 && (e.spread_y_mm - 0.04).abs() < 1e-12);
        assert!((e.farthest_mm - 0.04f64.hypot(0.04)).abs() < 1e-12);
        assert!((e.mean_turn_deg - 1.0).abs() < 1e-12);
        // the axis that was in use when the parts were measured does not change the answer
        assert_eq!(estimate_nozzle(&held(5.0, -3.0)).unwrap().x_mm, e.x_mm);
        assert_eq!(
            nozzle_cfg(&e),
            "nozzle_x_mm = 0.3000\nnozzle_y_mm = 0.1000\n"
        );
        // the lines are the keys of the configuration file
        let cfg = crate::config::Config::parse(&nozzle_cfg(&e)).unwrap();
        assert_eq!((cfg.nozzle_x_mm, cfg.nozzle_y_mm), (0.3, 0.1));
        // one part says nothing about the axis, and none says less
        assert!(estimate_nozzle(&held(0.0, 0.0)[..1])
            .unwrap_err()
            .starts_with("few_pickups"));
        assert!(estimate_nozzle(&[]).unwrap_err().starts_with("few_pickups"));
    }

    #[test]
    fn a_spot_that_is_not_the_part_does_not_move_the_axis_of_the_nozzle() {
        let p = package(2.0, 1.25);
        let s = OffsetSettings::default();
        let good = pick_offset(&[part(0.3, 0.1, 0.0, 2.0, 1.25)], &p, &s).unwrap();
        // a speck of dirt alone in the picture is measured as a part, far from the others
        let speck = pick_offset(&[part(4.0, -3.0, 0.0, 0.3, 0.3)], &p, &s).unwrap();
        let e = estimate_nozzle(&[good.clone(), speck]).unwrap_err();
        assert!(e.starts_with("part_size: the part of picture 2 "), "{e}");
        // a part held far from the axis in use is what the estimate is for
        let strict = OffsetSettings {
            max_offset_mm: 0.01,
            ..OffsetSettings::default()
        };
        let far = pick_offset(&[part(0.5, 0.1, 0.0, 2.0, 1.25)], &p, &strict).unwrap();
        assert!(!far.passes());
        assert!(estimate_nozzle(&[good, far]).is_ok());
    }

    #[test]
    fn the_files_have_the_offset_the_correction_the_size_and_the_verdict() {
        let p = package(2.0, 1.25);
        let s = OffsetSettings {
            max_offset_mm: 0.2,
            ..OffsetSettings::default()
        };
        let o = pick_offset(&[part(0.3, -0.1, 4.0, 2.0, 1.25)], &p, &s).unwrap();
        let csv = to_csv(&o, &p);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], CSV_HEADER);
        assert_eq!(
            lines[1],
            "0.3000,-0.1000,4.000,-0.3000,0.1000,-4.000,2.0000,1.2500,0805,0.0000,0.0000,fail,offset"
        );
        assert_eq!(lines.len(), 2);
        let json = to_json(&o, &p);
        assert!(
            json.contains("\"format\": \"eh3d-vision-offset\""),
            "{json}"
        );
        assert!(
            json.contains("\"offset\": {\"x_mm\": 0.3000, \"y_mm\": -0.1000, \"rot_deg\": 4.000"),
            "{json}"
        );
        assert!(
            json.contains(
                "\"correction\": {\"x_mm\": -0.3000, \"y_mm\": 0.1000, \"rot_deg\": -4.000}"
            ),
            "{json}"
        );
        assert!(
            json.contains("\"verdict\": \"fail\"") && json.contains("\"reasons\": [\"offset\"]"),
            "{json}"
        );
        // a name with a comma is quoted in the CSV and escaped in the JSON
        let odd = Package {
            name: "odd, \"name\"".into(),
            ..p.clone()
        };
        assert!(to_csv(&o, &odd).contains(",\"odd, \"\"name\"\"\","));
        assert!(to_json(&o, &odd).contains("\"odd, \\\"name\\\"\""));
        // a part that passes has no reason
        let ok = pick_offset(&[part(0.0, 0.0, 0.0, 2.0, 1.25)], &p, &s).unwrap();
        assert!(to_csv(&ok, &p).ends_with(",pass,\n"));
        assert!(to_json(&ok, &p).contains("\"reasons\": []"));
    }
}
