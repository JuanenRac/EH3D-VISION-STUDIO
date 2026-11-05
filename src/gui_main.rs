// EH3D-VISION-STUDIO - the window, as a program of its own.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! `eh3d-vision-gui` opens the window with the files named on the command line (a picture, and a file of points as the design). It is built with `--features gui`. On Windows it is a
//! program of the graphical subsystem, so no console window opens behind it; for that reason it prints nothing, and an error that keeps the window from opening is written to
//! `eh3d-vision-gui-error.txt` in the temporary folder. The command line (`eh3d-vision`) keeps its console, so its output and its exit codes work in a terminal and in a script.
#![forbid(unsafe_code)]
#![cfg_attr(windows, windows_subsystem = "windows")]

use eh3d_vision_studio::config::Config;
use std::path::PathBuf;
use std::process::ExitCode;

/// Writes why the window could not open where a person can find it, since there is no console to print it on.
fn report_error(text: &str) {
    let _ = std::fs::write(
        std::env::temp_dir().join("eh3d-vision-gui-error.txt"),
        format!("{text}\n"),
    );
}

fn main() -> ExitCode {
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
