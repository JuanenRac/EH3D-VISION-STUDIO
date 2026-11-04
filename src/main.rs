// EH3D-VISION-STUDIO - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

mod cli_args;

use cli_args::{finish, flag, num, switch};
use eh3d_vision_studio::align::{accept, fit};
use eh3d_vision_studio::fiducial::{find, Polarity, Settings};
use eh3d_vision_studio::image::from_bytes;
use eh3d_vision_studio::scale::Scale;
use std::env;
use std::fs;
use std::process::ExitCode;

const HELP: &str = "eh3d-vision: fiducials in a camera image

  eh3d-vision find IMAGE [--light] [--min-diam PX] [--max-diam PX] [--known-mm D]   the round marks found (--known-mm gives millimetres)
  eh3d-vision align DESIGN.csv MEASURED.csv [--scale] [--isocam-args] [--max-rms 0.1] [--max-stretch 0.01]   the turn and the shift that put the board where it is

IMAGE is a PNG or a PGM file. The CSV files have one point per line: x,y in millimetres, in the same order in both files.";

fn points(path: &str) -> Result<Vec<(f64, f64)>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .enumerate()
        .map(|(i, l)| {
            let (x, y) = l
                .split_once(',')
                .ok_or_else(|| format!("{path} line {}: it must be x,y", i + 1))?;
            Ok((num(x, "x")?, num(y, "y")?))
        })
        .collect()
}

fn run(mut args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || matches!(args[0].as_str(), "help" | "--help" | "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let cmd = args.remove(0);
    match cmd.as_str() {
        "find" => {
            let mut s = Settings::default();
            if switch(&mut args, "--light") {
                s.polarity = Polarity::Light;
            }
            if let Some(v) = flag(&mut args, "--min-diam")? {
                s.min_diameter = num(&v, "--min-diam")?;
            }
            if let Some(v) = flag(&mut args, "--max-diam")? {
                s.max_diameter = num(&v, "--max-diam")?;
            }
            let known = flag(&mut args, "--known-mm")?
                .map(|v| num(&v, "--known-mm"))
                .transpose()?;
            finish(&args, 1)?;
            let path = args.first().ok_or("find needs an image")?;
            let img = from_bytes(&fs::read(path).map_err(|e| format!("{path}: {e}"))?)?;
            let found = find(&img, &s)?;
            let scale = match (known, found.first()) {
                (Some(mm), Some(f)) => Some(Scale::from_known_diameter(f.diameter, mm)?),
                _ => None,
            };
            let centre = (img.w as f64 / 2.0, img.h as f64 / 2.0);
            for (i, f) in found.iter().enumerate() {
                match scale {
                    Some(sc) => {
                        let (x, y) = sc.point(f.x, f.y, centre);
                        println!("{}: ({:.1}, {:.1}) px, diameter {:.1} px, score {:.2}  ->  ({x:.3}, {y:.3}) mm from the centre", i + 1, f.x, f.y, f.diameter, f.score);
                    }
                    None => println!(
                        "{}: ({:.1}, {:.1}) px, diameter {:.1} px, score {:.2}",
                        i + 1,
                        f.x,
                        f.y,
                        f.diameter,
                        f.score
                    ),
                }
            }
            if found.is_empty() {
                println!("no marks found");
            }
            Ok(())
        }
        "align" => {
            let allow = switch(&mut args, "--scale");
            let isocam_args = switch(&mut args, "--isocam-args");
            let max_rms = flag(&mut args, "--max-rms")?
                .map(|v| num(&v, "--max-rms"))
                .transpose()?
                .unwrap_or(0.1);
            let max_stretch = flag(&mut args, "--max-stretch")?
                .map(|v| num(&v, "--max-stretch"))
                .transpose()?
                .unwrap_or(0.01);
            if args.len() < 2 {
                return Err("align needs the design points and the measured points".into());
            }
            finish(&args, 2)?;
            let t = fit(&points(&args[0])?, &points(&args[1])?, allow)?;
            println!(
                "shift ({:+.4}, {:+.4}) mm, turn {:+.4} degrees, size {:.5}, residual {:.4} mm",
                t.dx, t.dy, t.theta_deg, t.scale, t.rms_mm
            );
            if isocam_args {
                // the marks as the options of eh3d-isocam, one pair for each row: design x, design y, measured x, measured y
                let (design, measured) = (points(&args[0])?, points(&args[1])?);
                for (d, m) in design.iter().zip(&measured) {
                    println!("--fiducial {},{},{},{}", d.0, d.1, m.0, m.1);
                }
            }
            accept(&t, max_rms, max_stretch)
        }
        other => Err(format!("{other}: unknown command (see help)")),
    }
}

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
