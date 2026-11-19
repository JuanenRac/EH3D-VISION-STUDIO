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
    let (cx, cy, cr) = o.correction();
    format!(
        "{CSV_HEADER}\n{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
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
