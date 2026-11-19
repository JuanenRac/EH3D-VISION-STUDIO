//! The inspection of a part: the size that was measured in a picture (`measure::measure_parts`) is compared with the size that the package of the part should have, from a table in a
//! plain CSV file, and each part gets a verdict. The table has one line per package: `package`, `body_long_mm` and `body_short_mm`, and optionally `tolerance_mm` for the packages that
//! need another tolerance than the one asked for. A part passes when its length and its width are within the tolerance of the body, and, when a limit is given, its turn is within it.

use crate::batch::csv_field;
use crate::measure::{fixed, Part};

/// The most packages a table may hold; a table with more is refused.
pub const MAX_PACKAGES: usize = 10_000;
/// The line of column names of the CSV file of the verdicts.
pub const CSV_HEADER: &str = "index,x_mm,y_mm,angle_deg,length_mm,width_mm,package,expected_long_mm,expected_short_mm,delta_long_mm,delta_short_mm,verdict,reason";
/// The name of the format and its version, in the JSON file.
pub const FORMAT: &str = "eh3d-vision-inspection";
pub const FORMAT_VERSION: u32 = 1;
/// The reasons a part fails, as they are written in the `reason` column (several are joined with `+`).
pub const REASON_LENGTH: &str = "length";
pub const REASON_WIDTH: &str = "width";
pub const REASON_ANGLE: &str = "angle";

/// A package: the size of its body in millimetres, the long side and the short side.
#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub name: String,
    pub long_mm: f64,
    pub short_mm: f64,
    /// The tolerance of this package, when the table gives one.
    pub tolerance_mm: Option<f64>,
}

/// The fields of a line of CSV: separated by commas, a field in quotes may hold commas and a doubled quote is a quote.
fn fields(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            ('"', true) => quoted = false,
            ('"', false) if field.is_empty() => quoted = true,
            (',', false) => out.push(std::mem::take(&mut field)),
            _ => field.push(c),
        }
    }
    if quoted {
        return Err("a quote is not closed".into());
    }
    out.push(field);
    Ok(out.into_iter().map(|f| f.trim().to_string()).collect())
}

fn positive(text: &str, what: &str, line: usize) -> Result<f64, String> {
    text.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or_else(|| format!("line {line}: {what} '{text}' is not a number above 0"))
}

/// The packages of a table. The first line that is not empty and does not start with `#` names the columns (`package`, `body_long_mm`, `body_short_mm` and, if wanted,
/// `tolerance_mm`, in any order); the long side may not be shorter than the short one, a name may not be written twice and more than `MAX_PACKAGES` lines are refused.
pub fn parse_table(text: &str) -> Result<Vec<Package>, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with('#'));
    let (head_line, head) = lines
        .next()
        .ok_or("the table of packages is empty: the first line must name the columns")?;
    let names: Vec<String> = fields(head)
        .map_err(|e| format!("line {head_line}: {e}"))?
        .iter()
        .map(|n| n.to_ascii_lowercase())
        .collect();
    let column = |name: &str| names.iter().position(|n| n == name);
    let (Some(c_name), Some(c_long), Some(c_short)) = (
        column("package"),
        column("body_long_mm"),
        column("body_short_mm"),
    ) else {
        return Err(format!(
            "line {head_line}: the columns must include package, body_long_mm and body_short_mm"
        ));
    };
    let c_tolerance = column("tolerance_mm");
    let mut out: Vec<Package> = Vec::new();
    for (n, line) in lines {
        if out.len() >= MAX_PACKAGES {
            return Err(format!(
                "more than {MAX_PACKAGES} packages: a table is not larger than that"
            ));
        }
        let f = fields(line).map_err(|e| format!("line {n}: {e}"))?;
        if f.len() != names.len() {
            return Err(format!(
                "line {n}: {} fields, the header has {}",
                f.len(),
                names.len()
            ));
        }
        let name = f[c_name].clone();
        if name.is_empty() {
            return Err(format!("line {n}: the package has no name"));
        }
        if out.iter().any(|p| p.name.eq_ignore_ascii_case(&name)) {
            return Err(format!("line {n}: the package {name} is written twice"));
        }
        let long_mm = positive(&f[c_long], "body_long_mm", n)?;
        let short_mm = positive(&f[c_short], "body_short_mm", n)?;
        if long_mm < short_mm {
            return Err(format!(
                "line {n}: body_long_mm {long_mm} is shorter than body_short_mm {short_mm}"
            ));
        }
        let tolerance_mm = match c_tolerance {
            Some(c) if !f[c].is_empty() => Some(positive(&f[c], "tolerance_mm", n)?),
            _ => None,
        };
        out.push(Package {
            name,
            long_mm,
            short_mm,
            tolerance_mm,
        });
    }
    Ok(out)
}

/// The package of that name (the case of the letters does not matter), or an error that lists some of the names.
pub fn find_package<'a>(table: &'a [Package], name: &str) -> Result<&'a Package, String> {
    table
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name.trim()))
        .ok_or_else(|| {
            let known: Vec<&str> = table.iter().take(8).map(|p| p.name.as_str()).collect();
            format!(
                "package_unknown: '{name}' is not in the table ({}{})",
                known.join(", "),
                if table.len() > 8 { ", ..." } else { "" }
            )
        })
}

/// A part with the verdict.
#[derive(Debug, Clone, PartialEq)]
pub struct Judged {
    pub part: Part,
    /// The length of the part minus the long side of the body, in millimetres.
    pub delta_long_mm: f64,
    /// The width of the part minus the short side of the body.
    pub delta_short_mm: f64,
    /// Why the part fails; empty when it passes.
    pub reasons: Vec<&'static str>,
}

impl Judged {
    pub fn passes(&self) -> bool {
        self.reasons.is_empty()
    }
}

/// Compares each part with the package. `tolerance_mm` is used unless the package has its own; `max_angle_deg` of 0 leaves the turn unchecked, and above 0 a part whose long side is
/// turned by more than that from the x axis fails (a turn of 90 degrees counts as 90, so a part lying across fails when the limit is below it).
pub fn judge(
    parts: &[Part],
    package: &Package,
    tolerance_mm: f64,
    max_angle_deg: f64,
) -> Result<Vec<Judged>, String> {
    if !(tolerance_mm.is_finite() && tolerance_mm > 0.0) {
        return Err("the tolerance must be more than 0 millimetres".into());
    }
    if !(max_angle_deg.is_finite() && (0.0..=90.0).contains(&max_angle_deg)) {
        return Err("the largest turn must be from 0 (not checked) to 90 degrees".into());
    }
    let tolerance = package.tolerance_mm.unwrap_or(tolerance_mm);
    Ok(parts
        .iter()
        .map(|p| {
            let (dl, ds) = (p.length_mm - package.long_mm, p.width_mm - package.short_mm);
            let mut reasons = Vec::new();
            if dl.abs() > tolerance {
                reasons.push(REASON_LENGTH);
            }
            if ds.abs() > tolerance {
                reasons.push(REASON_WIDTH);
            }
            if max_angle_deg > 0.0 && p.angle_deg.abs() > max_angle_deg {
                reasons.push(REASON_ANGLE);
            }
            Judged {
                part: *p,
                delta_long_mm: dl,
                delta_short_mm: ds,
                reasons,
            }
        })
        .collect())
}

/// One line that says how many parts passed.
pub fn summary(judged: &[Judged]) -> String {
    let pass = judged.iter().filter(|j| j.passes()).count();
    format!(
        "{} parts: {pass} pass, {} fail",
        judged.len(),
        judged.len() - pass
    )
}

/// The verdicts as CSV: the line `CSV_HEADER` and one line per part, `index` from 1, the verdict `pass` or `fail` and the reasons joined with `+`.
pub fn to_csv(judged: &[Judged], package: &Package) -> String {
    let mut out = format!("{CSV_HEADER}\n");
    for (i, j) in judged.iter().enumerate() {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            i + 1,
            fixed(j.part.x_mm, 4),
            fixed(j.part.y_mm, 4),
            fixed(j.part.angle_deg, 3),
            fixed(j.part.length_mm, 4),
            fixed(j.part.width_mm, 4),
            csv_field(&package.name),
            fixed(package.long_mm, 4),
            fixed(package.short_mm, 4),
            fixed(j.delta_long_mm, 4),
            fixed(j.delta_short_mm, 4),
            if j.passes() { "pass" } else { "fail" },
            j.reasons.join("+")
        ));
    }
    out
}

/// A text as a JSON string.
pub(crate) fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The verdicts as JSON: an object with `format`, `version`, `unit` ("mm"), the `package` (its `name`, `body_long_mm`, `body_short_mm`), the `tolerance_mm` that was used, the number of
/// parts that `pass` and that `fail`, and the list `parts` of objects with the fields of the columns of the CSV file.
pub fn to_json(judged: &[Judged], package: &Package, tolerance_mm: f64) -> String {
    let pass = judged.iter().filter(|j| j.passes()).count();
    let mut out = format!(
        "{{\n  \"format\": \"{FORMAT}\",\n  \"version\": {FORMAT_VERSION},\n  \"unit\": \"mm\",\n  \"package\": {{\"name\": {}, \"body_long_mm\": {}, \"body_short_mm\": {}}},\n  \"tolerance_mm\": {},\n  \"pass\": {pass},\n  \"fail\": {},\n  \"parts\": [",
        json_string(&package.name),
        fixed(package.long_mm, 4),
        fixed(package.short_mm, 4),
        fixed(package.tolerance_mm.unwrap_or(tolerance_mm), 4),
        judged.len() - pass
    );
    for (i, j) in judged.iter().enumerate() {
        let reasons: Vec<String> = j.reasons.iter().map(|r| json_string(r)).collect();
        out.push_str(&format!(
            "{}\n    {{\"index\": {}, \"x_mm\": {}, \"y_mm\": {}, \"angle_deg\": {}, \"length_mm\": {}, \"width_mm\": {}, \"delta_long_mm\": {}, \"delta_short_mm\": {}, \"verdict\": \"{}\", \"reasons\": [{}]}}",
            if i == 0 { "" } else { "," },
            i + 1,
            fixed(j.part.x_mm, 4),
            fixed(j.part.y_mm, 4),
            fixed(j.part.angle_deg, 3),
            fixed(j.part.length_mm, 4),
            fixed(j.part.width_mm, 4),
            fixed(j.delta_long_mm, 4),
            fixed(j.delta_short_mm, 4),
            if j.passes() { "pass" } else { "fail" },
            reasons.join(", ")
        ));
    }
    out.push_str(if judged.is_empty() {
        "]\n}\n"
    } else {
        "\n  ]\n}\n"
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "# the bodies of the packages\npackage,body_long_mm,body_short_mm,tolerance_mm\n0603,1.6,0.8,\n0805,2.0,1.25,\nSOT-23,2.9,1.3,0.3\n\"odd, name\",5,5,\n";

    fn part(length: f64, width: f64, angle: f64) -> Part {
        Part {
            x_mm: 1.0,
            y_mm: -2.0,
            angle_deg: angle,
            length_mm: length,
            width_mm: width,
            area_mm2: length * width,
        }
    }

    #[test]
    fn a_table_of_packages_is_read_with_its_quotes_and_its_optional_tolerance() {
        let t = parse_table(TABLE).unwrap();
        assert_eq!(t.len(), 4);
        assert_eq!(t[0].name, "0603");
        assert_eq!(
            (t[0].long_mm, t[0].short_mm, t[0].tolerance_mm),
            (1.6, 0.8, None)
        );
        assert_eq!(t[2].tolerance_mm, Some(0.3));
        assert_eq!(t[3].name, "odd, name");
        // the columns may come in any order, and a table without the tolerance column is fine
        let t = parse_table("body_short_mm,PACKAGE,body_long_mm\n0.5,0402,1.0\n").unwrap();
        assert_eq!(
            (t[0].name.as_str(), t[0].long_mm, t[0].short_mm),
            ("0402", 1.0, 0.5)
        );
        // a mark of byte order at the start of the file is left out
        assert!(parse_table("\u{feff}package,body_long_mm,body_short_mm\nA,2,1\n").is_ok());
    }

    #[test]
    fn a_table_that_makes_no_sense_is_refused_with_its_line() {
        let head = "package,body_long_mm,body_short_mm\n";
        for (text, expected) in [
            ("", "is empty"),
            ("name,body_long_mm,body_short_mm\n", "must include package"),
            ("package,body_long_mm\nA,2\n", "must include package"),
            (
                &format!("{head}A,2\n") as &str,
                "line 2: 2 fields, the header has 3",
            ),
            (&format!("{head}A,x,1\n"), "line 2: body_long_mm 'x'"),
            (&format!("{head}A,2,0\n"), "line 2: body_short_mm '0'"),
            (&format!("{head}A,2,-1\n"), "line 2: body_short_mm '-1'"),
            (&format!("{head}A,inf,1\n"), "line 2: body_long_mm 'inf'"),
            (&format!("{head}A,1,2\n"), "shorter than"),
            (&format!("{head},2,1\n"), "has no name"),
            (
                &format!("{head}A,2,1\na,3,2\n"),
                "line 3: the package a is written twice",
            ),
            (&format!("{head}\"A,2,1\n"), "a quote is not closed"),
            (
                "package,body_long_mm,body_short_mm,tolerance_mm\nA,2,1,0\n",
                "tolerance_mm '0'",
            ),
        ] {
            let e = parse_table(text).unwrap_err();
            assert!(e.contains(expected), "{text:?} gave {e:?}");
        }
        let many: String = std::iter::once("package,body_long_mm,body_short_mm\n".to_string())
            .chain((0..=MAX_PACKAGES).map(|i| format!("P{i},2,1\n")))
            .collect();
        assert!(parse_table(&many).unwrap_err().contains("10000"));
    }

    #[test]
    fn a_package_is_found_by_its_name_and_an_unknown_one_lists_the_known() {
        let t = parse_table(TABLE).unwrap();
        assert_eq!(find_package(&t, " sot-23 ").unwrap().name, "SOT-23");
        let e = find_package(&t, "0402").unwrap_err();
        assert!(
            e.starts_with("package_unknown") && e.contains("0603, 0805"),
            "{e}"
        );
    }

    #[test]
    fn each_part_passes_or_fails_for_a_reason() {
        let t = parse_table(TABLE).unwrap();
        let p = find_package(&t, "0603").unwrap();
        let parts = [
            part(1.62, 0.79, 1.0),
            part(1.95, 0.8, 0.0),
            part(1.6, 0.4, 0.0),
            part(2.4, 1.3, 0.0),
        ];
        let j = judge(&parts, p, 0.1, 0.0).unwrap();
        let reasons: Vec<Vec<&str>> = j.iter().map(|x| x.reasons.clone()).collect();
        assert_eq!(
            reasons,
            vec![
                vec![],
                vec![REASON_LENGTH],
                vec![REASON_WIDTH],
                vec![REASON_LENGTH, REASON_WIDTH]
            ]
        );
        assert!(j[0].passes() && !j[1].passes());
        assert!((j[1].delta_long_mm - 0.35).abs() < 1e-9 && j[2].delta_short_mm < -0.39);
        assert_eq!(summary(&j), "4 parts: 1 pass, 3 fail");
        // the tolerance of the package wins over the one asked for, and a wider one lets more through
        let sot = find_package(&t, "SOT-23").unwrap();
        assert!(judge(&[part(3.15, 1.3, 0.0)], sot, 0.1, 0.0).unwrap()[0].passes());
        assert!(!judge(&[part(3.15, 1.3, 0.0)], p, 0.1, 0.0).unwrap()[0].passes());
        assert!(judge(&parts[1..2], p, 0.4, 0.0).unwrap()[0].passes());
    }

    #[test]
    fn the_turn_is_checked_only_when_a_limit_is_given() {
        let t = parse_table(TABLE).unwrap();
        let p = find_package(&t, "0805").unwrap();
        let turned = [part(2.0, 1.25, 12.0), part(2.0, 1.25, -90.0 + 1.0)];
        assert!(judge(&turned, p, 0.1, 0.0)
            .unwrap()
            .iter()
            .all(Judged::passes));
        let j = judge(&turned, p, 0.1, 5.0).unwrap();
        assert_eq!(j[0].reasons, vec![REASON_ANGLE]);
        assert_eq!(j[1].reasons, vec![REASON_ANGLE]);
        assert!(judge(&turned[..1], p, 0.1, 15.0).unwrap()[0].passes());
    }

    #[test]
    fn what_makes_no_sense_in_the_limits_is_refused() {
        let t = parse_table(TABLE).unwrap();
        let p = &t[0];
        for tolerance in [0.0, -0.1, f64::NAN, f64::INFINITY] {
            assert!(judge(&[], p, tolerance, 0.0).is_err());
        }
        for angle in [-1.0, 91.0, f64::NAN] {
            assert!(judge(&[], p, 0.1, angle).is_err());
        }
    }

    #[test]
    fn the_verdicts_are_written_as_csv_and_json() {
        let t = parse_table(TABLE).unwrap();
        let p = find_package(&t, "odd, name").unwrap();
        let j = judge(&[part(5.02, 4.7, 0.0), part(5.0, 5.0, 3.0)], p, 0.1, 0.0).unwrap();
        let csv = to_csv(&j, p);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], CSV_HEADER);
        assert_eq!(
            lines[1],
            "1,1.0000,-2.0000,0.000,5.0200,4.7000,\"odd, name\",5.0000,5.0000,0.0200,-0.3000,fail,width"
        );
        assert!(lines[2].ends_with(",pass,"), "{}", lines[2]);
        let json = to_json(&j, p, 0.1);
        assert!(
            json.contains("\"format\": \"eh3d-vision-inspection\""),
            "{json}"
        );
        assert!(
            json.contains("\"name\": \"odd, name\"")
                && json.contains("\"pass\": 1,\n  \"fail\": 1"),
            "{json}"
        );
        assert!(
            json.contains("\"verdict\": \"fail\", \"reasons\": [\"width\"]"),
            "{json}"
        );
        assert!(
            json.contains("\"verdict\": \"pass\", \"reasons\": []"),
            "{json}"
        );
        assert!(to_json(&[], p, 0.1).ends_with("\"parts\": []\n}\n"));
        // a name with a quote or a backslash does not break the JSON
        assert_eq!(json_string("a\"b\\c\n"), "\"a\\\"b\\\\c\\u000a\"");
    }
}
