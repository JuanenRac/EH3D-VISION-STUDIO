//! The size limit of the files the command line reads: a file within `max_file_mib` is read, a bigger one is refused with a code and its name, and nothing is loaded.

use eh3d_vision_studio::limit::{read_limited, read_text_limited, NOT_TEXT, TOO_LARGE, UNREADABLE};
use std::fs;
use std::process::Command;

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d_vision_limit_{}_{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_file_within_the_limit_is_read_and_a_bigger_one_is_refused_with_its_code() {
    let dir = scratch("size");
    let file = dir.join("points.csv");
    fs::write(&file, vec![b'7'; 1_048_576]).unwrap();
    assert_eq!(read_limited(&file, 1).unwrap().len(), 1_048_576);
    fs::write(&file, vec![b'7'; 1_048_577]).unwrap();
    let why = read_limited(&file, 1).unwrap_err();
    assert!(
        why.starts_with("file_too_large: ") && why.contains("points.csv") && why.contains("1 MiB"),
        "{why}"
    );
    assert!(why.starts_with(TOO_LARGE));
    // a limit of nothing refuses any file that has a byte
    fs::write(&file, "x").unwrap();
    assert!(read_limited(&file, 0).unwrap_err().starts_with(TOO_LARGE));
    fs::write(&file, "").unwrap();
    assert_eq!(read_limited(&file, 0).unwrap(), Vec::<u8>::new());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_file_and_a_file_that_is_not_text_have_their_own_codes() {
    let dir = scratch("codes");
    let why = read_limited(&dir.join("nothing.png"), 1).unwrap_err();
    assert!(
        why.starts_with(UNREADABLE) && why.contains("nothing.png"),
        "{why}"
    );
    let file = dir.join("binary.csv");
    fs::write(&file, [0xff, 0xfe, 0x00]).unwrap();
    let why = read_text_limited(&file, 1).unwrap_err();
    assert!(
        why.starts_with(NOT_TEXT) && why.contains("binary.csv"),
        "{why}"
    );
    fs::write(&file, "1,2\n").unwrap();
    assert_eq!(read_text_limited(&file, 1).unwrap(), "1,2\n");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_command_line_reads_with_the_limit_of_the_configuration_file() {
    let dir = scratch("cli");
    let big = dir.join("design.csv");
    fs::write(&big, vec![b'1'; 1_048_577]).unwrap();
    let cfg = dir.join("vision.cfg");
    fs::write(&cfg, "max_file_mib = 1\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-vision"))
        .args(["align", big.to_str().unwrap(), big.to_str().unwrap()])
        .env("EH3D_VISION_CONFIG", &cfg)
        .output()
        .expect("the program starts");
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(!out.status.success());
    assert!(
        err.contains("error: file_too_large: ") && err.contains("design.csv"),
        "{err}"
    );
    let _ = fs::remove_dir_all(&dir);
}
