//! The commands that work from a picture file and write what other programs read: the centre to a fraction of a pixel, the measure of parts, the batch report and the scale from a pitch.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "eh3d_vision_exchange_{}_{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs the command line in `dir`, where there is no `vision.cfg`, so the defaults are the values.
fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_eh3d-vision"))
        .args(args)
        .current_dir(dir)
        .env_remove("EH3D_VISION_CONFIG")
        .output()
        .expect("the program starts")
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).replace('\r', "")
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).replace('\r', "")
}

/// The bytes of a binary PGM picture.
fn pgm(w: usize, h: usize, data: &[u8]) -> Vec<u8> {
    let mut bytes = format!("P5\n{w} {h}\n255\n").into_bytes();
    bytes.extend_from_slice(data);
    bytes
}

/// A picture of 80 by 60 with one dark disc whose edge is grey where it cuts a pixel.
fn soft_disc_picture(cx: f64, cy: f64, r: f64) -> Vec<u8> {
    let (w, h) = (80usize, 60usize);
    let mut data = vec![220u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut hit = 0u32;
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = x as f64 - 0.5 + (f64::from(sx) + 0.5) / 4.0;
                    let py = y as f64 - 0.5 + (f64::from(sy) + 0.5) / 4.0;
                    if (px - cx).powi(2) + (py - cy).powi(2) <= r * r {
                        hit += 1;
                    }
                }
            }
            data[y * w + x] = (220.0 - 190.0 * f64::from(hit) / 16.0).round() as u8;
        }
    }
    pgm(w, h, &data)
}

/// The two numbers of the first `(x, y)` of a line of `find`.
fn first_point(text: &str) -> (f64, f64) {
    let inside = text.split('(').nth(1).unwrap().split(')').next().unwrap();
    let (x, y) = inside.split_once(',').unwrap();
    (x.trim().parse().unwrap(), y.trim().parse().unwrap())
}

#[test]
fn find_refine_prints_the_centre_to_a_fraction_of_a_pixel_and_a_confidence() {
    let dir = scratch("refine");
    fs::write(dir.join("soft.pgm"), soft_disc_picture(40.3, 30.7, 9.0)).unwrap();
    let plain = run(&dir, &["find", "soft.pgm"]);
    assert!(plain.status.success(), "{}", err(&plain));
    assert!(!out(&plain).contains("confidence"), "{}", out(&plain));
    let refined = run(&dir, &["find", "soft.pgm", "--refine"]);
    assert!(refined.status.success(), "{}", err(&refined));
    let text = out(&refined);
    assert!(text.contains(", confidence "), "{text}");
    let (x, y) = first_point(&text);
    assert!((x - 40.3).abs() < 0.11 && (y - 30.7).abs() < 0.11, "{text}");
    // the configuration file can ask for it, and a value that makes no sense is refused with a warning
    fs::write(dir.join("vision.cfg"), "refine_subpixel = yes\n").unwrap();
    assert!(out(&run(&dir, &["find", "soft.pgm"])).contains("confidence"));
    fs::write(dir.join("vision.cfg"), "full_contrast = 2\n").unwrap();
    let bad = run(&dir, &["find", "soft.pgm"]);
    assert!(err(&bad).contains("full_contrast"), "{}", err(&bad));
    let _ = fs::remove_dir_all(&dir);
}

/// A board whose light fades from 250 at the left to 60 at the right, with three dark discs that are 40 % of the light where they lie.
fn faded_board_picture() -> Vec<u8> {
    let (w, h) = (400usize, 120usize);
    let light = |x: usize| 250.0 - x as f64 * 190.0 / w as f64;
    let mut data = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            data[y * w + x] = light(x) as u8;
            for cx in [60usize, 190, 330] {
                if (x as f64 - cx as f64).powi(2) + (y as f64 - 60.0).powi(2) <= 100.0 {
                    data[y * w + x] = (0.4 * light(cx)) as u8;
                }
            }
        }
    }
    pgm(w, h, &data)
}

/// The number of marks a `find` printed.
fn marks_in(text: &str) -> usize {
    text.lines()
        .filter(|l| l.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .count()
}

#[test]
fn find_flatten_takes_uneven_light_out_before_the_search() {
    let dir = scratch("flatten");
    fs::write(dir.join("faded.pgm"), faded_board_picture()).unwrap();
    let plain = run(&dir, &["find", "faded.pgm"]);
    assert!(plain.status.success(), "{}", err(&plain));
    assert!(marks_in(&out(&plain)) < 3, "{}", out(&plain));
    let flat = run(&dir, &["find", "faded.pgm", "--flatten", "61"]);
    assert!(flat.status.success(), "{}", err(&flat));
    assert_eq!(marks_in(&out(&flat)), 3, "{}", out(&flat));
    // the configuration file can give the window, and a window that makes no sense is refused
    fs::write(dir.join("vision.cfg"), "flatten_window = 61\n").unwrap();
    assert_eq!(marks_in(&out(&run(&dir, &["find", "faded.pgm"]))), 3);
    let bad = run(&dir, &["find", "faded.pgm", "--flatten", "2"]);
    assert!(
        !bad.status.success() && err(&bad).contains("window"),
        "{}",
        err(&bad)
    );
    let word = run(&dir, &["find", "faded.pgm", "--flatten", "wide"]);
    assert!(!word.status.success() && err(&word).contains("--flatten"));
    let _ = fs::remove_dir_all(&dir);
}

/// A light board of 240 by 160 with a dark rectangle of 61 by 31 pixels centred at (100, 60).
fn part_picture() -> Vec<u8> {
    let (w, h) = (240usize, 160usize);
    let mut data = vec![220u8; w * h];
    for y in 45..=75 {
        for x in 70..=130 {
            data[y * w + x] = 30;
        }
    }
    pgm(w, h, &data)
}

#[test]
fn measure_writes_the_parts_as_csv_or_json_and_never_writes_over_a_file() {
    let dir = scratch("measure");
    fs::write(dir.join("part.pgm"), part_picture()).unwrap();
    let csv = run(&dir, &["measure", "part.pgm", "--mm-per-px", "0.5"]);
    assert!(csv.status.success(), "{}", err(&csv));
    let text = out(&csv);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert_eq!(
        lines[0],
        "index,x_mm,y_mm,angle_deg,length_mm,width_mm,area_mm2"
    );
    // the centre of the picture is (120, 80): the part is 10 mm to the left and 10 mm up
    assert!(
        lines[1].starts_with("1,-10.0000,10.0000,0.000,30.5000,15.5000,472.7500"),
        "{text}"
    );
    let json = run(
        &dir,
        &[
            "measure",
            "part.pgm",
            "--mm-per-px",
            "0.5",
            "--format",
            "json",
        ],
    );
    assert!(json.status.success(), "{}", err(&json));
    let j = out(&json);
    assert!(
        j.contains("\"format\": \"eh3d-vision-parts\"") && j.contains("\"x_mm\": -10.0000"),
        "{j}"
    );
    // a file is written only where there is none, and only over another one when asked
    let args = [
        "measure",
        "part.pgm",
        "--mm-per-px",
        "0.5",
        "--output",
        "parts.csv",
    ];
    assert!(run(&dir, &args).status.success());
    assert_eq!(
        fs::read_to_string(dir.join("parts.csv"))
            .unwrap()
            .replace('\r', ""),
        text
    );
    let again = run(&dir, &args);
    assert!(
        !again.status.success() && err(&again).starts_with("error: file_exists"),
        "{}",
        err(&again)
    );
    let mut with_replace = args.to_vec();
    with_replace.push("--replace");
    assert!(run(&dir, &with_replace).status.success());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn measure_needs_a_scale_a_known_format_and_a_picture_within_the_limits() {
    let dir = scratch("measure_bad");
    fs::write(dir.join("part.pgm"), part_picture()).unwrap();
    let no_scale = run(&dir, &["measure", "part.pgm"]);
    assert!(
        !no_scale.status.success() && err(&no_scale).contains("scale"),
        "{}",
        err(&no_scale)
    );
    // the scale can come from the configuration file
    fs::write(dir.join("vision.cfg"), "mm_per_px = 0.5\n").unwrap();
    assert!(run(&dir, &["measure", "part.pgm"]).status.success());
    let format = run(&dir, &["measure", "part.pgm", "--format", "xml"]);
    assert!(
        !format.status.success() && err(&format).contains("csv or json"),
        "{}",
        err(&format)
    );
    // a part has to be larger than part_min_area, and a picture with more parts than max_parts is refused
    fs::write(
        dir.join("vision.cfg"),
        "mm_per_px = 0.5\npart_min_area = 5000\n",
    )
    .unwrap();
    assert_eq!(out(&run(&dir, &["measure", "part.pgm"])).lines().count(), 1);
    fs::write(
        dir.join("vision.cfg"),
        "mm_per_px = 0.5\npart_min_area = 5\nmax_parts = 1\n",
    )
    .unwrap();
    let mut data = vec![220u8; 240 * 160];
    for (x0, y0) in [(20usize, 20usize), (100, 20), (180, 100)] {
        for y in y0..y0 + 12 {
            for x in x0..x0 + 12 {
                data[y * 240 + x] = 30;
            }
        }
    }
    fs::write(dir.join("many.pgm"), pgm(240, 160, &data)).unwrap();
    let many = run(&dir, &["measure", "many.pgm"]);
    assert!(
        !many.status.success() && err(&many).contains("too_many_parts"),
        "{}",
        err(&many)
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A board of 240 by 140 with one dark cross of 31 pixels centred at (60, 50) and one dark disc.
fn cross_picture() -> Vec<u8> {
    let (w, h) = (240usize, 140usize);
    let mut data = vec![220u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = (x.abs_diff(60), y.abs_diff(50));
            let in_cross = dx <= 15 && dy <= 15 && (dx <= 2 || dy <= 2);
            let in_disc = (x as f64 - 170.0).powi(2) + (y as f64 - 80.0).powi(2) <= 225.0;
            if in_cross || in_disc {
                data[y * w + x] = 30;
            }
        }
    }
    pgm(w, h, &data)
}

#[test]
fn crosses_finds_the_plus_signs_and_not_the_discs() {
    let dir = scratch("crosses");
    fs::write(dir.join("marks.pgm"), cross_picture()).unwrap();
    let found = run(&dir, &["crosses", "marks.pgm"]);
    assert!(found.status.success(), "{}", err(&found));
    let text = out(&found);
    assert_eq!(marks_in(&text), 1, "{text}");
    assert!(
        text.starts_with("1: (60.00, 50.00) px, size 31 px, arm 5.0 px, score "),
        "{text}"
    );
    // the discs are still the business of find
    assert_eq!(marks_in(&out(&run(&dir, &["find", "marks.pgm"]))), 1);
    // the range of the size is an option, and a bad one is refused
    let none = run(&dir, &["crosses", "marks.pgm", "--min-size", "40"]);
    assert!(out(&none).contains("no crosses found"), "{}", out(&none));
    let bad = run(&dir, &["crosses", "marks.pgm", "--min-size", "3"]);
    assert!(
        !bad.status.success() && err(&bad).contains("crosses"),
        "{}",
        err(&bad)
    );
    fs::write(dir.join("vision.cfg"), "cross_min_size = 40\n").unwrap();
    assert!(out(&run(&dir, &["crosses", "marks.pgm"])).contains("no crosses"));
    let _ = fs::remove_dir_all(&dir);
}

/// A light board of 260 by 200 with a grid of dark discs 40 pixels apart (5 columns, 4 rows).
fn target_picture(columns: usize, rows: usize, skip: Option<(usize, usize)>) -> Vec<u8> {
    let (w, h) = (260usize, 200usize);
    let mut data = vec![220u8; w * h];
    for r in 0..rows {
        for c in 0..columns {
            if skip == Some((c, r)) {
                continue;
            }
            let (cx, cy) = (40.0 + c as f64 * 40.0, 40.0 + r as f64 * 40.0);
            for y in 0..h {
                for x in 0..w {
                    if (x as f64 - cx).powi(2) + (y as f64 - cy).powi(2) <= 64.0 {
                        data[y * w + x] = 30;
                    }
                }
            }
        }
    }
    pgm(w, h, &data)
}

#[test]
fn calibrate_gives_the_scale_from_the_pitch_of_a_target() {
    let dir = scratch("calibrate");
    fs::write(dir.join("target.pgm"), target_picture(5, 4, None)).unwrap();
    let ok = run(&dir, &["calibrate", "target.pgm", "--pitch-mm", "2"]);
    assert!(ok.status.success(), "{}", err(&ok));
    let text = out(&ok);
    assert!(
        text.starts_with("20 marks, 2 mm apart: 0.050000 mm per pixel"),
        "{text}"
    );
    assert!(text.lines().any(|l| l == "mm_per_px = 0.050000"), "{text}");
    // a mark that is missing does not change the answer
    fs::write(dir.join("hole.pgm"), target_picture(5, 4, Some((2, 1)))).unwrap();
    let hole = run(&dir, &["calibrate", "hole.pgm", "--pitch-mm", "2"]);
    assert!(
        out(&hole).starts_with("19 marks"),
        "{}{}",
        out(&hole),
        err(&hole)
    );
    // the line it prints is a line the configuration file reads, and measure then needs no option
    fs::write(dir.join("vision.cfg"), "mm_per_px = 0.05\n").unwrap();
    assert!(run(&dir, &["measure", "target.pgm"]).status.success());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn calibrate_needs_the_pitch_and_enough_marks() {
    let dir = scratch("calibrate_bad");
    fs::write(dir.join("target.pgm"), target_picture(5, 4, None)).unwrap();
    let none = run(&dir, &["calibrate", "target.pgm"]);
    assert!(
        !none.status.success() && err(&none).contains("--pitch-mm"),
        "{}",
        err(&none)
    );
    let zero = run(&dir, &["calibrate", "target.pgm", "--pitch-mm", "0"]);
    assert!(
        !zero.status.success() && err(&zero).contains("more than 0"),
        "{}",
        err(&zero)
    );
    fs::write(dir.join("two.pgm"), target_picture(2, 1, None)).unwrap();
    let few = run(&dir, &["calibrate", "two.pgm", "--pitch-mm", "2"]);
    assert!(
        !few.status.success() && err(&few).contains("at least 4"),
        "{}",
        err(&few)
    );
    let tight = run(
        &dir,
        &[
            "calibrate",
            "target.pgm",
            "--pitch-mm",
            "2",
            "--tolerance",
            "-1",
        ],
    );
    assert!(
        !tight.status.success() && err(&tight).contains("tolerance"),
        "{}",
        err(&tight)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn batch_reports_every_picture_of_a_folder_and_does_not_write_over_a_report() {
    let dir = scratch("batch");
    let pictures = dir.join("pictures");
    fs::create_dir_all(&pictures).unwrap();
    fs::write(pictures.join("one.pgm"), soft_disc_picture(40.0, 30.0, 9.0)).unwrap();
    fs::write(pictures.join("two.pgm"), part_picture()).unwrap();
    fs::write(pictures.join("flat.pgm"), pgm(20, 20, &[100u8; 400])).unwrap();
    fs::write(pictures.join("readme.txt"), "left out").unwrap();
    // to the standard output: the CSV, and the summary on the error output
    let shown = run(&dir, &["batch", "pictures"]);
    assert!(shown.status.success(), "{}", err(&shown));
    let text = out(&shown);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 4, "{text}");
    assert_eq!(
        lines[0],
        "file,status,marks,mean_score,min_score,mean_diameter_px,detail"
    );
    assert!(lines[1].starts_with("flat.pgm,no_marks,0,,,,"), "{text}");
    assert!(lines[2].starts_with("one.pgm,ok,1,"), "{text}");
    assert!(
        err(&shown).contains("3 pictures: 1 with marks, 2 without"),
        "{}",
        err(&shown)
    );
    // to a report file
    let args = ["batch", "pictures", "--report", "report.csv"];
    let written = run(&dir, &args);
    assert!(written.status.success(), "{}", err(&written));
    assert!(
        out(&written).contains("the report is in report.csv"),
        "{}",
        out(&written)
    );
    assert_eq!(
        fs::read_to_string(dir.join("report.csv"))
            .unwrap()
            .replace('\r', ""),
        text
    );
    let again = run(&dir, &args);
    assert!(
        !again.status.success() && err(&again).contains("file_exists"),
        "{}",
        err(&again)
    );
    let mut with_replace = args.to_vec();
    with_replace.push("--replace");
    assert!(run(&dir, &with_replace).status.success());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn batch_says_what_went_wrong_with_a_picture_and_refuses_what_is_too_big_or_too_many() {
    let dir = scratch("batch_bad");
    let pictures = dir.join("pictures");
    fs::create_dir_all(&pictures).unwrap();
    fs::write(
        pictures.join("good.pgm"),
        soft_disc_picture(40.0, 30.0, 9.0),
    )
    .unwrap();
    fs::write(pictures.join("broken.pgm"), "P5\n10 10\n255\nshort").unwrap();
    let failed = run(&dir, &["batch", "pictures", "--report", "r.csv"]);
    // the report is written, and the exit code tells a script that a picture could not be used
    assert!(
        !failed.status.success() && err(&failed).contains("could not be used"),
        "{}",
        err(&failed)
    );
    let report = fs::read_to_string(dir.join("r.csv")).unwrap();
    assert!(
        report.contains("broken.pgm,picture_unreadable,0,,,,"),
        "{report}"
    );
    assert!(report.contains("good.pgm,ok,1,"), "{report}");
    fs::write(dir.join("vision.cfg"), "batch_max_files = 1\n").unwrap();
    let many = run(&dir, &["batch", "pictures"]);
    assert!(
        !many.status.success() && err(&many).contains("too_many_files"),
        "{}",
        err(&many)
    );
    fs::write(dir.join("vision.cfg"), "max_file_mib = 1\n").unwrap();
    fs::write(pictures.join("huge.pgm"), vec![0u8; 1_048_577]).unwrap();
    let big = run(&dir, &["batch", "pictures", "--report", "r2.csv"]);
    assert!(!big.status.success());
    let report = fs::read_to_string(dir.join("r2.csv")).unwrap();
    assert!(report.contains("huge.pgm,file_too_large,0,,,,"), "{report}");
    let none = run(&dir, &["batch", "nowhere"]);
    assert!(
        !none.status.success() && err(&none).contains("file_unreadable"),
        "{}",
        err(&none)
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A board of 240 by 140 with a dark square of 25 pixels turned by 30 degrees at (60, 60), a dark cross of 33 pixels turned by 20 degrees at (170, 70), and a dark disc at (120, 110).
fn turned_picture() -> Vec<u8> {
    let (w, h) = (240usize, 140usize);
    let mut data = vec![220u8; w * h];
    let (sq_sin, sq_cos) = 30f64.to_radians().sin_cos();
    let (cr_sin, cr_cos) = 20f64.to_radians().sin_cos();
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f64, y as f64);
            let (dx, dy) = (fx - 60.0, 60.0 - fy);
            let (u, v) = (
                (dx * sq_cos + dy * sq_sin).abs(),
                (-dx * sq_sin + dy * sq_cos).abs(),
            );
            let in_square = u <= 12.0 && v <= 12.0;
            let (dx, dy) = (fx - 170.0, 70.0 - fy);
            let (u, v) = (
                (dx * cr_cos + dy * cr_sin).abs(),
                (-dx * cr_sin + dy * cr_cos).abs(),
            );
            let in_cross = (u <= 16.0 && v <= 2.0) || (v <= 16.0 && u <= 2.0);
            let in_disc = (fx - 120.0).powi(2) + (fy - 110.0).powi(2) <= 144.0;
            if in_square || in_cross || in_disc {
                data[y * w + x] = 30;
            }
        }
    }
    pgm(w, h, &data)
}

#[test]
fn squares_and_turned_crosses_are_found_with_their_angle() {
    let dir = scratch("turned");
    fs::write(dir.join("turned.pgm"), turned_picture()).unwrap();
    let squares = run(&dir, &["squares", "turned.pgm"]);
    assert!(squares.status.success(), "{}", err(&squares));
    let text = out(&squares);
    assert_eq!(marks_in(&text), 1, "{text}");
    assert!(text.starts_with("1: (60.0"), "{text}");
    assert!(
        text.contains("angle +30.") || text.contains("angle +29."),
        "{text}"
    );
    let crosses = run(&dir, &["crosses", "turned.pgm", "--turned"]);
    assert!(crosses.status.success(), "{}", err(&crosses));
    let text = out(&crosses);
    assert_eq!(marks_in(&text), 1, "{text}");
    assert!(text.starts_with("1: (170.0"), "{text}");
    assert!(
        text.contains("angle +20.") || text.contains("angle +19."),
        "{text}"
    );
    // the cross whose arms are along the axes is still the business of crosses without --turned: the turned one is left out
    let plain = run(&dir, &["crosses", "turned.pgm"]);
    assert!(out(&plain).contains("no crosses found"), "{}", out(&plain));
    // the tolerances are in the configuration file
    fs::write(
        dir.join("vision.cfg"),
        "square_min_fill = 1\ncross_min_size = 40\n",
    )
    .unwrap();
    assert!(out(&run(&dir, &["squares", "turned.pgm"])).contains("no squares found"));
    assert!(out(&run(&dir, &["crosses", "turned.pgm", "--turned"])).contains("no crosses found"));
    fs::remove_file(dir.join("vision.cfg")).unwrap();
    let bad = run(&dir, &["squares", "turned.pgm", "--min-size", "3"]);
    assert!(
        !bad.status.success() && err(&bad).contains("squares and crosses"),
        "{}",
        err(&bad)
    );
    let missing = run(&dir, &["squares"]);
    assert!(!missing.status.success() && err(&missing).contains("squares needs an image"));
    let _ = fs::remove_dir_all(&dir);
}
