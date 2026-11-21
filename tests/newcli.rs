//! The commands `train`, `locate`, `pair`, `light-check`, `tape-locate`, `calib-card` and `jpeg`, run as a program on pictures that the tests draw: what they print as text and as JSON, what they
//! write (and that they never write over a file that exists), their exit codes (0 done, 1 could not be carried out, 2 the check was not passed) and that every example of the help runs.

use eh3d_vision_studio::image::Gray;
use eh3d_vision_studio::sim::to_pgm;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("eh3d_vision_newcli_{}_{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

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

fn code(o: &Output) -> i32 {
    o.status.code().unwrap_or(-1)
}

fn put(dir: &Path, name: &str, bytes: &[u8]) {
    fs::write(dir.join(name), bytes).unwrap();
}

/// A mark of 16 by 16 pixels that has no symmetry: an L with a dot.
fn patch() -> Vec<u8> {
    let mut p = vec![200u8; 256];
    for y in 0..16usize {
        for x in 0..16usize {
            let bar = (3..7).contains(&x) && (1..15).contains(&y);
            let foot = (11..15).contains(&y) && (3..14).contains(&x);
            let dot = (10..13).contains(&x) && (2..5).contains(&y);
            if bar || foot || dot {
                p[y * 16 + x] = 30;
            }
        }
    }
    p
}

fn scene(w: usize, h: usize, at: &[(usize, usize)], turned: bool) -> Gray {
    let mut g = Gray::blank(w, h, 200).unwrap();
    let mut p = patch();
    if turned {
        let src = p.clone();
        for y in 0..16 {
            for x in 0..16 {
                p[y * 16 + x] = src[(15 - x) * 16 + y];
            }
        }
    }
    for &(ox, oy) in at {
        for y in 0..16 {
            for x in 0..16 {
                g.data[(oy + y) * w + ox + x] = p[y * 16 + x];
            }
        }
    }
    g
}

fn pgm(g: &Gray) -> Vec<u8> {
    to_pgm(g)
}

/// A baseline JPEG of a flat picture of 8 by 8 pixels at the grey level `v` (the coefficients are only the first).
fn flat_jpeg(v: u8) -> Vec<u8> {
    let mut out = vec![0xFF, 0xD8];
    let seg = |out: &mut Vec<u8>, m: u8, body: &[u8]| {
        out.extend_from_slice(&[0xFF, m]);
        out.extend_from_slice(&((body.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(body);
    };
    seg(&mut out, 0xDB, &[&[0u8][..], &[1u8; 64][..]].concat());
    let mut dc = vec![0x00u8];
    let mut counts = [0u8; 16];
    counts[3] = 12;
    dc.extend_from_slice(&counts);
    dc.extend(0..12u8);
    seg(&mut out, 0xC4, &dc);
    let mut syms = vec![0x00u8, 0xF0];
    for r in 0..16u8 {
        for s in 1..=10u8 {
            syms.push(r << 4 | s);
        }
    }
    let mut ac = vec![0x10u8];
    let mut counts = [0u8; 16];
    counts[7] = 162;
    ac.extend_from_slice(&counts);
    ac.extend_from_slice(&syms);
    seg(&mut out, 0xC4, &ac);
    seg(&mut out, 0xC0, &[8, 0, 8, 0, 8, 1, 1, 0x11, 0]);
    seg(&mut out, 0xDA, &[1, 1, 0x00, 0, 63, 0]);
    let dc_value = (i32::from(v) - 128) * 8;
    let cat = if dc_value == 0 {
        0
    } else {
        32 - dc_value.unsigned_abs().leading_zeros()
    };
    let mut bits: Vec<u8> = Vec::new();
    let mut push = |value: u32, count: u32| {
        for i in (0..count).rev() {
            bits.push(((value >> i) & 1) as u8);
        }
    };
    push(cat, 4);
    if cat > 0 {
        let raw = if dc_value > 0 { dc_value } else { dc_value - 1 };
        push((raw as u32) & ((1 << cat) - 1), cat);
    }
    push(0, 8);
    while (bits.len() & 7) != 0 {
        bits.push(1);
    }
    for byte in bits.chunks(8) {
        let b = byte.iter().fold(0u8, |a, &x| a << 1 | x);
        out.push(b);
        if b == 0xFF {
            out.push(0);
        }
    }
    out.extend_from_slice(&[0xFF, 0xD9]);
    out
}

fn tape_picture(pitch: f64) -> Gray {
    let mut g = Gray::blank(480, 200, 190).unwrap();
    for k in 0..6 {
        g.disc(30.0 + f64::from(k) * pitch / 0.05, 60.0, 15.0, 20);
    }
    g
}

const DESIGN: &str = "0,0\n40,0\n40,20\n10,25\n";

fn found_of(turn_deg: f64, shift: (f64, f64), order: &[usize], extra: &[(f64, f64)]) -> String {
    let design = [(0.0, 0.0), (40.0, 0.0), (40.0, 20.0), (10.0, 25.0)];
    let (s, c) = turn_deg.to_radians().sin_cos();
    let mut text = String::new();
    for &i in order {
        let (x, y) = design[i];
        text.push_str(&format!(
            "{:.4},{:.4}\n",
            c * x - s * y + shift.0,
            s * x + c * y + shift.1
        ));
    }
    for (x, y) in extra {
        text.push_str(&format!("{x},{y}\n"));
    }
    text
}

#[test]
fn train_writes_the_mark_and_never_writes_over_a_file_that_is_there() {
    let dir = scratch("train");
    put(
        &dir,
        "scene.pgm",
        &pgm(&scene(120, 90, &[(20, 20), (80, 50)], false)),
    );
    let args = [
        "train",
        "scene.pgm",
        "--rect",
        "20,20,16,16",
        "--output",
        "mark.eh3dmark",
    ];
    let o = run(&dir, &args);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(
        out(&o).contains("mark.eh3dmark") && out(&o).contains("16 by 16"),
        "{}",
        out(&o)
    );
    let bytes = fs::read(dir.join("mark.eh3dmark")).unwrap();
    assert!(bytes.starts_with(b"EH3DMARK 1\n16 16 1\n"));
    // the second time the file is there
    let again = run(&dir, &args);
    assert_eq!(code(&again), 1);
    assert!(err(&again).contains("file_exists"), "{}", err(&again));
    assert_eq!(fs::read(dir.join("mark.eh3dmark")).unwrap(), bytes);
    // it is written over only when it is asked for
    let mut replace = args.to_vec();
    replace.push("--replace");
    assert_eq!(code(&run(&dir, &replace)), 0);
    let mut overwrite = args.to_vec();
    overwrite.push("--overwrite");
    assert_eq!(code(&run(&dir, &overwrite)), 0);
    let mut both = args.to_vec();
    both.extend(["--create", "--replace"]);
    assert_eq!(code(&run(&dir, &both)), 1);
    // --create asks for a new file: fine for a name that is free, refused for one that is there
    let mut create = args.to_vec();
    create.push("--create");
    assert_eq!(code(&run(&dir, &create)), 1);
    let mut free = [
        "train",
        "scene.pgm",
        "--rect",
        "20,20,16,16",
        "--output",
        "other.eh3dmark",
        "--create",
    ]
    .to_vec();
    assert_eq!(code(&run(&dir, &free)), 0);
    free.push("--json");
    free[5] = "third.eh3dmark";
    let j = run(&dir, &free);
    assert_eq!(code(&j), 0, "{}", err(&j));
    assert_eq!(
        out(&j).trim(),
        "{\"command\":\"train\",\"ok\":true,\"width\":16,\"height\":16,\"samples\":1,\"output\":\"third.eh3dmark\"}"
    );
}

#[test]
fn a_training_that_is_not_good_ends_with_2_and_a_command_that_cannot_go_on_with_1() {
    let dir = scratch("train2");
    put(
        &dir,
        "scene.pgm",
        &pgm(&scene(120, 90, &[(20, 20), (80, 50)], false)),
    );
    // a flat place has no contrast
    let flat = run(
        &dir,
        &[
            "train",
            "scene.pgm",
            "--rect",
            "50,5,16,16",
            "--output",
            "a.mark",
        ],
    );
    assert_eq!(code(&flat), 2);
    assert!(
        err(&flat).contains("Check not passed") && err(&flat).contains("sample_flat"),
        "{}",
        err(&flat)
    );
    assert!(!dir.join("a.mark").exists());
    let json = run(
        &dir,
        &[
            "train",
            "scene.pgm",
            "--rect",
            "50,5,16,16",
            "--output",
            "a.mark",
            "--json",
        ],
    );
    assert_eq!(code(&json), 2);
    assert!(
        out(&json).starts_with("{\"command\":\"train\",\"ok\":false,\"code\":\"sample_flat\""),
        "{}",
        out(&json)
    );
    // two examples that are not the same mark
    let apart = run(
        &dir,
        &[
            "train",
            "scene.pgm",
            "--rect",
            "20,20,16,16",
            "--rect",
            "10,60,16,16",
            "--output",
            "b.mark",
        ],
    );
    assert!(code(&apart) == 2 || code(&apart) == 1, "{}", err(&apart));
    // stripes look like themselves a few pixels away
    let mut stripes = Gray::blank(64, 64, 200).unwrap();
    for y in 0..64 {
        for x in 0..64 {
            if (x / 3) % 2 == 0 {
                stripes.data[y * 64 + x] = 40;
            }
        }
    }
    put(&dir, "stripes.pgm", &pgm(&stripes));
    let amb = run(
        &dir,
        &[
            "train",
            "stripes.pgm",
            "--rect",
            "10,10,24,24",
            "--output",
            "c.mark",
        ],
    );
    assert_eq!(code(&amb), 2, "{}", err(&amb));
    assert!(err(&amb).contains("mark_ambiguous"));
    // what is wrong with the command itself is 1
    for args in [
        vec!["train", "scene.pgm", "--output", "d.mark"],
        vec![
            "train",
            "scene.pgm",
            "--rect",
            "1,2,3",
            "--output",
            "d.mark",
        ],
        vec!["train", "scene.pgm", "--rect", "20,20,16,16"],
        vec![
            "train",
            "scene.pgm",
            "--rect",
            "110,80,16,16",
            "--output",
            "d.mark",
        ],
        vec![
            "train",
            "scene.pgm",
            "--rect",
            "20,20,16,16",
            "--output",
            "d.mark",
            "--oops",
        ],
        vec![
            "train",
            "nothing.pgm",
            "--rect",
            "20,20,16,16",
            "--output",
            "d.mark",
        ],
        vec![
            "train",
            "scene.pgm",
            "--rect",
            "20,20,2,2",
            "--output",
            "d.mark",
        ],
    ] {
        let o = run(&dir, &args);
        assert_eq!(code(&o), 1, "{args:?}: {}", err(&o));
        assert!(err(&o).starts_with("error: "), "{args:?}");
        assert!(!dir.join("d.mark").exists());
    }
}

#[test]
fn locate_finds_the_mark_where_it_is_and_says_in_the_language_what_it_found() {
    let dir = scratch("locate");
    put(
        &dir,
        "scene.pgm",
        &pgm(&scene(120, 90, &[(20, 20), (80, 50)], false)),
    );
    assert_eq!(
        code(&run(
            &dir,
            &[
                "train",
                "scene.pgm",
                "--rect",
                "20,20,16,16",
                "--output",
                "m.mark"
            ]
        )),
        0
    );
    let o = run(&dir, &["locate", "--mark", "m.mark", "scene.pgm"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    let text = out(&o);
    assert!(
        text.starts_with("2 place(s) found for the mark\n"),
        "{text}"
    );
    assert!(
        text.contains("1: (27.500, 27.500) px") || text.contains("1: (27.500, 27.500)"),
        "{text}"
    );
    let es = out(&run(
        &dir,
        &["locate", "--mark", "m.mark", "scene.pgm", "--lang", "es"],
    ));
    assert!(es.starts_with("2 lugar(es) encontrado(s)"), "{es}");
    let j = out(&run(
        &dir,
        &["locate", "--mark", "m.mark", "scene.pgm", "--json"],
    ));
    assert!(
        j.starts_with("{\"command\":\"locate\",\"ok\":true,\"count\":2,\"found\":[{\"x\":"),
        "{j}"
    );
    let alt = out(&run(
        &dir,
        &[
            "locate",
            "--mark",
            "m.mark",
            "scene.pgm",
            "--format",
            "json",
        ],
    ));
    assert_eq!(alt, j);
}

#[test]
fn locate_searches_with_a_turn_and_ends_with_2_when_nothing_is_found() {
    let dir = scratch("locate2");
    put(&dir, "plain.pgm", &pgm(&scene(64, 48, &[(20, 20)], false)));
    put(&dir, "turned.pgm", &pgm(&scene(64, 48, &[(20, 20)], true)));
    put(&dir, "empty.pgm", &pgm(&Gray::blank(64, 48, 200).unwrap()));
    assert_eq!(
        code(&run(
            &dir,
            &[
                "train",
                "plain.pgm",
                "--rect",
                "20,20,16,16",
                "--output",
                "m.mark"
            ]
        )),
        0
    );
    // upright only: the mark turned a quarter is not found
    let upright = run(&dir, &["locate", "--mark", "m.mark", "turned.pgm"]);
    assert_eq!(code(&upright), 2, "{}", out(&upright));
    assert!(out(&upright).contains("The mark was not found"));
    assert!(err(&upright).contains("Check not passed"));
    // with the turn allowed it is found, at about a quarter of a turn
    let turned = run(
        &dir,
        &[
            "locate",
            "--mark",
            "m.mark",
            "turned.pgm",
            "--turn",
            "100",
            "--json",
        ],
    );
    assert_eq!(code(&turned), 0, "{}{}", out(&turned), err(&turned));
    let t = out(&turned);
    let angle: f64 = t
        .split("\"angle_deg\":")
        .nth(1)
        .unwrap()
        .split(',')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!((angle.abs() - 90.0).abs() <= 3.0, "{t}");
    // a picture without the mark: 2, with the JSON saying so
    let none = run(&dir, &["locate", "--mark", "m.mark", "empty.pgm", "--json"]);
    assert_eq!(code(&none), 2);
    assert_eq!(
        out(&none).trim(),
        "{\"command\":\"locate\",\"ok\":false,\"count\":0,\"found\":[]}"
    );
    // a file that is not a mark, a picture too small and a missing option are 1
    put(&dir, "bad.mark", b"not a mark");
    for args in [
        vec!["locate", "--mark", "bad.mark", "plain.pgm"],
        vec!["locate", "--mark", "missing.mark", "plain.pgm"],
        vec!["locate", "plain.pgm"],
        vec!["locate", "--mark", "m.mark", "plain.pgm", "--turn", "x"],
        vec!["locate", "--mark", "m.mark", "plain.pgm", "--turn", "181"],
    ] {
        let o = run(&dir, &args);
        assert_eq!(code(&o), 1, "{args:?}: {}", err(&o));
    }
    // a file of a mark that is a stripe is ambiguous: 2
    let mut stripes = b"EH3DMARK 1\n16 16 1\n".to_vec();
    stripes.extend((0..256).map(|i| if (i % 16) / 3 % 2 == 0 { 40u8 } else { 200 }));
    put(&dir, "stripes.mark", &stripes);
    let amb = run(&dir, &["locate", "--mark", "stripes.mark", "plain.pgm"]);
    assert_eq!(code(&amb), 2, "{}", err(&amb));
    assert!(err(&amb).contains("mark_ambiguous"));
}

#[test]
fn pair_with_any_turn_pairs_a_board_that_is_turned_with_a_mark_extra() {
    let dir = scratch("pair");
    put(&dir, "design.csv", DESIGN.as_bytes());
    put(
        &dir,
        "found.csv",
        found_of(137.0, (12.0, -7.0), &[2, 0, 3, 1], &[(90.0, 90.0)]).as_bytes(),
    );
    let o = run(&dir, &["pair", "design.csv", "found.csv", "--any-turn"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    let text = out(&o);
    assert!(
        text.starts_with("Paired 4 of 4 design marks; turn +137.000 degrees"),
        "{text}"
    );
    assert!(
        text.contains("design 1 -> mark 2") && text.contains("design 3 -> mark 1"),
        "{text}"
    );
    assert!(
        text.contains("Marks found with no design mark: 5"),
        "{text}"
    );
    let j = out(&run(
        &dir,
        &["pair", "design.csv", "found.csv", "--any-turn", "--json"],
    ));
    assert!(j.starts_with("{\"command\":\"pair\",\"ok\":true,\"mode\":\"any-turn\",\"paired\":[[1,2],[2,4],[3,1],[4,3]],\"missing\":[],\"extra\":[5],\"turn_deg\":13"), "{j}");
    assert!(
        j.contains("\"dx_mm\":12.000") && j.contains("\"dy_mm\":-7.000"),
        "{j}"
    );
    let de = out(&run(
        &dir,
        &[
            "pair",
            "design.csv",
            "found.csv",
            "--any-turn",
            "--lang",
            "de",
        ],
    ));
    assert!(de.starts_with("4 von 4 Entwurfsmarken zugeordnet"), "{de}");
    // a mark missing is told
    put(
        &dir,
        "less.csv",
        found_of(20.0, (0.0, 0.0), &[3, 1, 0], &[]).as_bytes(),
    );
    let m = out(&run(
        &dir,
        &["pair", "design.csv", "less.csv", "--any-turn", "--json"],
    ));
    assert!(
        m.contains("\"missing\":[3]") && m.contains("\"extra\":[]"),
        "{m}"
    );
}

#[test]
fn pair_by_place_pairs_a_board_that_is_nearly_where_it_should_be() {
    let dir = scratch("pair2");
    put(&dir, "design.csv", DESIGN.as_bytes());
    put(
        &dir,
        "found.csv",
        found_of(2.0, (1.0, 1.0), &[3, 2, 1, 0], &[]).as_bytes(),
    );
    let o = run(&dir, &["pair", "design.csv", "found.csv"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert_eq!(
        out(&o),
        "design 1 -> mark 4\ndesign 2 -> mark 3\ndesign 3 -> mark 2\ndesign 4 -> mark 1\n"
    );
    // a different number of marks cannot be paired by place
    put(
        &dir,
        "more.csv",
        found_of(2.0, (1.0, 1.0), &[0, 1, 2, 3], &[(5.0, 5.0)]).as_bytes(),
    );
    let bad = run(&dir, &["pair", "design.csv", "more.csv"]);
    assert_eq!(code(&bad), 1);
}

#[test]
fn pair_ends_with_2_when_two_ways_explain_the_marks_and_with_0_when_the_turn_is_narrowed() {
    let dir = scratch("pair3");
    // the four corners of a rectangle look the same turned by a half turn
    put(&dir, "rect.csv", b"0,0\n40,0\n40,20\n0,20\n");
    let (s, c) = 5.0f64.to_radians().sin_cos();
    let rotated: String = [(0.0, 0.0), (40.0, 0.0), (40.0, 20.0), (0.0, 20.0)]
        .iter()
        .map(|(x, y)| format!("{:.4},{:.4}\n", c * x - s * y + 3.0, s * x + c * y + 4.0))
        .collect();
    put(&dir, "rotated.csv", rotated.as_bytes());
    let amb = run(&dir, &["pair", "rect.csv", "rotated.csv", "--any-turn"]);
    assert_eq!(code(&amb), 2, "{}", out(&amb));
    assert!(err(&amb).contains("pair_ambiguous"), "{}", err(&amb));
    let json = run(
        &dir,
        &["pair", "rect.csv", "rotated.csv", "--any-turn", "--json"],
    );
    assert_eq!(code(&json), 2);
    assert!(
        out(&json).starts_with("{\"command\":\"pair\",\"ok\":false,\"code\":\"pair_ambiguous\""),
        "{}",
        out(&json)
    );
    let narrow = run(
        &dir,
        &[
            "pair",
            "rect.csv",
            "rotated.csv",
            "--any-turn",
            "--max-turn",
            "90",
        ],
    );
    assert_eq!(code(&narrow), 0, "{}", err(&narrow));
    assert!(
        out(&narrow).contains("turn +5.000 degrees"),
        "{}",
        out(&narrow)
    );
    // marks that do not make the design: 2; a point that is not a number or an option that is wrong: 1
    put(&dir, "noise.csv", b"1,1\n17,3\n8,40\n55,2\n");
    let none = run(&dir, &["pair", "rect.csv", "noise.csv", "--any-turn"]);
    assert_eq!(code(&none), 2, "{}", err(&none));
    for args in [
        vec!["pair", "rect.csv"],
        vec![
            "pair",
            "rect.csv",
            "rotated.csv",
            "--any-turn",
            "--max-turn",
            "x",
        ],
        vec![
            "pair",
            "rect.csv",
            "rotated.csv",
            "--any-turn",
            "--max-turn",
            "181",
        ],
        vec!["pair", "rect.csv", "rotated.csv", "--any-turn", "--oops"],
        vec!["pair", "rect.csv", "missing.csv", "--any-turn"],
    ] {
        assert_eq!(code(&run(&dir, &args)), 1, "{args:?}");
    }
}

#[test]
fn light_check_says_whether_the_light_is_good_and_why_not() {
    let dir = scratch("light");
    put(&dir, "good.pgm", &pgm(&Gray::blank(64, 48, 128).unwrap()));
    put(&dir, "dark.pgm", &pgm(&Gray::blank(64, 48, 8).unwrap()));
    let g = run(&dir, &["light-check", "good.pgm"]);
    assert_eq!(code(&g), 0, "{}", err(&g));
    assert!(
        out(&g).contains("Light: mean 128.0, from 128 to 128;")
            && out(&g).contains("The light is good for measuring"),
        "{}",
        out(&g)
    );
    let d = run(&dir, &["light-check", "dark.pgm"]);
    assert_eq!(code(&d), 2);
    assert!(
        out(&d).contains("light_dim: the picture is too dark"),
        "{}",
        out(&d)
    );
    assert!(
        err(&d).contains("Check not passed: light_dim"),
        "{}",
        err(&d)
    );
    let es = out(&run(&dir, &["light-check", "dark.pgm", "--lang", "es"]));
    assert!(es.contains("la imagen es demasiado oscura"), "{es}");
    let j = out(&run(&dir, &["light-check", "dark.pgm", "--json"]));
    assert!(
        j.contains("\"good\":false,\"reasons\":[\"light_dim\"]"),
        "{j}"
    );
    let csv = out(&run(&dir, &["light-check", "good.pgm", "--format", "csv"]));
    assert!(
        csv.starts_with(
            "width,height,mean,min,max,saturated,uneven,tilt_x,tilt_y,good,reasons\n64,48,128.000,"
        ),
        "{csv}"
    );
    // the limits come from the options and from vision.cfg
    let loose = run(
        &dir,
        &[
            "light-check",
            "dark.pgm",
            "--cells",
            "2",
            "--format",
            "json",
        ],
    );
    assert_eq!(code(&loose), 2);
    put(&dir, "vision.cfg", b"light_min_mean = 5\n");
    assert_eq!(code(&run(&dir, &["light-check", "dark.pgm"])), 0);
    // the output is a new file
    let w = run(&dir, &["light-check", "good.pgm", "--output", "r.txt"]);
    assert_eq!(code(&w), 0);
    assert!(fs::read_to_string(dir.join("r.txt"))
        .unwrap()
        .contains("Light: mean"));
    let again = run(&dir, &["light-check", "good.pgm", "--output", "r.txt"]);
    assert_eq!(code(&again), 1);
    assert!(err(&again).contains("file_exists"));
    for args in [
        vec!["light-check"],
        vec!["light-check", "good.pgm", "--cells", "1"],
        vec!["light-check", "good.pgm", "--format", "xml"],
        vec!["light-check", "good.pgm", "--json", "--format", "csv"],
        vec!["light-check", "nothing.pgm"],
    ] {
        assert_eq!(code(&run(&dir, &args)), 1, "{args:?}");
    }
}

#[test]
fn tape_locate_finds_the_holes_the_pitch_and_the_place_to_pick() {
    let dir = scratch("tape");
    put(&dir, "tape.pgm", &pgm(&tape_picture(4.0)));
    put(&dir, "wide.pgm", &pgm(&tape_picture(4.3)));
    let o = run(&dir, &["tape-locate", "tape.pgm", "--mm-per-px", "0.05"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    let text = out(&o);
    assert!(text.starts_with("Tape: 6 holes, pitch 4.0"), "{text}");
    assert!(
        text.contains("hole 1: (30.00, 60.00) px, 1.5")
            && text.contains("pocket 1: (70.00, 130.00) px"),
        "{text}"
    );
    assert!(
        text.contains("Pick the part at pocket") && text.contains("The tape can be trusted"),
        "{text}"
    );
    let j = out(&run(
        &dir,
        &["tape-locate", "tape.pgm", "--mm-per-px", "0.05", "--json"],
    ));
    assert!(
        j.starts_with("{\"command\":\"tape-locate\",\"mm_per_px\":0.050000,"),
        "{j}"
    );
    assert!(j.contains("\"good\":true,\"reasons\":[]"), "{j}");
    let wide = run(&dir, &["tape-locate", "wide.pgm", "--mm-per-px", "0.05"]);
    assert_eq!(code(&wide), 2);
    assert!(
        out(&wide).contains("tape_pitch: the pitch measured is not the pitch of the tape"),
        "{}",
        out(&wide)
    );
    // the pitch of the tape can be said, and the scale can come from vision.cfg
    let right = run(
        &dir,
        &[
            "tape-locate",
            "wide.pgm",
            "--mm-per-px",
            "0.05",
            "--pitch-mm",
            "4.3",
        ],
    );
    assert_eq!(code(&right), 0, "{}", out(&right));
    put(&dir, "vision.cfg", b"mm_per_px = 0.05\n");
    assert_eq!(code(&run(&dir, &["tape-locate", "tape.pgm"])), 0);
    let _ = fs::remove_file(dir.join("vision.cfg"));
    let none = run(&dir, &["tape-locate", "tape.pgm"]);
    assert_eq!(code(&none), 1);
    assert!(err(&none).contains("tape_no_scale"), "{}", err(&none));
    // a picture with only two holes does not place the tape
    let mut few = Gray::blank(480, 200, 190).unwrap();
    few.disc(30.0, 60.0, 15.0, 20);
    few.disc(110.0, 60.0, 15.0, 20);
    put(&dir, "few.pgm", &pgm(&few));
    let f = run(&dir, &["tape-locate", "few.pgm", "--mm-per-px", "0.05"]);
    assert_eq!(code(&f), 2);
    assert!(out(&f).contains("tape_few_holes"), "{}", out(&f));
}

#[test]
fn calib_card_prints_the_same_drawing_every_time_and_does_not_write_over_a_file() {
    let dir = scratch("card");
    let a = run(&dir, &["calib-card"]);
    assert_eq!(code(&a), 0, "{}", err(&a));
    assert_eq!(out(&a), out(&run(&dir, &["calib-card"])));
    assert!(out(&a).starts_with("<svg xmlns=") && out(&a).matches("<circle").count() == 63);
    let pt = out(&run(&dir, &["calib-card", "--lang", "pt"]));
    assert!(
        pt.contains("grelha") && pt.contains("Imprima a 100 %"),
        "{pt}"
    );
    let w = run(
        &dir,
        &[
            "calib-card",
            "--output",
            "card.svg",
            "--columns",
            "5",
            "--rows",
            "4",
        ],
    );
    assert_eq!(code(&w), 0, "{}", err(&w));
    assert_eq!(
        fs::read_to_string(dir.join("card.svg"))
            .unwrap()
            .matches("<circle")
            .count(),
        20
    );
    let again = run(&dir, &["calib-card", "--output", "card.svg"]);
    assert_eq!(code(&again), 1);
    assert!(err(&again).contains("file_exists"));
    assert_eq!(
        code(&run(
            &dir,
            &["calib-card", "--output", "card.svg", "--replace"]
        )),
        0
    );
    // a card that does not fit its page is refused, and so are wrong numbers
    let big = run(&dir, &["calib-card", "--columns", "40", "--pitch-mm", "10"]);
    assert_eq!(code(&big), 1);
    assert!(err(&big).contains("card_too_big"), "{}", err(&big));
    assert!(err(&run(&dir, &["calib-card", "--columns", "2"])).contains("card_bad_setting"));
    assert_eq!(code(&run(&dir, &["calib-card", "extra"])), 1);
    put(&dir, "vision.cfg", b"card_columns = 6\ncard_rows = 3\n");
    assert_eq!(
        out(&run(&dir, &["calib-card"])).matches("<circle").count(),
        18
    );
}

#[test]
fn jpeg_reads_a_frame_and_refuses_a_frame_that_is_cut_with_its_code() {
    let dir = scratch("jpeg");
    let frame = flat_jpeg(200);
    put(&dir, "frame.jpg", &frame);
    let o = run(&dir, &["jpeg", "frame.jpg", "--output", "frame.pgm"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert_eq!(
        out(&o).trim(),
        "Picture of 8 by 8 pixels read from the frame, mean grey level 200.0"
    );
    let pgm_bytes = fs::read(dir.join("frame.pgm")).unwrap();
    assert!(pgm_bytes.starts_with(b"P5\n8 8\n255\n") && pgm_bytes.len() == 11 + 64);
    assert!(pgm_bytes[11..].iter().all(|&v| v == 200));
    let j = out(&run(&dir, &["jpeg", "frame.jpg", "--json"]));
    assert_eq!(
        j.trim(),
        "{\"command\":\"jpeg\",\"ok\":true,\"width\":8,\"height\":8,\"mean\":200.000}"
    );
    assert_eq!(
        code(&run(&dir, &["jpeg", "frame.jpg", "--output", "frame.pgm"])),
        1
    );
    put(&dir, "cut.jpg", &frame[..frame.len() - 9]);
    let cut = run(&dir, &["jpeg", "cut.jpg"]);
    assert_eq!(code(&cut), 1);
    assert!(
        err(&cut).starts_with("error: jpeg_truncated"),
        "{}",
        err(&cut)
    );
    put(&dir, "text.jpg", b"hello");
    assert!(err(&run(&dir, &["jpeg", "text.jpg"])).contains("jpeg_not_jpeg"));
    put(&dir, "vision.cfg", b"mjpeg_max_bytes = 1024\n");
    let mut padded = frame.clone();
    padded.resize(2000, 0);
    put(&dir, "padded.jpg", &padded);
    assert_eq!(code(&run(&dir, &["jpeg", "frame.jpg"])), 0);
    let big = run(&dir, &["jpeg", "padded.jpg"]);
    assert_eq!(code(&big), 1);
    assert!(err(&big).contains("jpeg_file_too_big"), "{}", err(&big));
}

#[test]
fn every_example_of_the_help_runs() {
    let dir = scratch("examples");
    put(
        &dir,
        "scene.pgm",
        &pgm(&scene(120, 90, &[(20, 20), (80, 50)], false)),
    );
    put(&dir, "design.csv", DESIGN.as_bytes());
    put(
        &dir,
        "found.csv",
        found_of(20.0, (5.0, 5.0), &[2, 0, 3, 1], &[]).as_bytes(),
    );
    put(&dir, "light.pgm", &pgm(&Gray::blank(64, 48, 128).unwrap()));
    put(&dir, "tape.pgm", &pgm(&tape_picture(4.0)));
    put(&dir, "frame.jpg", &flat_jpeg(90));
    let help = out(&run(&dir, &["help"]));
    let examples: Vec<&str> = help
        .lines()
        .skip_while(|l| !l.starts_with("Examples"))
        .skip(1)
        .filter(|l| l.starts_with("  eh3d-vision "))
        .collect();
    assert!(examples.len() >= 7, "{examples:?}");
    for line in examples {
        let words: Vec<&str> = line.split_whitespace().skip(1).collect();
        let o = run(&dir, &words);
        assert_eq!(code(&o), 0, "{line}: {}{}", out(&o), err(&o));
    }
    for word in [
        "eh3d-vision train",
        "eh3d-vision locate",
        "eh3d-vision pair",
        "eh3d-vision light-check",
        "eh3d-vision tape-locate",
        "eh3d-vision calib-card",
        "eh3d-vision jpeg",
        "Exit codes: 0 done",
    ] {
        assert!(help.contains(word), "{word}");
    }
}
