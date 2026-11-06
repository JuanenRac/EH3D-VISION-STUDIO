// EH3D-VISION-STUDIO - the window, as a program of its own.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! `eh3d-vision-gui` opens the window with the files named on the command line (a picture, and a file of points as the design). It is built with `--features gui`. On Windows it is a
//! program of the graphical subsystem, so no console window opens behind it; for that reason it prints nothing, and an error that keeps the window from opening is written to
//! `eh3d-vision-gui-error.txt` in the temporary folder. The command line (`eh3d-vision`) keeps its console, so its output and its exit codes work in a terminal and in a script.
#![forbid(unsafe_code)]
#![cfg_attr(windows, windows_subsystem = "windows")]

use eh3d_vision_studio::config::Config;
use eh3d_vision_studio::shot;
use std::path::PathBuf;
use std::process::ExitCode;

/// Writes why the window could not open where a person can find it, since there is no console to print it on.
fn report_error(text: &str) {
    let _ = std::fs::write(
        std::env::temp_dir().join("eh3d-vision-gui-error.txt"),
        format!("{text}\n"),
    );
}

/// The picture mode: `--shot FILE.png` opens the window with the default values, saves what it draws as a PNG file and closes. The exit code is 0 when the file was written, 1 when it
/// was not and 2 when the arguments are wrong; the reason is written on the error output, which a script that redirects it can read.
fn take_picture(args: &[String]) -> ExitCode {
    let parsed = match shot::parse_window_args(args, 8) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let Some(request) = parsed.shot else {
        eprintln!("error: --shot needs a file to write");
        return ExitCode::from(2);
    };
    let lang = parsed
        .lang
        .unwrap_or_else(eh3d_vision_studio::lang::Lang::from_env);
    match eh3d_vision_studio::gui::run_shot(parsed.files, lang, request) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: the picture could not be taken: {e}");
            ExitCode::from(1)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if shot::wants_picture(&args) {
        return take_picture(&args);
    }
    let files: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let (cfg, problem) = Config::load();
    match eh3d_vision_studio::gui::run(files, cfg, problem) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            report_error(&format!("the window could not open: {e}"));
            ExitCode::from(1)
        }
    }
}
