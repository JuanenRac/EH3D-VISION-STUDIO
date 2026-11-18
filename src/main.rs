// EH3D-VISION-STUDIO - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

mod cli_args;

use cli_args::{finish, flag, num, switch};
use eh3d_vision_studio::align::{accept, fit, isocam_args as isocam_lines, pair_nearest};
use eh3d_vision_studio::config::Config;
use eh3d_vision_studio::fiducial::{find, refine, Polarity, Settings};
use eh3d_vision_studio::image::from_bytes;
use eh3d_vision_studio::limit::{read_limited, read_text_limited};
use eh3d_vision_studio::points::parse as parse_points;
use eh3d_vision_studio::scale::Scale;
use std::env;
use std::path::Path;
use std::process::ExitCode;

const HELP: &str = "eh3d-vision: fiducials in a camera image

  eh3d-vision find IMAGE [--light] [--min-diam PX] [--max-diam PX] [--known-mm D] [--refine]   the round marks found (--known-mm gives millimetres, --refine the centre to a fraction of a pixel with a confidence)
  eh3d-vision align DESIGN.csv MEASURED.csv [--scale] [--pair-nearest] [--isocam-args] [--max-rms 0.1] [--max-stretch 0.01]   the turn and the shift that put the board where it is

IMAGE is a PNG or a PGM file. The CSV files have one point per line: x,y in millimetres, in the same order in both files.";

/// The configuration file (the default values when the file is wrong, with a warning).
fn config() -> Config {
    let (config, problem) = Config::load();
    if let Some(p) = problem {
        eprintln!("warning: {p}; the default values are used");
    }
    config
}

/// The largest file that is read, in MiB: `max_file_mib` of the configuration file.
fn max_mib() -> u64 {
    config().max_file_mib
}

fn points(path: &str) -> Result<Vec<(f64, f64)>, String> {
    let text = read_text_limited(Path::new(path), max_mib())?;
    parse_points(&text, path)
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
            let refine_on = switch(&mut args, "--refine");
            let known = flag(&mut args, "--known-mm")?
                .map(|v| num(&v, "--known-mm"))
                .transpose()?;
            finish(&args, 1)?;
            let path = args.first().ok_or("find needs an image")?;
            let img = from_bytes(&read_limited(Path::new(path), max_mib())?)?;
            let mut found = find(&img, &s)?;
            // the centre to a fraction of a pixel, and the confidence of each mark, when the option or the configuration file asks for it
            let cfg = config();
            let mut confidence: Vec<f64> = Vec::new();
            if refine_on || cfg.refine_subpixel {
                for f in &mut found {
                    let r = refine(&img, f, s.polarity, cfg.full_contrast);
                    (f.x, f.y) = (r.x, r.y);
                    confidence.push(r.confidence);
                }
            }
            let scale = match (known, found.first()) {
                (Some(mm), Some(f)) => Some(Scale::from_known_diameter(f.diameter, mm)?),
                _ => None,
            };
            let centre = (img.w as f64 / 2.0, img.h as f64 / 2.0);
            for (i, f) in found.iter().enumerate() {
                let sure = confidence
                    .get(i)
                    .map(|c| format!(", confidence {c:.2}"))
                    .unwrap_or_default();
                match scale {
                    Some(sc) => {
                        let (x, y) = sc.point(f.x, f.y, centre);
                        println!("{}: ({:.1}, {:.1}) px, diameter {:.1} px, score {:.2}{sure}  ->  ({x:.3}, {y:.3}) mm from the centre", i + 1, f.x, f.y, f.diameter, f.score);
                    }
                    None => println!(
                        "{}: ({:.1}, {:.1}) px, diameter {:.1} px, score {:.2}{sure}",
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
            let pair = switch(&mut args, "--pair-nearest");
            let max_rms = flag(&mut args, "--max-rms")?
                .map(|v| num(&v, "--max-rms"))
                .transpose()?
                .unwrap_or(0.1);
            let max_stretch = flag(&mut args, "--max-stretch")?
                .map(|v| num(&v, "--max-stretch"))
                .transpose()?
                .unwrap_or(0.01);
            if !(max_rms.is_finite()
                && max_rms >= 0.0
                && max_stretch.is_finite()
                && max_stretch >= 0.0)
            {
                return Err(
                    "--max-rms and --max-stretch must be numbers that are not negative".into(),
                );
            }
            if args.len() < 2 {
                return Err("align needs the design points and the measured points".into());
            }
            finish(&args, 2)?;
            let design = points(&args[0])?;
            let mut measured = points(&args[1])?;
            if pair {
                // the measured marks in any order: each design mark takes the nearest one
                measured = pair_nearest(&design, &measured)?;
            }
            let t = fit(&design, &measured, allow)?;
            println!(
                "shift ({:+.4}, {:+.4}) mm, turn {:+.4} degrees, size {:.5}, residual {:.4} mm",
                t.dx, t.dy, t.theta_deg, t.scale, t.rms_mm
            );
            // the arguments for the other program are printed only for a fit that is accepted: a script that captures the output must not receive a rejected fit
            let verdict = accept(&t, max_rms, max_stretch);
            if isocam_args && verdict.is_ok() {
                // the marks as the options of eh3d-isocam, one pair for each row: design x, design y, measured x, measured y
                for line in isocam_lines(&design, &measured) {
                    println!("{line}");
                }
            }
            verdict
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
