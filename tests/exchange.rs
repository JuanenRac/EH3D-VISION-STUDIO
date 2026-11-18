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
