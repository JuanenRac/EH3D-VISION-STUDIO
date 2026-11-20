//! The commands `orientation`, `footprint`, `sharpmap` and `board`, run as a program on pictures that the tests draw: what they print, what they write, their exit codes and what they refuse.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "eh3d_vision_analysis_{}_{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs the command line in `dir`, where there is no `vision.cfg` unless the test writes one.
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

fn pgm(w: usize, h: usize, data: &[u8]) -> Vec<u8> {
    let mut bytes = format!("P5\n{w} {h}\n255\n").into_bytes();
    bytes.extend_from_slice(data);
    bytes
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> String {
    fs::write(dir.join(name), bytes).unwrap();
    name.to_string()
}

fn fill(data: &mut [u8], w: usize, x0: usize, y0: usize, x1: usize, y1: usize, v: u8) {
    for y in y0..y1 {
        for x in x0..x1 {
            data[y * w + x] = v;
        }
    }
}

/// A dark body of 60 by 30 pixels on a light board with a mark on it at the left or at the right of its middle.
fn marked_part(dot_at_left: bool) -> Vec<u8> {
    let (w, h) = (100usize, 80usize);
    let mut data = vec![210u8; w * h];
    fill(&mut data, w, 20, 25, 80, 55, 60);
    if dot_at_left {
        fill(&mut data, w, 26, 37, 32, 43, 230);
    } else {
        fill(&mut data, w, 68, 37, 74, 43, 230);
    }
    pgm(w, h, &data)
}

#[test]
fn orientation_says_which_way_the_mark_points_and_the_exit_code_is_the_verdict() {
    let dir = scratch("orientation");
    let left = write(&dir, "left.pgm", &marked_part(true));
    let right = write(&dir, "right.pgm", &marked_part(false));
    let o = run(&dir, &["orientation", &left, "--mm-per-px", "0.1"]);
    assert!(o.status.success(), "{}", err(&o));
    let text = out(&o);
    let row: Vec<&str> = text.lines().nth(1).unwrap().split(',').collect();
    assert_eq!(row[11], "pass");
    assert_eq!(row[6], "180", "{text}");
    let direction: f64 = row[5].parse().unwrap();
    assert!((direction.abs() - 180.0).abs() < 3.0, "{text}");
    assert!(err(&o).contains("pointing"));
    // the wanted direction
    let o = run(
        &dir,
        &[
            "orientation",
            &left,
            "--mm-per-px",
            "0.1",
            "--expect-deg",
            "180",
        ],
    );
    assert!(o.status.success(), "{}", err(&o));
    let o = run(
        &dir,
        &[
            "orientation",
            &right,
            "--mm-per-px",
            "0.1",
            "--expect-deg",
            "180",
        ],
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(out(&o).contains("fail,wrong_way"), "{}", out(&o));
    let o = run(
        &dir,
        &[
            "orientation",
            &right,
            "--mm-per-px",
            "0.1",
            "--expect-deg",
            "0",
            "--format",
            "json",
        ],
    );
    assert!(o.status.success(), "{}", err(&o));
    assert!(out(&o).contains("\"format\": \"eh3d-vision-orientation\""));
    assert!(out(&o).contains("\"verdict\": \"pass\""));
}

#[test]
fn orientation_takes_the_scale_and_the_limits_from_the_file_and_refuses_what_makes_no_sense() {
    let dir = scratch("orientation_cfg");
    let left = write(&dir, "left.pgm", &marked_part(true));
    let o = run(&dir, &["orientation", &left]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        err(&o).contains("orientation needs the scale"),
        "{}",
        err(&o)
    );
    fs::write(
        dir.join("vision.cfg"),
        "mm_per_px = 0.1\norientation_mark_contrast = 250\n",
    )
    .unwrap();
    // with a mark contrast that nothing reaches there is no mark
    let o = run(&dir, &["orientation", &left]);
    assert_eq!(o.status.code(), Some(1));
    assert!(out(&o).contains("fail,no_mark"), "{}", out(&o));
    // the option wins over the file
    let o = run(&dir, &["orientation", &left, "--mark-contrast", "40"]);
    assert!(o.status.success(), "{}", err(&o));
    let flat = write(&dir, "flat.pgm", &pgm(30, 30, &[100; 900]));
    let o = run(&dir, &["orientation", &flat]);
    assert!(err(&o).contains("no_part"), "{}", err(&o));
    for args in [
        vec!["orientation"],
        vec!["orientation", &left, "--expect-deg"],
        vec!["orientation", &left, "--expect-deg", "north"],
        vec!["orientation", &left, "--tolerance", "95"],
        vec!["orientation", &left, "--format", "xml"],
        vec!["orientation", &left, "extra.pgm"],
        vec!["orientation", &left, "--nonsense"],
    ] {
        assert_eq!(run(&dir, &args).status.code(), Some(1), "{args:?}");
    }
}

/// A picture of 120 by 60 with `count` dark pads of 10 by 22 pixels, the centres `pitch` pixels apart.
fn pads(count: usize, pitch: usize, last_shorter: bool) -> Vec<u8> {
    let (w, h) = (120usize, 60usize);
    let mut data = vec![220u8; w * h];
    for k in 0..count {
        let x0 = 8 + k * pitch;
        let top = if last_shorter && k + 1 == count {
            24
        } else {
            19
        };
        fill(&mut data, w, x0, top, x0 + 10, 41, 30);
    }
    pgm(w, h, &data)
}

#[test]
fn footprint_judges_each_pad_against_the_design() {
    let dir = scratch("footprint");
    let good = write(&dir, "good.pgm", &pads(4, 15, false));
    let short = write(&dir, "short.pgm", &pads(4, 15, true));
    let base = ["footprint", good.as_str(), "--mm-per-px", "0.04"];
    let mut args = base.to_vec();
    args.extend(["--pad-length", "0.88", "--pad-width", "0.4"]);
    let o = run(&dir, &args);
    assert!(o.status.success(), "{}", err(&o));
    let text = out(&o);
    assert_eq!(text.lines().count(), 5, "{text}");
    assert!(text.lines().skip(1).all(|l| l.contains(",pass,")), "{text}");
    assert!(err(&o).contains("4 pads, 4 good"), "{}", err(&o));
    // the last pad is 17 pixels long instead of 22: 0.2 mm short
    let o = run(
        &dir,
        &[
            "footprint",
            &short,
            "--mm-per-px",
            "0.04",
            "--pad-length",
            "0.88",
            "--pad-width",
            "0.4",
        ],
    );
    assert_eq!(o.status.code(), Some(1));
    let text = out(&o);
    assert!(
        text.lines().nth(4).unwrap().ends_with("fail,length"),
        "{text}"
    );
    assert!(err(&o).contains("3 good"), "{}", err(&o));
    // a wider tolerance accepts it, and a count that is wrong refuses the lot
    let o = run(
        &dir,
        &[
            "footprint",
            &short,
            "--mm-per-px",
            "0.04",
            "--pad-length",
            "0.88",
            "--pad-width",
            "0.4",
            "--tolerance",
            "0.3",
        ],
    );
    assert!(o.status.success(), "{}", err(&o));
    let o = run(
        &dir,
        &[
            "footprint",
            &good,
            "--mm-per-px",
            "0.04",
            "--pad-length",
            "0.88",
            "--pad-width",
            "0.4",
            "--count",
            "5",
            "--format",
            "json",
        ],
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(out(&o).contains("\"reasons\": [\"count\"]"), "{}", out(&o));
}

#[test]
fn footprint_refuses_what_it_cannot_judge() {
    let dir = scratch("footprint_bad");
    let good = write(&dir, "good.pgm", &pads(4, 15, false));
    for args in [
        vec!["footprint", &good, "--mm-per-px", "0.04"],
        vec![
            "footprint",
            &good,
            "--mm-per-px",
            "0.04",
            "--pad-length",
            "0.88",
        ],
        vec![
            "footprint",
            &good,
            "--pad-length",
            "0.88",
            "--pad-width",
            "0.4",
        ],
        vec![
            "footprint",
            &good,
            "--mm-per-px",
            "0.04",
            "--pad-length",
            "0.3",
            "--pad-width",
            "0.4",
        ],
        vec![
            "footprint",
            &good,
            "--mm-per-px",
            "0.04",
            "--pad-length",
            "0.88",
            "--pad-width",
            "0.4",
            "--tolerance",
            "0",
        ],
        vec![
            "footprint",
            &good,
            "--mm-per-px",
            "0.04",
            "--pad-length",
            "0.88",
            "--pad-width",
            "0.4",
            "--count",
            "0",
        ],
    ] {
        assert_eq!(run(&dir, &args).status.code(), Some(1), "{args:?}");
    }
    let empty = write(&dir, "empty.pgm", &pgm(40, 40, &[200; 1600]));
    let o = run(
        &dir,
        &[
            "footprint",
            &empty,
            "--mm-per-px",
            "0.04",
            "--pad-length",
            "0.88",
            "--pad-width",
            "0.4",
        ],
    );
    assert!(err(&o).contains("no_pads"), "{}", err(&o));
}

/// A picture of a pattern whose contrast fades towards the right (`fade`) or stays the same.
fn pattern(fade: bool) -> Vec<u8> {
    let (w, h) = (64usize, 64usize);
    let mut data = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let c = if fade { 120.0 - x as f64 * 1.6 } else { 100.0 };
            let sign = if (x + y) % 2 == 0 { 1.0 } else { -1.0 };
            data[y * w + x] = (128.0 + sign * c / 2.0).round() as u8;
        }
    }
    pgm(w, h, &data)
}

#[test]
fn sharpmap_gives_a_cell_for_every_place_and_writes_the_heat_picture() {
    let dir = scratch("sharpmap");
    let even = write(&dir, "even.pgm", &pattern(false));
    let blur = write(&dir, "blur.pgm", &pattern(true));
    let o = run(&dir, &["sharpmap", &even, "--cells", "4"]);
    assert!(o.status.success(), "{}", err(&o));
    let text = out(&o);
    assert_eq!(
        text.lines().next().unwrap(),
        "col,row,x0,y0,x1,y1,sharpness,contrast,state"
    );
    assert_eq!(text.lines().count(), 17, "{text}");
    assert!(err(&o).contains("16 cells, 0 weak"), "{}", err(&o));
    let o = run(
        &dir,
        &[
            "sharpmap", &blur, "--cells", "4", "--heat", "heat.pgm", "--format", "json",
        ],
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(out(&o).contains("\"reasons\": [\"uneven\"]"), "{}", out(&o));
    let heat = fs::read(dir.join("heat.pgm")).unwrap();
    assert!(heat.starts_with(b"P5\n64 64\n255\n"));
    assert_eq!(heat.len(), "P5\n64 64\n255\n".len() + 64 * 64);
    // never over a file that is there, unless it is asked for
    let o = run(
        &dir,
        &["sharpmap", &blur, "--cells", "4", "--heat", "heat.pgm"],
    );
    assert!(err(&o).contains("file_exists"), "{}", err(&o));
    let o = run(
        &dir,
        &[
            "sharpmap",
            &blur,
            "--cells",
            "4",
            "--heat",
            "heat.pgm",
            "--replace",
        ],
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(!err(&o).contains("file_exists"), "{}", err(&o));
    // a looser share accepts the fade
    let o = run(
        &dir,
        &["sharpmap", &blur, "--cells", "4", "--min-share", "0.01"],
    );
    assert!(o.status.success(), "{}", err(&o));
}

#[test]
fn sharpmap_refuses_a_picture_too_small_for_its_cells_and_bad_options() {
    let dir = scratch("sharpmap_bad");
    let tiny = write(&dir, "tiny.pgm", &pgm(10, 10, &[100; 100]));
    let o = run(&dir, &["sharpmap", &tiny]);
    assert!(err(&o).contains("picture_small"), "{}", err(&o));
    let even = write(&dir, "even.pgm", &pattern(false));
    for args in [
        vec!["sharpmap", &even, "--cells", "1"],
        vec!["sharpmap", &even, "--cells", "many"],
        vec!["sharpmap", &even, "--min-share", "0"],
        vec!["sharpmap", &even, "--format", "html"],
        vec!["sharpmap"],
    ] {
        assert_eq!(run(&dir, &args).status.code(), Some(1), "{args:?}");
    }
    let flat = write(&dir, "flat.pgm", &pgm(64, 64, &[128; 4096]));
    let o = run(&dir, &["sharpmap", &flat]);
    assert!(err(&o).contains("flat"), "{}", err(&o));
}

/// A board of `cols` by `rows` dark squares of 20 pixels, `pitch_x` and `pitch_y` pixels apart.
fn squares(cols: usize, rows: usize, pitch_x: usize, pitch_y: usize) -> Vec<u8> {
    let (w, h) = (40 + cols * pitch_x, 40 + rows * pitch_y);
    let mut data = vec![220u8; w * h];
    for r in 0..rows {
        for c in 0..cols {
            let (x0, y0) = (20 + c * pitch_x, 20 + r * pitch_y);
            fill(&mut data, w, x0, y0, x0 + 20, y0 + 20, 30);
        }
    }
    pgm(w, h, &data)
}

#[test]
fn board_gives_the_scale_along_each_axis_with_a_report() {
    let dir = scratch("board");
    let square = write(&dir, "board.pgm", &squares(4, 3, 30, 30));
    let o = run(
        &dir,
        &[
            "board",
            &square,
            "--pitch-mm",
            "3",
            "--square-mm",
            "2",
            "--report",
            "board.txt",
        ],
    );
    assert!(o.status.success(), "{}", err(&o));
    let text = out(&o);
    let row: Vec<&str> = text.lines().nth(1).unwrap().split(',').collect();
    assert_eq!(&row[..3], ["12", "4", "3"], "{text}");
    assert_eq!(row[13], "0.100000", "{text}");
    assert_eq!(row[14], "pass");
    assert!(
        err(&o).contains("12 squares in 4 columns and 3 rows"),
        "{}",
        err(&o)
    );
    assert!(err(&o).contains("mm_per_px = 0.100000"));
    let report = fs::read_to_string(dir.join("board.txt")).unwrap();
    assert!(report.contains("Verdict: good"), "{report}");
    let o = run(
        &dir,
        &["board", &square, "--pitch-mm", "3", "--report", "board.txt"],
    );
    assert!(err(&o).contains("file_exists"), "{}", err(&o));
    // the pixels are not square: 30 along x and 36 along y
    let stretched = write(&dir, "stretched.pgm", &squares(4, 3, 30, 36));
    let o = run(
        &dir,
        &["board", &stretched, "--pitch-mm", "3", "--format", "json"],
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(out(&o).contains("\"reasons\": [\"aspect\"]"), "{}", out(&o));
    assert!(out(&o).contains("\"format\": \"eh3d-vision-board\""));
}

#[test]
fn board_takes_the_pitch_from_the_file_and_refuses_what_is_not_a_board() {
    let dir = scratch("board_bad");
    let square = write(&dir, "board.pgm", &squares(4, 3, 30, 30));
    let o = run(&dir, &["board", &square]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("need_size"), "{}", err(&o));
    fs::write(dir.join("vision.cfg"), "pitch_mm = 3\n").unwrap();
    let o = run(&dir, &["board", &square]);
    assert!(o.status.success(), "{}", err(&o));
    let one = write(&dir, "one.pgm", &squares(1, 1, 30, 30));
    let o = run(&dir, &["board", &one]);
    assert!(err(&o).contains("few_squares"), "{}", err(&o));
    let line = write(&dir, "line.pgm", &squares(5, 1, 30, 30));
    let o = run(&dir, &["board", &line]);
    assert!(err(&o).contains("no_lattice"), "{}", err(&o));
    for args in [
        vec!["board", &square, "--pitch-mm", "-1"],
        vec!["board", &square, "--square-mm", "4"],
        vec!["board", &square, "--max-aspect", "0"],
        vec!["board", &square, "--max-spread", "x"],
        vec!["board", &square, "--format", "txt"],
        vec!["board"],
    ] {
        assert_eq!(run(&dir, &args).status.code(), Some(1), "{args:?}");
    }
}

#[test]
fn the_help_lists_the_analysis_commands() {
    let dir = scratch("help");
    let text = out(&run(&dir, &["help"]));
    for word in [
        "eh3d-vision orientation",
        "eh3d-vision footprint",
        "eh3d-vision sharpmap",
        "eh3d-vision board",
        "orientation|footprint|sharpmap|board",
    ] {
        assert!(text.contains(word), "{word}");
    }
}
