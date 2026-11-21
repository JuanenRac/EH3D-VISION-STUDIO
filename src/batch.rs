//! A folder of pictures at once: every PNG and PGM file of the folder is read within the size limit, the marks are searched for, and one line per picture says how it went (the number of
//! marks, their mean and lowest score, their mean diameter, or the code of what went wrong). The report is a CSV file that other tools and spreadsheets read.

use crate::fiducial::{find_with, Fiducial, Settings};
use crate::flatten::flatten;
use crate::image::from_bytes;
use crate::limit::read_limited;
use std::path::{Path, PathBuf};

/// The line of column names of the report.
pub const CSV_HEADER: &str = "file,status,marks,mean_score,min_score,mean_diameter_px,detail";
/// The status of a picture that was read and has marks.
pub const OK: &str = "ok";
/// The status of a picture that was read and has none.
pub const NO_MARKS: &str = "no_marks";
/// The status of a picture that is not a PNG or PGM file, or is damaged.
pub const UNREADABLE_PICTURE: &str = "picture_unreadable";
/// The error of a folder with more pictures than the limit.
pub const TOO_MANY_FILES: &str = "too_many_files";

/// How it went with one picture.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// The name of the file, without the folder.
    pub file: String,
    /// `ok`, `no_marks`, `picture_unreadable` or the code of a refusal of the reader of files (`file_too_large`, `file_unreadable`).
    pub status: String,
    pub marks: usize,
    pub mean_score: Option<f64>,
    pub min_score: Option<f64>,
    pub mean_diameter: Option<f64>,
    /// The reason, when the picture could not be used.
    pub detail: String,
}

/// A picture that was read: its row, its size and the marks that were found in it (what a report that draws them needs).
#[derive(Debug, Clone, PartialEq)]
pub struct Checked {
    pub row: Row,
    /// The size of the picture in pixels; 0 by 0 when it could not be read.
    pub width: usize,
    pub height: usize,
    pub marks: Vec<Fiducial>,
}

/// The PNG and PGM files of a folder (not the folders, not the links, not the folders inside), by name. More than `max_files` is an error with the code `too_many_files`; the folder is
/// not read further than the first file above the limit.
pub fn list_pictures(dir: &Path, max_files: usize) -> Result<Vec<PathBuf>, String> {
    let unreadable = |e: std::io::Error| format!("file_unreadable: {}: {e}", dir.display());
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(unreadable)? {
        let entry = entry.map_err(unreadable)?;
        // the type of the entry itself: a link is not followed, so it is not a file here
        if !entry.file_type().map_err(unreadable)?.is_file() {
            continue;
        }
        let path = entry.path();
        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase());
        if !matches!(extension.as_deref(), Some("png" | "pgm")) {
            continue;
        }
        if files.len() >= max_files {
            return Err(format!(
                "{TOO_MANY_FILES}: {}: more than {max_files} pictures (batch_max_files in vision.cfg)",
                dir.display()
            ));
        }
        files.push(path);
    }
    files.sort();
    Ok(files)
}

/// The code at the start of a message such as `file_too_large: name: ...`, and the rest of it. A message with no code is a damaged picture.
fn split_code(message: &str) -> (String, String) {
    match message.split_once(": ") {
        Some((code, rest))
            if !code.is_empty() && code.chars().all(|c| c.is_ascii_lowercase() || c == '_') =>
        {
            (code.to_string(), rest.to_string())
        }
        _ => (UNREADABLE_PICTURE.to_string(), message.to_string()),
    }
}

/// Reads one picture and searches it for marks. Nothing here fails: a picture that cannot be used gives a row with its status and reason.
pub fn check_picture(
    path: &Path,
    max_mib: u64,
    s: &Settings,
    threshold: Option<u8>,
    flatten_px: usize,
) -> Row {
    check_detailed(path, max_mib, s, threshold, flatten_px).row
}

/// The same as `check_picture`, with the size of the picture and the marks that were found.
pub fn check_detailed(
    path: &Path,
    max_mib: u64,
    s: &Settings,
    threshold: Option<u8>,
    flatten_px: usize,
) -> Checked {
    let file = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let failed = |message: String| {
        let (status, detail) = split_code(&message);
        Checked {
            row: Row {
                file: file.clone(),
                status,
                marks: 0,
                mean_score: None,
                min_score: None,
                mean_diameter: None,
                detail,
            },
            width: 0,
            height: 0,
            marks: Vec::new(),
        }
    };
    let bytes = match read_limited(path, max_mib) {
        Ok(b) => b,
        Err(e) => return failed(e),
    };
    let mut img = match from_bytes(&bytes) {
        Ok(i) => i,
        Err(e) => return failed(e),
    };
    if flatten_px != 0 {
        img = match flatten(&img, flatten_px) {
            Ok(i) => i,
            Err(e) => return failed(e),
        };
    }
    let found = match find_with(&img, s, threshold) {
        Ok(f) => f,
        Err(e) => return failed(e),
    };
    let n = found.len() as f64;
    Checked {
        row: Row {
            file,
            status: if found.is_empty() { NO_MARKS } else { OK }.to_string(),
            marks: found.len(),
            mean_score: (!found.is_empty()).then(|| found.iter().map(|f| f.score).sum::<f64>() / n),
            min_score: found.iter().map(|f| f.score).reduce(f64::min),
            mean_diameter: (!found.is_empty())
                .then(|| found.iter().map(|f| f.diameter).sum::<f64>() / n),
            detail: String::new(),
        },
        width: img.w,
        height: img.h,
        marks: found,
    }
}

/// A field of a CSV file: quoted when it holds a comma, a quote or a line end, and with a quote in front when it starts with a character (`=`, `+`, `-`, `@`) that makes a spreadsheet
/// run it as a formula.
pub fn csv_field(text: &str) -> String {
    let guarded = if text.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{text}")
    } else {
        text.to_string()
    };
    if guarded.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", guarded.replace('"', "\"\""))
    } else {
        guarded
    }
}

fn number(v: Option<f64>, decimals: usize) -> String {
    v.map(|v| format!("{v:.decimals$}")).unwrap_or_default()
}

/// The report: the line `CSV_HEADER` and one line per picture, with the line end `\n`; the fields that do not apply are empty.
pub fn report_csv(rows: &[Row]) -> String {
    let mut out = format!("{CSV_HEADER}\n");
    for r in rows {
        out.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            csv_field(&r.file),
            csv_field(&r.status),
            r.marks,
            number(r.mean_score, 3),
            number(r.min_score, 3),
            number(r.mean_diameter, 2),
            csv_field(&r.detail)
        ));
    }
    out
}

/// One line that says how many pictures there were and how they went.
pub fn summary(rows: &[Row]) -> String {
    let with = rows.iter().filter(|r| r.status == OK).count();
    let without = rows.iter().filter(|r| r.status == NO_MARKS).count();
    format!(
        "{} pictures: {with} with marks, {without} without, {} that could not be used",
        rows.len(),
        rows.len() - with - without
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::Gray;
    use crate::sim::to_pgm;
    use std::fs;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("eh3d_vision_batch_{}_{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn with_marks(n: usize) -> Vec<u8> {
        let mut g = Gray::blank(200, 100, 220).unwrap();
        for i in 0..n {
            g.disc(30.0 + i as f64 * 50.0, 50.0, 10.0, 30);
        }
        to_pgm(&g)
    }

    #[test]
    fn a_folder_gives_one_row_per_picture_and_the_rest_is_left_out() {
        let dir = scratch("folder");
        fs::write(dir.join("b_two.pgm"), with_marks(2)).unwrap();
        fs::write(dir.join("a_one.PGM"), with_marks(1)).unwrap();
        fs::write(
            dir.join("c_none.pgm"),
            to_pgm(&Gray::blank(50, 50, 100).unwrap()),
        )
        .unwrap();
        fs::write(dir.join("d_bad.pgm"), b"not a picture").unwrap();
        fs::write(dir.join("notes.txt"), b"left out").unwrap();
        fs::create_dir_all(dir.join("folder.pgm")).unwrap();
        let files = list_pictures(&dir, 100).unwrap();
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["a_one.PGM", "b_two.pgm", "c_none.pgm", "d_bad.pgm"]);
        let rows: Vec<Row> = files
            .iter()
            .map(|p| check_picture(p, 64, &Settings::default(), None, 0))
            .collect();
        assert_eq!(
            rows.iter()
                .map(|r| (r.status.as_str(), r.marks))
                .collect::<Vec<_>>(),
            [
                ("ok", 1),
                ("ok", 2),
                ("no_marks", 0),
                ("picture_unreadable", 0)
            ]
        );
        assert!(
            rows[1].mean_score.unwrap() > 0.9
                && (rows[1].mean_diameter.unwrap() - 20.0).abs() < 1.5
        );
        assert!(
            rows[2].mean_score.is_none() && rows[3].detail.contains("PGM"),
            "{:?}",
            rows[3]
        );
        let csv = report_csv(&rows);
        assert_eq!(csv.lines().next().unwrap(), CSV_HEADER);
        assert_eq!(csv.lines().count(), 5);
        assert!(
            csv.lines()
                .nth(3)
                .unwrap()
                .starts_with("c_none.pgm,no_marks,0,,,,"),
            "{csv}"
        );
        assert_eq!(
            summary(&rows),
            "4 pictures: 2 with marks, 1 without, 1 that could not be used"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_larger_than_the_limit_is_a_row_with_its_code_and_a_folder_with_too_many_pictures_is_refused(
    ) {
        let dir = scratch("limits");
        for i in 0..4 {
            fs::write(dir.join(format!("p{i}.pgm")), with_marks(1)).unwrap();
        }
        let big = check_picture(&dir.join("p0.pgm"), 0, &Settings::default(), None, 0);
        assert_eq!(big.status, "file_too_large");
        assert!(big.detail.contains("p0.pgm"), "{big:?}");
        let why = list_pictures(&dir, 3).unwrap_err();
        assert!(why.starts_with("too_many_files: "), "{why}");
        assert_eq!(list_pictures(&dir, 4).unwrap().len(), 4);
        let gone = check_picture(&dir.join("missing.pgm"), 64, &Settings::default(), None, 0);
        assert_eq!(gone.status, "file_unreadable");
        let no_dir = list_pictures(&dir.join("nowhere"), 3).unwrap_err();
        assert!(no_dir.starts_with("file_unreadable: "), "{no_dir}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn uneven_light_can_be_taken_out_for_the_whole_folder_and_a_bad_window_is_a_row() {
        let dir = scratch("flat");
        fs::write(dir.join("p.pgm"), with_marks(2)).unwrap();
        let flat = check_picture(&dir.join("p.pgm"), 64, &Settings::default(), None, 61);
        assert_eq!((flat.status.as_str(), flat.marks), ("ok", 2), "{flat:?}");
        let bad = check_picture(&dir.join("p.pgm"), 64, &Settings::default(), None, 2);
        assert_eq!(bad.status, UNREADABLE_PICTURE);
        assert!(bad.detail.contains("window"), "{bad:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_fields_of_the_report_cannot_run_as_formulas_or_break_a_line() {
        assert_eq!(csv_field("plain.png"), "plain.png");
        assert_eq!(csv_field("=cmd|' /c calc'!A1"), "'=cmd|' /c calc'!A1");
        assert_eq!(csv_field("+1+1"), "'+1+1");
        assert_eq!(csv_field("-2"), "'-2");
        assert_eq!(csv_field("@sum"), "'@sum");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("two\nlines"), "\"two\nlines\"");
        let row = Row {
            file: "=x,y.png".into(),
            status: "ok".into(),
            marks: 1,
            mean_score: Some(0.5),
            min_score: Some(0.25),
            mean_diameter: Some(10.0),
            detail: String::new(),
        };
        assert_eq!(
            report_csv(&[row]).lines().nth(1).unwrap(),
            "\"'=x,y.png\",ok,1,0.500,0.250,10.00,"
        );
    }
}
