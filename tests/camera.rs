//! The commands `edge`, `histogram`, `refcheck`, `lens --residuals` and the commands `server check`, `server discover` and `server send fiducials|focus|blobs`, run as a program on pictures that
//! the tests draw: what they print, what they write, their exit codes and what they refuse.

use eh3d_vision_studio::convention::{document, Finding, Source};
use eh3d_vision_studio::image::Gray;
use eh3d_vision_studio::sim::to_pgm;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("eh3d_vision_camera_{}_{name}", std::process::id()));
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

fn save(dir: &Path, name: &str, g: &Gray) -> String {
    fs::write(dir.join(name), to_pgm(g)).unwrap();
    name.to_string()
}

/// A vertical edge at `x0` (between the pixels, with the share of area that a pixel holds on each side) from the grey level 40 on the left to 210 on the right.
fn edge_picture(x0: f64) -> Gray {
    let (w, h) = (64usize, 32usize);
    let mut data = Vec::new();
    for _ in 0..h {
        for x in 0..w {
            let cover = (x as f64 - x0 + 0.5).clamp(0.0, 1.0);
            data.push((40.0 + 170.0 * cover).round() as u8);
        }
    }
    Gray::new(w, h, data).unwrap()
}

fn patched(mut g: Gray, x0: usize, y0: usize, side: usize, v: u8) -> Gray {
    for y in y0..y0 + side {
        for x in x0..x0 + side {
            g.data[y * g.w + x] = v;
        }
    }
    g
}

#[test]
fn the_edge_is_found_to_a_fraction_of_a_pixel_and_written_as_csv_and_json() {
    let dir = scratch("edge");
    let pic = save(&dir, "edge.pgm", &edge_picture(30.4));
    let o = run(&dir, &["edge", &pic, "--from", "10,16", "--to", "50,16"]);
    assert!(o.status.success(), "{}", err(&o));
    let text = out(&o);
    let mut lines = text.lines();
    assert!(lines.next().unwrap().starts_with("position_px,x,y,rising"));
    let row: Vec<&str> = lines.next().unwrap().split(',').collect();
    let x: f64 = row[1].parse().unwrap();
    assert!((x - 30.4).abs() < 0.1, "{x}");
    assert_eq!(row[3], "true");
    assert_eq!(row[10], "sure");
    assert!(err(&o).contains("uncertainty"));
    let o = run(
        &dir,
        &[
            "edge", &pic, "--from", "10,16", "--to", "50,16", "--format", "json",
        ],
    );
    assert!(o.status.success());
    assert!(out(&o).contains("\"format\": \"eh3d-vision-edge\""));
    // a limit that nothing can meet: the result is written and the exit code is 1
    let o = run(
        &dir,
        &[
            "edge",
            &pic,
            "--from",
            "10,16",
            "--to",
            "50,16",
            "--max-uncertainty",
            "0.0001",
        ],
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(out(&o).contains("unsure"));
    assert!(err(&o).contains("not sure enough"));
}

#[test]
fn an_edge_that_is_not_there_or_a_line_that_does_not_fit_is_refused() {
    let dir = scratch("edge_bad");
    let flat = save(&dir, "flat.pgm", &Gray::blank(64, 32, 90).unwrap());
    let o = run(&dir, &["edge", &flat, "--from", "10,16", "--to", "50,16"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("edge_flat"), "{}", err(&o));
    let pic = save(&dir, "edge.pgm", &edge_picture(30.4));
    for args in [
        vec!["edge", pic.as_str(), "--to", "50,16"],
        vec!["edge", pic.as_str(), "--from", "10,16"],
        vec!["edge", pic.as_str(), "--from", "10;16", "--to", "50,16"],
        vec!["edge", pic.as_str(), "--from", "10,16", "--to", "500,16"],
        vec![
            "edge",
            pic.as_str(),
            "--from",
            "10,16",
            "--to",
            "50,16",
            "--step",
            "5",
        ],
        vec![
            "edge",
            pic.as_str(),
            "--from",
            "10,16",
            "--to",
            "50,16",
            "--format",
            "xml",
        ],
        vec![
            "edge",
            pic.as_str(),
            "--from",
            "10,16",
            "--to",
            "50,16",
            "--sideways",
        ],
        vec!["edge", "--from", "10,16", "--to", "50,16"],
    ] {
        let o = run(&dir, &args);
        assert_eq!(o.status.code(), Some(1), "{args:?}");
        assert!(err(&o).starts_with("error: "), "{args:?}: {}", err(&o));
    }
}

#[test]
fn the_histogram_groups_the_levels_and_says_where_the_cuts_are() {
    let dir = scratch("hist");
    let mut g = Gray::blank(40, 30, 200).unwrap();
    g = patched(g, 5, 5, 12, 30);
    let pic = save(&dir, "p.pgm", &g);
    let o = run(&dir, &["histogram", &pic, "--bins", "8"]);
    assert!(o.status.success(), "{}", err(&o));
    let text = out(&o);
    let rows: Vec<&str> = text.lines().skip(1).collect();
    assert_eq!(rows.len(), 8);
    let total: u64 = rows
        .iter()
        .map(|r| r.split(',').nth(2).unwrap().parse::<u64>().unwrap())
        .sum();
    assert_eq!(total, 1200);
    assert!(err(&o).contains("Otsu"));
    let o = run(&dir, &["histogram", &pic, "--format", "json"]);
    assert!(o.status.success());
    let json = out(&o);
    assert!(json.contains("\"format\": \"eh3d-vision-histogram\""));
    assert!(json.contains("\"pixels\": 1200"));
    assert!(json.contains("\"bins\": 16"));
    for args in [
        vec!["histogram", pic.as_str(), "--bins", "10"],
        vec!["histogram", pic.as_str(), "--bins", "many"],
        vec!["histogram", pic.as_str(), "--format", "xml"],
        vec!["histogram"],
    ] {
        let o = run(&dir, &args);
        assert_eq!(o.status.code(), Some(1), "{args:?}");
    }
}

#[test]
fn a_picture_is_checked_against_a_reference_through_a_mask_of_tolerance() {
    let dir = scratch("ref");
    let base = Gray::blank(40, 30, 120).unwrap();
    let reference = save(&dir, "ref.pgm", &base);
    let same = save(&dir, "same.pgm", &base);
    let changed = save(&dir, "changed.pgm", &patched(base.clone(), 6, 6, 8, 230));
    let o = run(&dir, &["refcheck", &reference, &same]);
    assert!(o.status.success(), "{}", err(&o));
    assert!(out(&o).trim_end().ends_with("same"));
    // the change is outside the tolerance: exit 1, written, and the map says where
    let o = run(
        &dir,
        &[
            "refcheck", &reference, &changed, "--map", "map.pgm", "--format", "json",
        ],
    );
    assert_eq!(o.status.code(), Some(1));
    let json = out(&o);
    assert!(json.contains("\"outside_px\": 64"), "{json}");
    assert!(
        json.contains("\"bbox\": {\"x0\": 6, \"y0\": 6, \"x1\": 13, \"y1\": 13}"),
        "{json}"
    );
    assert!(json.contains("\"verdict\": \"different\""));
    let map = fs::read(dir.join("map.pgm")).unwrap();
    assert!(map.starts_with(b"P5\n40 30\n255\n"));
    assert_eq!(map.iter().skip(13).filter(|&&v| v == 255).count(), 64);
    // the map is not written over a file that is there
    let o = run(
        &dir,
        &["refcheck", &reference, &changed, "--map", "map.pgm"],
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(
        err(&o).contains("map.pgm") || err(&o).contains("exists") || err(&o).contains("replace"),
        "{}",
        err(&o)
    );
    // a mask that leaves the place out: the same; one that allows enough: the same
    let blind = save(
        &dir,
        "blind.pgm",
        &patched(Gray::blank(40, 30, 0).unwrap(), 6, 6, 8, 255),
    );
    let o = run(
        &dir,
        &["refcheck", &reference, &changed, "--tolerance", &blind],
    );
    assert!(o.status.success(), "{}", err(&o));
    assert!(err(&o).contains("left out by the mask"));
    let wide = save(
        &dir,
        "wide.pgm",
        &patched(Gray::blank(40, 30, 0).unwrap(), 6, 6, 8, 254),
    );
    let o = run(
        &dir,
        &["refcheck", &reference, &changed, "--tolerance", &wide],
    );
    assert!(o.status.success(), "{}", err(&o));
    // a picture of another size is refused with its code
    let small = save(&dir, "small.pgm", &Gray::blank(20, 30, 120).unwrap());
    let o = run(&dir, &["refcheck", &reference, &small]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("size_mismatch"));
    let o = run(&dir, &["refcheck", &reference]);
    assert_eq!(o.status.code(), Some(1));
}

/// A grid of 5 by 4 round marks, 60 pixels apart, on a light board of 400 by 300.
fn mark_grid() -> Gray {
    let mut g = Gray::blank(400, 300, 220).unwrap();
    for r in 0..4 {
        for c in 0..5 {
            g.disc(80.0 + 60.0 * c as f64, 70.0 + 60.0 * r as f64, 9.0, 30);
        }
    }
    g
}

#[test]
fn the_lens_command_can_report_what_is_left_over_mark_by_mark() {
    let dir = scratch("lens");
    let pic = save(&dir, "grid.pgm", &mark_grid());
    // without the option nothing changes: the factors are printed
    let o = run(&dir, &["lens", &pic, "--columns", "5", "--rows", "4"]);
    assert!(o.status.success(), "{}", err(&o));
    assert!(out(&o).contains("lens_k1 = "));
    let o = run(
        &dir,
        &["lens", &pic, "--columns", "5", "--rows", "4", "--residuals"],
    );
    assert!(o.status.success(), "{}", err(&o));
    let text = out(&o);
    let mut lines = text.lines();
    assert!(lines.next().unwrap().starts_with("column,row,x,y,radius"));
    assert_eq!(lines.count(), 20);
    assert!(err(&o).contains("20 marks"));
    let o = run(
        &dir,
        &[
            "lens",
            &pic,
            "--columns",
            "5",
            "--rows",
            "4",
            "--residuals",
            "--format",
            "json",
            "--output",
            "r.json",
        ],
    );
    assert!(o.status.success(), "{}", err(&o));
    let json = fs::read_to_string(dir.join("r.json")).unwrap();
    assert!(json.contains("\"format\": \"eh3d-vision-lens-residuals\""));
    // the options of the report mean nothing without it
    let o = run(
        &dir,
        &[
            "lens",
            &pic,
            "--columns",
            "5",
            "--rows",
            "4",
            "--output",
            "x.json",
        ],
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("--residuals"));
    assert!(!dir.join("x.json").exists());
}

#[test]
fn a_result_of_the_convention_is_checked_from_a_file() {
    let dir = scratch("check");
    let source = Source {
        width: 640,
        height: 480,
        sha256: None,
    };
    let good = document(
        "blobs",
        &source,
        &[Finding::Blob {
            x: 10.0,
            y: 20.0,
            area_px: 77,
        }],
        "picture",
    )
    .unwrap();
    fs::write(dir.join("good.json"), &good).unwrap();
    let o = run(&dir, &["server", "check", "good.json"]);
    assert!(o.status.success(), "{}", err(&o));
    assert!(
        out(&o).contains("ok: blobs result, 1 findings, evidence picture, picture 640 x 480 px")
    );
    fs::write(dir.join("bad.json"), good.replace("\"px\"", "\"mm\"")).unwrap();
    let o = run(&dir, &["server", "check", "bad.json"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("units"), "{}", err(&o));
    for name in ["missing.json", ""] {
        let o = run(&dir, &["server", "check", name]);
        assert_eq!(o.status.code(), Some(1), "{name}");
    }
    let o = run(&dir, &["server", "check"]);
    assert_eq!(o.status.code(), Some(1));
}

#[test]
fn the_search_for_servers_and_the_results_for_the_server_need_what_they_need() {
    let dir = scratch("server");
    let pic = save(
        &dir,
        "p.pgm",
        &patched(Gray::blank(60, 40, 220).unwrap(), 10, 10, 20, 30),
    );
    // the search on the network asks nothing while the key is off, and a build without the resolver says so
    let o = run(&dir, &["server", "discover"]);
    assert_eq!(o.status.code(), Some(1));
    let e = err(&o);
    assert!(
        e.contains("server_mdns is off") || e.contains("without the search of the local network"),
        "{e}"
    );
    // sending needs a server and a machine, and says so before it reads anything that is not there
    for kind in ["fiducials", "focus", "blobs"] {
        let o = run(&dir, &["server", "send", kind, &pic]);
        assert_eq!(o.status.code(), Some(1), "{kind}");
        assert!(
            err(&o).contains("server_address is not set"),
            "{kind}: {}",
            err(&o)
        );
    }
    let o = run(&dir, &["server", "detect", "focus", &pic]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("server_address is not set"));
    // with the key on, a program built without the resolver says so (one built with it would look at the real network, which a test does not do)
    if cfg!(not(feature = "mdns")) {
        fs::write(dir.join("vision.cfg"), "server_mdns = on\n").unwrap();
        let o = run(&dir, &["server", "discover"]);
        assert_eq!(o.status.code(), Some(1));
        assert!(!err(&o).contains("server_mdns is off"), "{}", err(&o));
        assert!(
            err(&o).contains("without the search of the local network"),
            "{}",
            err(&o)
        );
    }
}

#[test]
fn the_help_lists_the_new_commands() {
    let dir = scratch("help");
    let o = run(&dir, &["help"]);
    let text = out(&o);
    for word in [
        "eh3d-vision edge ",
        "eh3d-vision histogram ",
        "eh3d-vision refcheck ",
        "discover",
        "check FILE",
        "detect fiducials|barcode|focus|blobs",
        "send fiducials|focus|blobs",
    ] {
        assert!(text.contains(word), "{word}");
    }
}
