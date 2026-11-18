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
