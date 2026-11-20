//! The commands `correlate`, `rectify`, `pads`, `diff` and `server`, run as a program on pictures that the tests draw: what they print, what they write, their exit codes and what they refuse.

use eh3d_vision_studio::server::SimServer;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "eh3d_vision_commands_{}_{name}",
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
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    name.to_string()
}

/// A picture of 90 by 70 with one dark disc whose edge is grey where it cuts a pixel.
fn soft_disc(cx: f64, cy: f64, r: f64) -> Vec<u8> {
    let (w, h) = (90usize, 70usize);
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

/// A picture of 120 by 60 with dark pads of 10 by 22 pixels in a row, the centres `pitch` pixels apart; `moved` shifts one of them along the row.
fn pads_picture(count: usize, pitch: usize, moved: Option<(usize, i64)>) -> Vec<u8> {
    let (w, h) = (120usize, 60usize);
    let mut data = vec![220u8; w * h];
    for k in 0..count {
        let shift = match moved {
            Some((which, by)) if which == k => by,
            _ => 0,
        };
        let x0 = (8 + k * pitch) as i64 + shift;
        for y in 19..41 {
            for x in x0..x0 + 10 {
                data[y * w + x as usize] = 30;
            }
        }
    }
    pgm(w, h, &data)
}

#[test]
fn correlate_prints_the_place_of_the_mark_to_a_fraction_of_a_pixel() {
    let dir = scratch("correlate");
    let pic = write(&dir, "disc.pgm", &soft_disc(41.3, 33.7, 8.0));
    let o = run(&dir, &["correlate", &pic, "--diameter", "16"]);
    assert!(o.status.success(), "{}", err(&o));
    let text = out(&o);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "x_px,y_px,score");
    assert_eq!(lines.len(), 2, "{text}");
    let f: Vec<f64> = lines[1].split(',').map(|v| v.parse().unwrap()).collect();
    assert!(
        (f[0] - 41.3).abs() < 0.3 && (f[1] - 33.7).abs() < 0.3,
        "{text}"
    );
    assert!(f[2] > 0.9);
    assert!(err(&o).contains("1 marks"));
}

#[test]
fn correlate_takes_the_diameter_and_the_score_from_the_file_when_the_options_are_missing() {
    let dir = scratch("correlate_cfg");
    let pic = write(&dir, "disc.pgm", &soft_disc(40.0, 30.0, 8.0));
    fs::write(
        dir.join("vision.cfg"),
        "correlate_diameter = 16\ncorrelate_min_score = 0.9\n",
    )
    .unwrap();
    let o = run(&dir, &["correlate", &pic]);
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(out(&o).lines().count(), 2);
    // the option wins over the file: a template of 6 pixels does not fit a disc of 16
    let o = run(&dir, &["correlate", &pic, "--diameter", "6"]);
    assert!(o.status.success());
    assert_eq!(out(&o).lines().count(), 1);
}

#[test]
fn correlate_refuses_what_makes_no_sense() {
    let dir = scratch("correlate_bad");
    let pic = write(&dir, "disc.pgm", &soft_disc(40.0, 30.0, 8.0));
    let tiny = write(&dir, "tiny.pgm", &pgm(8, 8, &[100; 64]));
    for args in [
        vec!["correlate", &pic, "--diameter", "2"],
        vec!["correlate", &pic, "--min-score", "0"],
        vec!["correlate", &pic, "--min-score", "high"],
        vec!["correlate", &pic, "--extra"],
        vec!["correlate", &tiny],
        vec!["correlate"],
        vec!["correlate", "missing.pgm"],
    ] {
        let o = run(&dir, &args);
        assert_eq!(o.status.code(), Some(1), "{args:?}");
        assert!(err(&o).starts_with("error:"), "{args:?}: {}", err(&o));
    }
}

#[test]
fn rectify_cuts_out_a_straight_rectangle() {
    let dir = scratch("rectify");
    let pic = write(&dir, "disc.pgm", &soft_disc(41.0, 33.0, 8.0));
    let o = run(
        &dir,
        &[
            "rectify",
            &pic,
            "--corners",
            "20,10,69,10,69,59,20,59",
            "--output",
            "crop.pgm",
        ],
    );
    assert!(o.status.success(), "{}", err(&o));
    assert!(err(&o).contains("50 by 50 pixels"));
    let bytes = fs::read(dir.join("crop.pgm")).unwrap();
    assert!(bytes.starts_with(b"P5\n50 50\n255\n"));
    let original = fs::read(dir.join("disc.pgm")).unwrap();
    // the picture of the crop is the same pixels, row by row
    let (head, crop) = bytes.split_at(b"P5\n50 50\n255\n".len());
    assert_eq!(head, b"P5\n50 50\n255\n");
    let src = &original[b"P5\n90 70\n255\n".len()..];
    for y in 0..50 {
        for x in 0..50 {
            assert_eq!(crop[y * 50 + x], src[(y + 10) * 90 + x + 20], "({x}, {y})");
        }
    }
}

#[test]
fn rectify_never_writes_over_a_file_without_replace_and_takes_a_size() {
    let dir = scratch("rectify_replace");
    let pic = write(&dir, "disc.pgm", &soft_disc(41.0, 33.0, 8.0));
    let corners = "20,10,69,10,69,59,20,59";
    let first = run(
        &dir,
        &["rectify", &pic, "--corners", corners, "--output", "a.pgm"],
    );
    assert!(first.status.success());
    let again = run(
        &dir,
        &[
            "rectify",
            &pic,
            "--corners",
            corners,
            "--output",
            "a.pgm",
            "--size",
            "30x20",
        ],
    );
    assert_eq!(again.status.code(), Some(1));
    assert!(err(&again).contains("file_exists"), "{}", err(&again));
    assert!(fs::read(dir.join("a.pgm"))
        .unwrap()
        .starts_with(b"P5\n50 50\n"));
    let replaced = run(
        &dir,
        &[
            "rectify",
            &pic,
            "--corners",
            corners,
            "--output",
            "a.pgm",
            "--size",
            "30x20",
            "--replace",
        ],
    );
    assert!(replaced.status.success(), "{}", err(&replaced));
    assert!(fs::read(dir.join("a.pgm"))
        .unwrap()
        .starts_with(b"P5\n30 20\n"));
}

#[test]
fn rectify_refuses_bad_corners_sizes_and_missing_options() {
    let dir = scratch("rectify_bad");
    let pic = write(&dir, "disc.pgm", &soft_disc(41.0, 33.0, 8.0));
    let good = "20,10,69,10,69,59,20,59";
    for args in [
        vec!["rectify", &pic, "--output", "x.pgm"],
        vec!["rectify", &pic, "--corners", good],
        vec!["rectify", &pic, "--corners", "1,2,3", "--output", "x.pgm"],
        vec![
            "rectify",
            &pic,
            "--corners",
            "20,10,69,59,69,10,20,59",
            "--output",
            "x.pgm",
        ],
        vec![
            "rectify",
            &pic,
            "--corners",
            good,
            "--output",
            "x.pgm",
            "--size",
            "30",
        ],
        vec![
            "rectify",
            &pic,
            "--corners",
            good,
            "--output",
            "x.pgm",
            "--size",
            "1x1",
        ],
        vec![
            "rectify",
            &pic,
            "--corners",
            good,
            "--output",
            "x.pgm",
            "--size",
            "9999x9999",
        ],
        vec![
            "rectify",
            &pic,
            "--corners",
            good,
            "--output",
            "x.pgm",
            "--fill",
            "300",
        ],
    ] {
        let o = run(&dir, &args);
        assert_eq!(o.status.code(), Some(1), "{args:?}");
        assert!(!dir.join("x.pgm").exists(), "{args:?} wrote a file");
    }
}

#[test]
fn pads_gives_the_pitch_of_a_good_row() {
    let dir = scratch("pads");
    let pic = write(&dir, "row.pgm", &pads_picture(7, 15, None));
    let o = run(&dir, &["pads", &pic, "--mm-per-px", "0.04"]);
    assert!(o.status.success(), "{}", err(&o));
    let text = out(&o);
    let line = text.lines().nth(1).unwrap();
    assert!(line.starts_with("7,0.6000,"), "{text}");
    assert!(line.ends_with(",pass,"), "{text}");
    let json = run(
        &dir,
        &["pads", &pic, "--mm-per-px", "0.04", "--format", "json"],
    );
    assert!(out(&json).contains("\"format\": \"eh3d-vision-pads\""));
    // the pitch that is asked for
    let asked = run(
        &dir,
        &["pads", &pic, "--mm-per-px", "0.04", "--expect-pitch", "0.6"],
    );
    assert!(asked.status.success());
    let wrong = run(
        &dir,
        &["pads", &pic, "--mm-per-px", "0.04", "--expect-pitch", "0.5"],
    );
    assert_eq!(wrong.status.code(), Some(1));
    assert!(out(&wrong).contains("fail,expected"), "{}", out(&wrong));
}

#[test]
fn pads_finds_a_pad_that_is_off_and_still_writes_the_file() {
    let dir = scratch("pads_bad");
    let pic = write(&dir, "row.pgm", &pads_picture(7, 15, Some((3, 4))));
    let o = run(
        &dir,
        &["pads", &pic, "--mm-per-px", "0.04", "--output", "r.csv"],
    );
    assert_eq!(o.status.code(), Some(1));
    let text = fs::read_to_string(dir.join("r.csv")).unwrap();
    assert!(text.contains("fail,pitch"), "{text}");
    assert!(err(&o).contains("refused (pitch)"), "{}", err(&o));
    // a wide tolerance lets it pass
    let loose = run(
        &dir,
        &[
            "pads",
            &pic,
            "--mm-per-px",
            "0.04",
            "--tolerance",
            "0.6",
            "--output",
            "r2.csv",
        ],
    );
    assert!(loose.status.success(), "{}", err(&loose));
}

#[test]
fn pads_needs_the_scale_three_pads_and_a_picture() {
    let dir = scratch("pads_few");
    let two = write(&dir, "two.pgm", &pads_picture(2, 15, None));
    let row = write(&dir, "row.pgm", &pads_picture(5, 15, None));
    for args in [
        vec!["pads", &row],
        vec!["pads", &two, "--mm-per-px", "0.04"],
        vec!["pads", "--mm-per-px", "0.04"],
        vec!["pads", &row, "--mm-per-px", "0.04", "--tolerance", "0"],
        vec!["pads", &row, "--mm-per-px", "0.04", "--format", "xml"],
    ] {
        let o = run(&dir, &args);
        assert_eq!(o.status.code(), Some(1), "{args:?}");
    }
    let o = run(&dir, &["pads", &two, "--mm-per-px", "0.04"]);
    assert!(err(&o).contains("few_pads"));
    // the scale can come from the file
    fs::write(dir.join("vision.cfg"), "mm_per_px = 0.04\n").unwrap();
    assert!(run(&dir, &["pads", &row]).status.success());
}

#[test]
fn diff_says_that_two_pictures_are_the_same_or_where_they_differ() {
    let dir = scratch("diff");
    let before = write(&dir, "before.pgm", &pgm(40, 30, &[200; 1200]));
    let mut changed = vec![200u8; 1200];
    for y in 8..14 {
        for x in 10..16 {
            changed[y * 40 + x] = 40;
        }
    }
    let after = write(&dir, "after.pgm", &pgm(40, 30, &changed));
    let o = run(&dir, &["diff", &before, &before]);
    assert!(o.status.success(), "{}", err(&o));
    assert!(out(&o).lines().nth(1).unwrap().ends_with(",0,,,,,same"));
    let o = run(&dir, &["diff", &before, &after, "--mask", "mask.pgm"]);
    assert_eq!(o.status.code(), Some(1));
    let text = out(&o);
    assert!(
        text.lines()
            .nth(1)
            .unwrap()
            .starts_with("36,1200,0.030000,"),
        "{text}"
    );
    assert!(text.trim_end().ends_with(",1,10,8,15,13,changed"), "{text}");
    assert!(err(&o).contains("36 of 1200 pixels changed"));
    let mask = fs::read(dir.join("mask.pgm")).unwrap();
    let body = &mask[b"P5\n40 30\n255\n".len()..];
    assert_eq!(body.iter().filter(|v| **v == 255).count(), 36);
    // a wide share allows it, and --format json says the same
    let o = run(
        &dir,
        &[
            "diff",
            &before,
            &after,
            "--max-changed",
            "0.05",
            "--format",
            "json",
        ],
    );
    assert!(o.status.success(), "{}", err(&o));
    assert!(out(&o).contains("\"verdict\": \"same\""));
}

#[test]
fn diff_refuses_pictures_of_different_sizes_and_bad_options() {
    let dir = scratch("diff_bad");
    let a = write(&dir, "a.pgm", &pgm(40, 30, &[200; 1200]));
    let b = write(&dir, "b.pgm", &pgm(30, 40, &[200; 1200]));
    let o = run(&dir, &["diff", &a, &b]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("size_mismatch"));
    for args in [
        vec!["diff", &a],
        vec!["diff", &a, &a, &a],
        vec!["diff", &a, &a, "--threshold", "255"],
        vec!["diff", &a, &a, "--threshold", "-1"],
        vec!["diff", &a, &a, "--min-area", "0"],
        vec!["diff", &a, &a, "--max-changed", "2"],
        vec!["diff", &a, &a, "--format", "xml"],
        vec!["diff", &a, "missing.pgm"],
    ] {
        let o = run(&dir, &args);
        assert_eq!(o.status.code(), Some(1), "{args:?}");
    }
    // the mask is never written over a file without --replace
    assert!(run(&dir, &["diff", &a, &a, "--mask", "m.pgm"])
        .status
        .success());
    let again = run(&dir, &["diff", &a, &a, "--mask", "m.pgm"]);
    assert!(err(&again).contains("file_exists"), "{}", err(&again));
    assert!(run(&dir, &["diff", &a, &a, "--mask", "m.pgm", "--replace"])
        .status
        .success());
}

/// A server on the loopback that answers `connections` requests with the server in memory and then stops; it gives its address.
fn serve(sim: SimServer, connections: usize) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let handle = std::thread::spawn(move || {
        for _ in 0..connections {
            let (mut s, _) = listener.accept().unwrap();
            s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = s.read(&mut buf).unwrap_or(0);
                got.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&got).to_string();
                if let Some(at) = text.find("\r\n\r\n") {
                    let want = text[..at]
                        .lines()
                        .find_map(|l| {
                            l.strip_prefix("Content-Length: ")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if got.len() >= at + 4 + want || n == 0 {
                        break;
                    }
                } else if n == 0 {
                    break;
                }
            }
            let text = String::from_utf8_lossy(&got).to_string();
            let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
            let first = head.lines().next().unwrap_or("");
            let mut parts = first.split_whitespace();
            let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
            let token = head
                .lines()
                .find_map(|l| l.strip_prefix("Authorization: Bearer "))
                .unwrap_or("")
                .trim()
                .to_string();
            let reply = sim
                .handle(
                    method,
                    path,
                    &token,
                    if body.is_empty() { None } else { Some(body) },
                )
                .unwrap();
            let answer = format!(
                "HTTP/1.1 {} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                reply.status,
                reply.body.len(),
                reply.body
            );
            s.write_all(answer.as_bytes()).unwrap();
        }
    });
    (address, handle)
}

fn with_server(dir: &Path, address: &str, token: &str) {
    fs::write(
        dir.join("vision.cfg"),
        format!("server_address = {address}\nserver_token = {token}\nserver_machine = bench\nserver_retries = 0\n"),
    )
    .unwrap();
}

const REMOTE: &str = r#"{"polarity": "light", "quality_check": true, "pitch_mm": 0.5, "server_address": "10.1.1.1:9", "mystery": 1}"#;

#[test]
fn server_health_asks_the_server_and_prints_ok() {
    let dir = scratch("server_health");
    let sim = SimServer::new(REMOTE);
    let (address, handle) = serve(sim.clone(), 1);
    with_server(&dir, &address, "");
    let o = run(&dir, &["server", "health"]);
    handle.join().unwrap();
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(out(&o), "ok\n");
    assert_eq!(sim.log()[0].1, "/api/v1/health");
}

#[test]
fn server_config_writes_the_values_that_the_server_holds_as_lines_for_the_file() {
    let dir = scratch("server_config");
    let sim = SimServer::new(REMOTE);
    // the list of the artifacts and then the one that holds the configuration: two requests for each run
    let (address, handle) = serve(sim, 4);
    with_server(&dir, &address, SimServer::PROGRAM_TOKEN);
    let o = run(&dir, &["server", "config"]);
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(
        out(&o),
        "polarity = light\nquality_check = true\npitch_mm = 0.5\n"
    );
    assert!(
        err(&o).contains("3 values from the server, 2 left out (server_address, mystery)"),
        "{}",
        err(&o)
    );
    let o = run(&dir, &["server", "config", "--output", "from_server.cfg"]);
    handle.join().unwrap();
    assert!(o.status.success(), "{}", err(&o));
    let text = fs::read_to_string(dir.join("from_server.cfg")).unwrap();
    // the file that comes out is a configuration file the program reads
    fs::write(dir.join("vision.cfg"), &text).unwrap();
    let cfg = eh3d_vision_studio::config::Config::parse(&text).unwrap();
    assert!(cfg.quality_check && cfg.pitch_mm == 0.5);
}

#[test]
fn server_send_sends_the_result_of_the_command_and_the_verdict_is_the_exit_code() {
    let dir = scratch("server_send");
    let sim = SimServer::new(REMOTE);
    let (address, handle) = serve(sim.clone(), 4);
    with_server(&dir, &address, SimServer::PROGRAM_TOKEN);
    let good = write(&dir, "good.pgm", &soft_disc(40.0, 30.0, 8.0));
    let flat = write(&dir, "flat.pgm", &pgm(30, 30, &[120; 900]));
    let row = write(&dir, "row.pgm", &pads_picture(6, 15, None));
    let a = write(&dir, "a.pgm", &pgm(40, 30, &[200; 1200]));
    let o = run(&dir, &["server", "send", "quality", &good]);
    assert!(o.status.success(), "{}", err(&o));
    assert!(err(&o).contains("the quality result was sent as a-2"));
    // a picture that is refused is still sent, and the exit code is 1
    let o = run(&dir, &["server", "send", "quality", &flat]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("sent as a-3"));
    let o = run(
        &dir,
        &["server", "send", "pads", &row, "--mm-per-px", "0.04"],
    );
    assert!(o.status.success(), "{}", err(&o));
    let o = run(&dir, &["server", "send", "diff", &a, &a]);
    handle.join().unwrap();
    assert!(o.status.success(), "{}", err(&o));
    let kept = sim.stored();
    assert_eq!(kept.len(), 4);
    assert!(
        kept[0].data.contains("\"verdict\": \"pass\"") && kept[0].name == "vision-quality.json"
    );
    assert!(kept[1].data.contains("\"verdict\": \"fail\"") && kept[1].data.contains("flat"));
    assert!(kept[2].data.contains("eh3d-vision-pads") && kept[2].kind == "pads");
    assert!(kept[3].data.contains("eh3d-vision-diff") && kept[3].kind == "diff");
    assert!(kept.iter().all(|r| r.machine == "bench"
        && r.format == "application/json"
        && r.tool == "eh3d-vision"));
    // the token never goes in what is kept
    assert!(kept
        .iter()
        .all(|r| !r.data.contains(SimServer::PROGRAM_TOKEN)));
}

#[test]
fn server_commands_refuse_what_cannot_work_without_asking_the_network() {
    let dir = scratch("server_bad");
    let pic = write(&dir, "good.pgm", &soft_disc(40.0, 30.0, 8.0));
    // no server in the file
    let o = run(&dir, &["server", "health"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("server_address is not set"), "{}", err(&o));
    // bad values of the file or of the options
    let o = run(&dir, &["server", "health", "--address", "nohost"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        err(&o).contains("server_address must be host:port"),
        "{}",
        err(&o)
    );
    let o = run(
        &dir,
        &[
            "server",
            "send",
            "quality",
            &pic,
            "--address",
            "127.0.0.1:9",
            "--machine",
            "a/b",
        ],
    );
    assert!(err(&o).contains("server_machine"), "{}", err(&o));
    for args in [
        vec!["server", "--address", "127.0.0.1:9"],
        vec!["server", "--address", "127.0.0.1:9", "teleport"],
        vec!["server", "--address", "127.0.0.1:9", "send"],
        vec![
            "server",
            "--address",
            "127.0.0.1:9",
            "send",
            "weather",
            &pic,
        ],
        vec!["server", "--address", "127.0.0.1:9", "health", "extra"],
    ] {
        let o = run(&dir, &args);
        assert_eq!(o.status.code(), Some(1), "{args:?}");
    }
    // a machine is needed for the routes of the machine
    let o = run(&dir, &["server", "config", "--address", "127.0.0.1:9"]);
    assert!(err(&o).contains("server_machine is not set"), "{}", err(&o));
}

#[test]
fn a_server_that_is_not_there_is_unreachable_after_the_tries_and_a_wrong_token_is_unauthorized() {
    let dir = scratch("server_down");
    // nothing listens on this port any more
    let free = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .to_string();
    with_server(&dir, &free, "t");
    let o = run(&dir, &["server", "config"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("server_unreachable"), "{}", err(&o));
    let sim = SimServer::new(REMOTE);
    let (address, handle) = serve(sim, 1);
    with_server(&dir, &address, "wrong-token");
    let o = run(&dir, &["server", "config"]);
    handle.join().unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("unauthorized"), "{}", err(&o));
    // the token is not in anything the program says
    assert!(!err(&o).contains("wrong-token") && !out(&o).contains("wrong-token"));
}

#[test]
fn the_help_lists_the_new_commands() {
    let dir = scratch("help");
    let o = run(&dir, &["help"]);
    let text = out(&o);
    for word in [
        "eh3d-vision correlate",
        "eh3d-vision rectify",
        "eh3d-vision pads",
        "eh3d-vision diff",
        "eh3d-vision server",
    ] {
        assert!(text.contains(word), "{word}");
    }
}
