//! The command line refuses an option written wrong and a word too many instead of ignoring them.

use std::process::Command;

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-vision"))
        .args(args)
        .output()
        .expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn an_option_written_wrong_is_an_error_that_names_it() {
    for (cmd, args, named) in [
        ("find", vec!["find", "a.png", "--powr"], "--powr"),
        (
            "align",
            vec!["align", "design.csv", "measured.csv", "--max-rmss", "1"],
            "--max-rmss",
        ),
    ] {
        let (ok, e) = run(&args);
        assert!(
            !ok && e.contains(named) && e.contains("unknown option"),
            "{cmd}: {e}"
        );
    }
}

#[test]
fn a_word_too_many_is_an_error_that_names_it() {
    for (cmd, args, named) in [
        ("find", vec!["find", "a.png", "b.png"], "b.png"),
        (
            "align",
            vec!["align", "design.csv", "measured.csv", "extra"],
            "extra",
        ),
    ] {
        let (ok, e) = run(&args);
        assert!(
            !ok && e.contains(named) && e.contains("unexpected argument"),
            "{cmd}: {e}"
        );
    }
}

#[test]
fn nothing_is_said_against_the_help_and_the_unknown_commands_still_say_so() {
    let (ok, _) = run(&["help"]);
    assert!(ok);
    let (ok, e) = run(&["nonsense", "--powr", "5"]);
    assert!(!ok && !e.is_empty(), "{e}");
}
