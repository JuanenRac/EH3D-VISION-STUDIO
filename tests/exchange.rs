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
