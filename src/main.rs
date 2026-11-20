// EH3D-VISION-STUDIO - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

mod cli_args;

use cli_args::{finish, flag, num, switch};
use eh3d_vision_studio::align::{accept, fit, isocam_args as isocam_lines, pair_nearest};
use eh3d_vision_studio::batch::{check_detailed, list_pictures, report_csv, summary, NO_MARKS, OK};
use eh3d_vision_studio::config::Config;
use eh3d_vision_studio::correlate::{correlate, to_csv as correlate_csv};
use eh3d_vision_studio::cross::{find_crosses, CrossSettings};
use eh3d_vision_studio::diff::{
    compare as diff_compare, mask as diff_mask, to_csv as diff_csv, to_json as diff_json,
};
use eh3d_vision_studio::fiducial::{find, refine, Polarity, Settings};
use eh3d_vision_studio::flatten::flatten;
use eh3d_vision_studio::image::{from_bytes, Gray};
use eh3d_vision_studio::inspect::{
    find_package, judge, parse_table, summary as inspect_summary, to_csv as inspect_csv,
    to_json as inspect_json, Package,
};
use eh3d_vision_studio::lang::Lang;
use eh3d_vision_studio::lens::{fit_lens, undistort2};
use eh3d_vision_studio::limit::{read_limited, read_text_limited, write_bytes, write_text};
use eh3d_vision_studio::measure::{measure_parts, to_csv, to_json, Part, PartSettings};
use eh3d_vision_studio::offset::{
    estimate_nozzle, mirror_parts, nozzle_cfg, pick_offset, pick_offsets, to_csv as offset_csv,
    to_json as offset_json, to_table_csv as offset_table, OffsetSettings,
};
use eh3d_vision_studio::pads::{measure_row, to_csv as pads_csv, to_json as pads_json};
use eh3d_vision_studio::perspective::{parse_corners, rectify};
use eh3d_vision_studio::points::parse as parse_points;
use eh3d_vision_studio::quality;
use eh3d_vision_studio::report::html as report_html;
use eh3d_vision_studio::ring::find_rings;
use eh3d_vision_studio::scale::{from_pitch, Scale};
use eh3d_vision_studio::server::{
    apply_remote, check_link, config_lines, describe, link_settings, plain_token_warning, sc,
    Client,
};
use eh3d_vision_studio::shots::{plan as shots_plan, to_csv as shots_csv};
use eh3d_vision_studio::sim::to_pgm;
use eh3d_vision_studio::square::{find_squares, find_turned_crosses, ShapeSettings};
use std::env;
use std::path::Path;
use std::process::ExitCode;

const HELP: &str = "eh3d-vision: fiducials in a camera image

  eh3d-vision find IMAGE [--light] [--min-diam PX] [--max-diam PX] [--known-mm D] [--refine] [--undistort] [--flatten PX]   the round marks found (--known-mm gives millimetres, --refine the centre to a fraction of a pixel with a confidence, --undistort straightens the places with lens_k1 and lens_k2 of vision.cfg, --flatten takes uneven light out first)
  eh3d-vision align DESIGN.csv MEASURED.csv [--scale] [--pair-nearest] [--isocam-args] [--max-rms 0.1] [--max-stretch 0.01]   the turn and the shift that put the board where it is
  eh3d-vision measure IMAGE [--mm-per-px V] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   the parts of a picture: centre, turn, length, width and area in millimetres, as a CSV or JSON file
  eh3d-vision crosses IMAGE [--turned] [--light] [--min-size PX] [--max-size PX] [--flatten PX]   the cross-shaped marks found (plus signs; with --turned the arms may have any angle and the angle is printed)
  eh3d-vision offset IMAGE --package NAME [--table FILE] [--mm-per-px V] [--nozzle-x MM] [--nozzle-y MM] [--nominal-angle DEG] [--tolerance MM] [--max-offset MM] [--max-turn DEG] [--mirror] [--each] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   where a picked part is held, from a picture of it taken from below (one part in it): its offset from the axis of the nozzle and its turn from the way it should be held, the correction that puts it right, its size against the body of the package; --mirror when the camera sees the part turned over left to right; --each takes every part of the picture (a tray) and writes a table of corrections, one row for each; exit 1 when a part is the wrong size, too far off or turned too much
  eh3d-vision nozzle IMAGE IMAGE... --package NAME [--table FILE] [--mm-per-px V] [--mirror] [--light] [--flatten PX] [--output FILE] [--replace]   where the axis of the nozzle is, from pictures of parts picked with it (one part in each, at least two pictures): the lines nozzle_x_mm and nozzle_y_mm for vision.cfg, with how spread the parts are
  eh3d-vision quality IMAGE [--min-contrast V] [--min-mean V] [--max-mean V] [--min-sharpness V] [--format csv|json] [--output FILE] [--replace]   whether a picture is good enough to measure from: its sharpness, contrast and mean grey level, and why it is refused (flat, dark, bright or blurry); exit 1 when it is refused
  eh3d-vision inspect IMAGE --package NAME [--table FILE] [--mm-per-px V] [--tolerance MM] [--max-angle DEG] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   each part of a picture compared with the body of its package from a table (CSV with package, body_long_mm, body_short_mm), one verdict per part; exit 1 when a part fails
  eh3d-vision shots (--package NAME [--table FILE] | --body-long MM --body-short MM) --fov-w MM --fov-h MM [--overlap F] [--margin MM] [--output FILE] [--replace]   the places of the camera that cover a part bigger than its field (millimetres from the centre of the part), row after row, as CSV; no camera is needed
  eh3d-vision lens IMAGE [--columns C] [--rows R] [--max-rms PX] [--light] [--flatten PX]   the two radial factors of the lens from a grid of marks (columns and rows as grid_columns and grid_rows), refused when the marks stay farther than --max-rms pixels from their lattice
  eh3d-vision rings IMAGE [--light] [--min-size PX] [--max-size PX] [--split PX] [--flatten PX]   the ring marks found (donuts: a round mark with a hole), with the outer and inner diameter, the size class when the rings are of two sizes (--split gives the diameter that tells small from large, else it is found from the diameters), the score of the shape and a confidence
  eh3d-vision squares IMAGE [--light] [--min-size PX] [--max-size PX] [--flatten PX]   the square marks found, turned by any angle (centre, side and angle)
  eh3d-vision calibrate IMAGE --pitch-mm D [--light] [--tolerance 0.05] [--flatten PX]   the millimetres of a pixel from a target whose neighbouring marks are D millimetres apart
  eh3d-vision batch FOLDER [--light] [--flatten PX] [--report FILE.csv] [--html FILE.html] [--lang CODE] [--replace]   every PNG and PGM picture of a folder: marks, scores and diameters, or the code of what went wrong, one line each; --html writes a page that stands alone, with a drawing of each picture and the warnings, in the language of --lang or of vision.cfg
  eh3d-vision correlate IMAGE [--diameter PX] [--min-score S] [--light] [--flatten PX]   the round marks found by sliding a disc of that diameter over the picture and working out the correlation at every place (a mark that the search by threshold loses on uneven light), with the place to a fraction of a pixel and the score
  eh3d-vision rectify IMAGE --corners X1,Y1,X2,Y2,X3,Y3,X4,Y4 --output FILE.pgm [--size WxH] [--fill V] [--replace]   the picture drawn again as seen from straight above, from the four corners of a rectangle in it (top left, top right, bottom right, bottom left, in pixels)
  eh3d-vision pads IMAGE [--mm-per-px V] [--expect-pitch MM] [--tolerance F] [--light] [--flatten PX] [--format csv|json] [--output FILE] [--replace]   a row of pads: the pitch, how much it varies, how far the pads lie from the row and their size; exit 1 when the pitch varies, a pad is off the row or the pitch is not the one asked for
  eh3d-vision diff BEFORE AFTER [--threshold V] [--min-area PX] [--max-changed F] [--mask FILE.pgm] [--format csv|json] [--output FILE] [--replace]   two pictures of the same size compared: how many pixels changed, how much, where and in how many regions; exit 1 when more than --max-changed of the pixels changed
  eh3d-vision server health|config|send KIND ... [--address HOST:PORT] [--machine ID]   the EH3D server of the machine (server_address, server_token and server_machine of vision.cfg): health asks whether it answers, config reads the values of the configuration file that it holds (written as lines for vision.cfg with --output, never over a file without --replace), send quality|offset|pads|diff takes the arguments of the command of that name and sends its result as JSON

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

/// The picture of a file, read within the size limit, with the uneven light taken out when the option or the configuration file gives a window.
fn picture(path: &str, cfg: &Config, flatten_px: Option<usize>) -> Result<Gray, String> {
    let img = from_bytes(&read_limited(Path::new(path), cfg.max_file_mib)?)?;
    match flatten_px.unwrap_or(cfg.flatten_window) {
        0 => Ok(img),
        window => flatten(&img, window),
    }
}

/// Prints a text, or writes it to a new file when `output` is given.
fn deliver(text: &str, output: Option<String>, replace: bool) -> Result<(), String> {
    match output {
        Some(path) => write_text(Path::new(&path), text, replace),
        None => {
            print!("{text}");
            Ok(())
        }
    }
}

/// The settings of the search for squares or for turned crosses that the configuration file gives: each shape has its own range of size and its own elongation.
fn shape_settings(s: &mut ShapeSettings, cfg: &Config, squares: bool) {
    s.threshold = cfg.threshold;
    if squares {
        s.min_size = cfg.square_min_size;
        s.max_size = cfg.square_max_size;
        s.max_elongation = cfg.square_max_elongation;
    } else {
        s.min_size = cfg.cross_min_size;
        s.max_size = cfg.cross_max_size;
        s.max_elongation = cfg.cross_max_elongation;
    }
    s.min_fill = cfg.square_min_fill;
    s.min_score = cfg.cross_min_score;
    s.max_spots = cfg.max_parts;
}

/// A whole number, or an error that names what it is for.
fn count(text: &str, what: &str) -> Result<usize, String> {
    text.trim()
        .parse::<usize>()
        .map_err(|_| format!("{what} '{text}' is not a whole number"))
}

/// What `offset` and `nozzle` read from their options and the configuration file before the picture is looked at.
struct PickOptions {
    parts: PartSettings,
    flatten_px: Option<usize>,
    mm_per_px: Option<f64>,
    package: String,
    table: Option<String>,
    settings: OffsetSettings,
    /// The camera sees the part turned over left to right (`--mirror` or `pick_mirror`).
    mirror: bool,
}

/// Takes the options of a command about a part held by a nozzle out of `args`: the polarity, the light, the scale, the package and its table, and the numbers of the offset.
fn pick_options(cmd: &str, args: &mut Vec<String>, cfg: &Config) -> Result<PickOptions, String> {
    let mut parts = PartSettings::default();
    if switch(args, "--light") {
        parts.polarity = Polarity::Light;
    }
    let mirror = switch(args, "--mirror") || cfg.pick_mirror;
    let flatten_px = flag(args, "--flatten")?
        .map(|v| count(&v, "--flatten"))
        .transpose()?;
    let mm_per_px = flag(args, "--mm-per-px")?
        .map(|v| num(&v, "--mm-per-px"))
        .transpose()?;
    let package = flag(args, "--package")?;
    let table = flag(args, "--table")?;
    let package = package
        .or_else(|| (!cfg.inspect_package.is_empty()).then(|| cfg.inspect_package.clone()))
        .ok_or_else(|| format!("{cmd} needs --package, the name of the package in the table (or inspect_package in vision.cfg)"))?;
    let mut settings = cfg.offset_settings();
    for (name, slot) in [
        ("--nozzle-x", &mut settings.nozzle_x_mm),
        ("--nozzle-y", &mut settings.nozzle_y_mm),
        ("--nominal-angle", &mut settings.nominal_angle_deg),
        ("--tolerance", &mut settings.tolerance_mm),
        ("--max-offset", &mut settings.max_offset_mm),
        ("--max-turn", &mut settings.max_turn_deg),
    ] {
        if let Some(v) = flag(args, name)? {
            *slot = num(&v, name)?;
        }
    }
    parts.threshold = cfg.threshold;
    parts.min_area_px = cfg.part_min_area;
    parts.max_area_px = cfg.part_max_area;
    parts.max_parts = cfg.max_parts;
    Ok(PickOptions {
        parts,
        flatten_px,
        mm_per_px,
        package,
        table,
        settings,
        mirror,
    })
}

/// The millimetres of a pixel and the package (read from its table) that the options name.
fn pick_inputs(cmd: &str, opts: &PickOptions, cfg: &Config) -> Result<(f64, Package), String> {
    let mm = opts.mm_per_px.unwrap_or(cfg.mm_per_px);
    if mm <= 0.0 {
        return Err(format!(
            "{cmd} needs the scale: --mm-per-px, or mm_per_px in vision.cfg (the command calibrate works it out)"
        ));
    }
    let table_file = opts
        .table
        .clone()
        .unwrap_or_else(|| cfg.package_table.clone());
    if table_file.is_empty() {
        return Err(format!(
            "{cmd} needs the table of packages: --table, or package_table in vision.cfg"
        ));
    }
    let table = parse_table(&read_text_limited(
        Path::new(&table_file),
        cfg.max_file_mib,
    )?)
    .map_err(|e| format!("{table_file}: {e}"))?;
    let body = find_package(&table, &opts.package)?.clone();
    Ok((mm, body))
}

/// The parts of a picture of picked parts, in millimetres: the picture is read, checked when `quality_check` is on (as it was taken, before the light is flattened), measured and turned over
/// left to right when the camera sees them mirrored.
fn picked_parts(
    path: &str,
    cfg: &Config,
    opts: &PickOptions,
    mm_per_px: f64,
) -> Result<Vec<Part>, String> {
    let raw = picture(path, cfg, Some(0))?;
    if cfg.quality_check {
        quality::require_good(&raw, &cfg.quality_settings()).map_err(|e| format!("{path}: {e}"))?;
    }
    let img = match opts.flatten_px.unwrap_or(cfg.flatten_window) {
        0 => raw,
        window => flatten(&raw, window)?,
    };
    let parts = measure_parts(&img, &opts.parts, Scale { mm_per_px })?;
    Ok(if opts.mirror {
        mirror_parts(&parts)
    } else {
        parts
    })
}

/// What `pads` and `server send pads` do: the parts of the picture are measured and the row of pads is judged. The text, whether the row passes and a line for the error output.
fn pads_run(
    args: &mut Vec<String>,
    cfg: &Config,
    json: bool,
) -> Result<(String, bool, String), String> {
    let mut s = PartSettings::default();
    if switch(args, "--light") {
        s.polarity = Polarity::Light;
    }
    let flatten_px = flag(args, "--flatten")?
        .map(|v| count(&v, "--flatten"))
        .transpose()?;
    let mm_per_px = flag(args, "--mm-per-px")?
        .map(|v| num(&v, "--mm-per-px"))
        .transpose()?
        .unwrap_or(cfg.mm_per_px);
    let mut settings = cfg.pad_settings();
    if let Some(v) = flag(args, "--tolerance")? {
        settings.tolerance = num(&v, "--tolerance")?;
    }
    settings.expected_pitch_mm = flag(args, "--expect-pitch")?
        .map(|v| num(&v, "--expect-pitch"))
        .transpose()?;
    finish(args, 1)?;
    let path = args.first().ok_or("pads needs an image")?;
    if mm_per_px <= 0.0 {
        return Err(
            "pads needs the scale: --mm-per-px, or mm_per_px in vision.cfg (the command calibrate works it out)"
                .into(),
        );
    }
    s.threshold = cfg.threshold;
    s.min_area_px = cfg.part_min_area;
    s.max_area_px = cfg.part_max_area;
    s.max_parts = cfg.max_parts;
    let img = picture(path, cfg, flatten_px)?;
    let parts = measure_parts(&img, &s, Scale { mm_per_px })?;
    let row = measure_row(&parts, &settings)?;
    let note = format!(
        "{} pads, pitch {:.4} mm (from {:.4} to {:.4}), {:.4} mm off the row, row at {:+.2} degrees: {}",
        row.count,
        row.pitch_mm,
        row.pitch_min_mm,
        row.pitch_max_mm,
        row.row_deviation_mm,
        row.row_angle_deg,
        if row.passes() {
            "good".to_string()
        } else {
            format!("refused ({})", row.reasons.join(", "))
        }
    );
    let text = if json {
        pads_json(&row)
    } else {
        pads_csv(&row)
    };
    Ok((text, row.passes(), note))
}

/// What `diff` and `server send diff` do: the two pictures are compared. The text, whether they count as the same and a line for the error output. `--mask` writes the picture of the changes.
fn diff_run(
    args: &mut Vec<String>,
    cfg: &Config,
    json: bool,
    replace: bool,
) -> Result<(String, bool, String), String> {
    let mut s = cfg.diff_settings();
    if let Some(v) = flag(args, "--threshold")? {
        s.threshold = u8::try_from(count(&v, "--threshold")?)
            .map_err(|_| format!("--threshold '{v}' is not from 0 to 254"))?;
    }
    if let Some(v) = flag(args, "--min-area")? {
        s.min_area = count(&v, "--min-area")? as u64;
    }
    if let Some(v) = flag(args, "--max-changed")? {
        s.max_changed = num(&v, "--max-changed")?;
    }
    let mask_file = flag(args, "--mask")?;
    finish(args, 2)?;
    let (before, after) = match (args.first(), args.get(1)) {
        (Some(a), Some(b)) => (a, b),
        _ => return Err("diff needs two pictures: the one before and the one after".into()),
    };
    let a = picture(before, cfg, Some(0))?;
    let b = picture(after, cfg, Some(0))?;
    let d = diff_compare(&a, &b, &s)?;
    if let Some(file) = mask_file {
        write_bytes(Path::new(&file), &to_pgm(&diff_mask(&a, &b, &s)?), replace)?;
    }
    let note = format!(
        "{} of {} pixels changed ({:.3} percent), {} regions, the largest difference {} levels: {}",
        d.changed_px,
        d.total_px,
        d.changed_share * 100.0,
        d.regions,
        d.max_abs,
        if d.same { "the same" } else { "changed" }
    );
    let text = if json { diff_json(&d) } else { diff_csv(&d) };
    Ok((text, d.same, note))
}

/// The client of the server for the settings of the configuration file, with `--address` and `--machine` over them.
fn server_settings(args: &mut Vec<String>, cfg: &Config) -> Result<sc::Settings, String> {
    let mut s = link_settings(cfg)?;
    if let Some(a) = flag(args, "--address")? {
        s.set("server_address", &a).map_err(|e| e.to_string())?;
    }
    if let Some(m) = flag(args, "--machine")? {
        s.set("server_machine", &m).map_err(|e| e.to_string())?;
    }
    check_link(&s)?;
    if let Some(w) = plain_token_warning(&s) {
        eprintln!("warning: {w}");
    }
    Ok(s)
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
            let undistort_on = switch(&mut args, "--undistort");
            let flatten_px = flag(&mut args, "--flatten")?
                .map(|v| count(&v, "--flatten"))
                .transpose()?;
            let known = flag(&mut args, "--known-mm")?
                .map(|v| num(&v, "--known-mm"))
                .transpose()?;
            finish(&args, 1)?;
            let path = args.first().ok_or("find needs an image")?;
            let cfg = config();
            let img = picture(path, &cfg, flatten_px)?;
            let mut found = find(&img, &s)?;
            // the centre to a fraction of a pixel, and the confidence of each mark, when the option or the configuration file asks for it
            let mut confidence: Vec<f64> = Vec::new();
            if refine_on || cfg.refine_subpixel {
                for f in &mut found {
                    let r = refine(&img, f, s.polarity, cfg.full_contrast);
                    (f.x, f.y) = (r.x, r.y);
                    confidence.push(r.confidence);
                }
            }
            let centre = (img.w as f64 / 2.0, img.h as f64 / 2.0);
            if undistort_on {
                if cfg.lens_k1 == 0.0 && cfg.lens_k2 == 0.0 {
                    return Err("--undistort needs lens_k1 and lens_k2 in vision.cfg (the command lens works them out)".into());
                }
                let norm = centre.0.hypot(centre.1);
                for f in &mut found {
                    (f.x, f.y) = undistort2((f.x, f.y), centre, cfg.lens_k1, cfg.lens_k2, norm);
                }
            }
            let scale = match (known, found.first()) {
                (Some(mm), Some(f)) => Some(Scale::from_known_diameter(f.diameter, mm)?),
                _ => None,
            };
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
        "batch" => {
            let mut settings = Settings::default();
            if switch(&mut args, "--light") {
                settings.polarity = Polarity::Light;
            }
            let replace = switch(&mut args, "--replace");
            let flatten_px = flag(&mut args, "--flatten")?
                .map(|v| count(&v, "--flatten"))
                .transpose()?;
            let report = flag(&mut args, "--report")?;
            let html_path = flag(&mut args, "--html")?;
            let lang_code = flag(&mut args, "--lang")?;
            finish(&args, 1)?;
            let folder = args.first().ok_or("batch needs a folder")?;
            let cfg = config();
            let replace = replace || cfg.replace_output;
            let lang = match lang_code {
                Some(code) => Lang::from_code(&code).ok_or_else(|| {
                    format!("--lang '{code}' is not en, es, de, fr, it, ja, zh or pt")
                })?,
                None => Lang::from_setting(&cfg.language),
            };
            settings.min_diameter = cfg.min_diameter;
            settings.max_diameter = cfg.max_diameter;
            settings.min_fill = cfg.min_fill;
            settings.max_elongation = cfg.max_elongation;
            let window = flatten_px.unwrap_or(cfg.flatten_window);
            let files = list_pictures(Path::new(folder), cfg.batch_max_files)?;
            let items: Vec<_> = files
                .iter()
                .map(|f| check_detailed(f, cfg.max_file_mib, &settings, cfg.threshold, window))
                .collect();
            let rows: Vec<_> = items.iter().map(|i| i.row.clone()).collect();
            if let Some(path) = &html_path {
                write_text(
                    Path::new(path),
                    &report_html(&items, lang, cfg.report_low_score),
                    replace,
                )?;
                eprintln!("the HTML report is in {path}");
            }
            let text = report_csv(&rows);
            let line = summary(&rows);
            match report {
                Some(path) => {
                    write_text(Path::new(&path), &text, replace)?;
                    println!("{line}; the report is in {path}");
                }
                None => {
                    print!("{text}");
                    eprintln!("{line}");
                }
            }
            // a picture that could not be used is an error for a script, after the report is written
            if rows.iter().any(|r| r.status != OK && r.status != NO_MARKS) {
                return Err(
                    "some pictures could not be used (see the status column of the report)".into(),
                );
            }
            Ok(())
        }
        "calibrate" => {
            let mut settings = Settings::default();
            if switch(&mut args, "--light") {
                settings.polarity = Polarity::Light;
            }
            let flatten_px = flag(&mut args, "--flatten")?
                .map(|v| count(&v, "--flatten"))
                .transpose()?;
            let cfg = config();
            let pitch = match flag(&mut args, "--pitch-mm")? {
                Some(v) => num(&v, "--pitch-mm")?,
                None if cfg.pitch_mm > 0.0 => cfg.pitch_mm,
                None => {
                    return Err("calibrate needs --pitch-mm (or pitch_mm in vision.cfg), the distance in millimetres between neighbouring marks".into())
                }
            };
            let tolerance = flag(&mut args, "--tolerance")?
                .map(|v| num(&v, "--tolerance"))
                .transpose()?
                .unwrap_or(cfg.pitch_tolerance);
            finish(&args, 1)?;
            let path = args.first().ok_or("calibrate needs an image")?;
            settings.min_diameter = cfg.min_diameter;
            settings.max_diameter = cfg.max_diameter;
            settings.min_fill = cfg.min_fill;
            settings.max_elongation = cfg.max_elongation;
            let img = picture(path, &cfg, flatten_px)?;
            let marks: Vec<(f64, f64)> =
                find(&img, &settings)?.iter().map(|f| (f.x, f.y)).collect();
            let p = from_pitch(&marks, pitch, tolerance)?;
            println!(
                "{} marks, {pitch} mm apart: {:.6} mm per pixel (the distances to the nearest neighbour differ by {:.1} %)",
                p.marks,
                p.scale.mm_per_px,
                p.spread * 100.0
            );
            println!("mm_per_px = {:.6}", p.scale.mm_per_px);
            Ok(())
        }
        "shots" => {
            let replace = switch(&mut args, "--replace");
            let package = flag(&mut args, "--package")?;
            let table_path = flag(&mut args, "--table")?;
            let long = flag(&mut args, "--body-long")?
                .map(|v| num(&v, "--body-long"))
                .transpose()?;
            let short = flag(&mut args, "--body-short")?
                .map(|v| num(&v, "--body-short"))
                .transpose()?;
            let fov_w = flag(&mut args, "--fov-w")?
                .map(|v| num(&v, "--fov-w"))
                .transpose()?
                .ok_or(
                    "shots needs --fov-w, the width of the field of the camera in millimetres",
                )?;
            let fov_h = flag(&mut args, "--fov-h")?
                .map(|v| num(&v, "--fov-h"))
                .transpose()?
                .ok_or(
                    "shots needs --fov-h, the height of the field of the camera in millimetres",
                )?;
            let cfg = config();
            let overlap = flag(&mut args, "--overlap")?
                .map(|v| num(&v, "--overlap"))
                .transpose()?
                .unwrap_or(cfg.shots_overlap);
            let margin = flag(&mut args, "--margin")?
                .map(|v| num(&v, "--margin"))
                .transpose()?
                .unwrap_or(cfg.shots_margin);
            let output = flag(&mut args, "--output")?;
            finish(&args, 0)?;
            // nothing about the part on the command line: the size or the package of the configuration
            let (package, long, short) = if package.is_none() && long.is_none() && short.is_none() {
                if cfg.shots_part_width > 0.0 && cfg.shots_part_height > 0.0 {
                    (
                        None,
                        Some(cfg.shots_part_width),
                        Some(cfg.shots_part_height),
                    )
                } else if !cfg.inspect_package.is_empty() {
                    (Some(cfg.inspect_package.clone()), None, None)
                } else {
                    (package, long, short)
                }
            } else {
                (package, long, short)
            };
            let (width, height) = match (package, long, short) {
                (Some(name), None, None) => {
                    let file = table_path.unwrap_or_else(|| cfg.package_table.clone());
                    if file.is_empty() {
                        return Err("shots with --package needs the table of packages: --table, or package_table in vision.cfg".into());
                    }
                    let table =
                        parse_table(&read_text_limited(Path::new(&file), cfg.max_file_mib)?)
                            .map_err(|e| format!("{file}: {e}"))?;
                    let body = find_package(&table, &name)?;
                    (body.long_mm, body.short_mm)
                }
                (None, Some(l), Some(s)) => (l, s),
                _ => {
                    return Err("shots needs either --package (with a table) or both --body-long and --body-short (or shots_part_width and shots_part_height, or inspect_package, in vision.cfg)".into())
                }
            };
            let plan = shots_plan(width, height, fov_w, fov_h, overlap, margin)?;
            deliver(&shots_csv(&plan), output, replace || cfg.replace_output)?;
            eprintln!(
                "{} pictures ({} x {})",
                plan.shots.len(),
                plan.columns,
                plan.rows
            );
            Ok(())
        }
        "offset" => {
            let replace = switch(&mut args, "--replace");
            let each = switch(&mut args, "--each");
            let cfg = config();
            let opts = pick_options("offset", &mut args, &cfg)?;
            let replace = replace || cfg.replace_output;
            let format = flag(&mut args, "--format")?.unwrap_or_else(|| "csv".to_string());
            let output = flag(&mut args, "--output")?;
            finish(&args, 1)?;
            if !matches!(format.as_str(), "csv" | "json") {
                return Err(format!("--format '{format}' is not csv or json"));
            }
            if each && format == "json" {
                return Err("--each writes a CSV table; --format json is for one part".into());
            }
            let path = args.first().ok_or("offset needs an image")?;
            let (mm, body) = pick_inputs("offset", &opts, &cfg)?;
            let parts = picked_parts(path, &cfg, &opts, mm)?;
            if each {
                // every part of the picture, each on its own: the table of corrections of a tray
                let held = pick_offsets(&parts, &body, &opts.settings)?;
                deliver(&offset_table(&held, &body), output, replace)?;
                let failing = held.iter().filter(|o| !o.passes()).count();
                eprintln!(
                    "{} parts, {} pass and {} do not",
                    held.len(),
                    held.len() - failing,
                    failing
                );
                if failing > 0 {
                    return Err(format!(
                        "{failing} of {} parts do not pass for the package {}",
                        held.len(),
                        body.name
                    ));
                }
                return Ok(());
            }
            let held = pick_offset(&parts, &body, &opts.settings)?;
            let text = if format == "json" {
                offset_json(&held, &body)
            } else {
                offset_csv(&held, &body)
            };
            deliver(&text, output, replace)?;
            let (cx, cy, cr) = held.correction();
            eprintln!(
                "offset ({:+.3}, {:+.3}) mm, turn {:+.2} degrees; correction ({:+.3}, {:+.3}) mm, {:+.2} degrees",
                held.x_mm, held.y_mm, held.rot_deg, cx, cy, cr
            );
            // a part that fails is an error for a script, after the file is written
            if !held.passes() {
                return Err(format!(
                    "the part does not pass for the package {}: {}",
                    body.name,
                    held.reasons.join(", ")
                ));
            }
            Ok(())
        }
        "nozzle" => {
            let replace = switch(&mut args, "--replace");
            let cfg = config();
            let opts = pick_options("nozzle", &mut args, &cfg)?;
            let replace = replace || cfg.replace_output;
            let output = flag(&mut args, "--output")?;
            finish(&args, usize::MAX)?;
            if args.len() < 2 {
                return Err(
                    "nozzle needs at least two pictures, each with one part picked with the nozzle"
                        .into(),
                );
            }
            let (mm, body) = pick_inputs("nozzle", &opts, &cfg)?;
            let mut held = Vec::new();
            for path in &args {
                let parts = picked_parts(path, &cfg, &opts, mm)?;
                held.push(
                    pick_offset(&parts, &body, &opts.settings)
                        .map_err(|e| format!("{path}: {e}"))?,
                );
            }
            let e = estimate_nozzle(&held)?;
            deliver(&nozzle_cfg(&e), output, replace)?;
            eprintln!(
                "the axis of the nozzle from {} parts: ({:+.4}, {:+.4}) mm; spread {:.4} mm in x and {:.4} mm in y, the farthest part {:.4} mm from it; mean turn {:+.2} degrees",
                e.count, e.x_mm, e.y_mm, e.spread_x_mm, e.spread_y_mm, e.farthest_mm, e.mean_turn_deg
            );
            Ok(())
        }
        "quality" => {
            let replace = switch(&mut args, "--replace");
            let cfg = config();
            let mut s = cfg.quality_settings();
            for (name, slot) in [
                ("--min-contrast", &mut s.min_contrast),
                ("--min-mean", &mut s.min_mean),
                ("--max-mean", &mut s.max_mean),
                ("--min-sharpness", &mut s.min_sharpness),
            ] {
                if let Some(v) = flag(&mut args, name)? {
                    *slot = num(&v, name)?;
                }
            }
            let format = flag(&mut args, "--format")?.unwrap_or_else(|| "csv".to_string());
            let output = flag(&mut args, "--output")?;
            finish(&args, 1)?;
            if !matches!(format.as_str(), "csv" | "json") {
                return Err(format!("--format '{format}' is not csv or json"));
            }
            let path = args.first().ok_or("quality needs an image")?;
            let img = picture(path, &cfg, Some(0))?;
            let q = quality::measure(&img)?;
            let why = quality::reasons(&q, &s)?;
            let text = if format == "json" {
                quality::to_json(&q, &why)
            } else {
                quality::to_csv(&q, &why)
            };
            deliver(&text, output, replace || cfg.replace_output)?;
            eprintln!(
                "sharpness {:.2}, contrast {:.2}, mean level {:.2}: {}",
                q.sharpness,
                q.contrast,
                q.mean,
                if why.is_empty() {
                    "good".to_string()
                } else {
                    format!("poor ({})", why.join(", "))
                }
            );
            // a picture that is refused is an error for a script, after the file is written
            if !why.is_empty() {
                return Err(format!(
                    "the picture is not good enough to measure from: {}",
                    why.join(", ")
                ));
            }
            Ok(())
        }
        "inspect" => {
            let mut s = PartSettings::default();
            if switch(&mut args, "--light") {
                s.polarity = Polarity::Light;
            }
            let replace = switch(&mut args, "--replace");
            let flatten_px = flag(&mut args, "--flatten")?
                .map(|v| count(&v, "--flatten"))
                .transpose()?;
            let mm_per_px = flag(&mut args, "--mm-per-px")?
                .map(|v| num(&v, "--mm-per-px"))
                .transpose()?;
            let package = flag(&mut args, "--package")?;
            let table_path = flag(&mut args, "--table")?;
            let cfg = config();
            let package = package
                .or_else(|| (!cfg.inspect_package.is_empty()).then(|| cfg.inspect_package.clone()))
                .ok_or("inspect needs --package, the name of the package in the table (or inspect_package in vision.cfg)")?;
            let replace = replace || cfg.replace_output;
            let tolerance = flag(&mut args, "--tolerance")?
                .map(|v| num(&v, "--tolerance"))
                .transpose()?
                .unwrap_or(cfg.inspect_tolerance);
            let max_angle = flag(&mut args, "--max-angle")?
                .map(|v| num(&v, "--max-angle"))
                .transpose()?
                .unwrap_or(cfg.inspect_max_angle);
            let format = flag(&mut args, "--format")?.unwrap_or_else(|| "csv".to_string());
            let output = flag(&mut args, "--output")?;
            finish(&args, 1)?;
            if !matches!(format.as_str(), "csv" | "json") {
                return Err(format!("--format '{format}' is not csv or json"));
            }
            let path = args.first().ok_or("inspect needs an image")?;
            let mm = mm_per_px.unwrap_or(cfg.mm_per_px);
            if mm <= 0.0 {
                return Err(
                    "inspect needs the scale: --mm-per-px, or mm_per_px in vision.cfg (the command calibrate works it out)"
                        .into(),
                );
            }
            let table_file = table_path.unwrap_or_else(|| cfg.package_table.clone());
            if table_file.is_empty() {
                return Err(
                    "inspect needs the table of packages: --table, or package_table in vision.cfg"
                        .into(),
                );
            }
            let table = parse_table(&read_text_limited(
                Path::new(&table_file),
                cfg.max_file_mib,
            )?)
            .map_err(|e| format!("{table_file}: {e}"))?;
            let body = find_package(&table, &package)?;
            s.threshold = cfg.threshold;
            s.min_area_px = cfg.part_min_area;
            s.max_area_px = cfg.part_max_area;
            s.max_parts = cfg.max_parts;
            let img = picture(path, &cfg, flatten_px)?;
            let parts = measure_parts(&img, &s, Scale { mm_per_px: mm })?;
            if parts.is_empty() {
                return Err(
                    "no_parts: no part was found in the picture, so nothing was inspected".into(),
                );
            }
            let judged = judge(&parts, body, tolerance, max_angle)?;
            let text = if format == "json" {
                inspect_json(&judged, body, tolerance)
            } else {
                inspect_csv(&judged, body)
            };
            deliver(&text, output, replace)?;
            let line = inspect_summary(&judged);
            eprintln!("{line}");
            // a part that fails is an error for a script, after the verdicts are written
            if judged.iter().any(|j| !j.passes()) {
                return Err(format!("some parts do not fit the package {}", body.name));
            }
            Ok(())
        }
        "lens" => {
            let mut settings = Settings::default();
            if switch(&mut args, "--light") {
                settings.polarity = Polarity::Light;
            }
            let flatten_px = flag(&mut args, "--flatten")?
                .map(|v| count(&v, "--flatten"))
                .transpose()?;
            let cfg = config();
            let columns = flag(&mut args, "--columns")?
                .map(|v| count(&v, "--columns"))
                .transpose()?
                .unwrap_or(cfg.grid_columns);
            let rows = flag(&mut args, "--rows")?
                .map(|v| count(&v, "--rows"))
                .transpose()?
                .unwrap_or(cfg.grid_rows);
            let max_rms = flag(&mut args, "--max-rms")?
                .map(|v| num(&v, "--max-rms"))
                .transpose()?
                .unwrap_or(cfg.lens_max_rms);
            finish(&args, 1)?;
            let path = args.first().ok_or("lens needs an image")?;
            settings.min_diameter = cfg.min_diameter;
            settings.max_diameter = cfg.max_diameter;
            settings.min_fill = cfg.min_fill;
            settings.max_elongation = cfg.max_elongation;
            let img = picture(path, &cfg, flatten_px)?;
            let marks: Vec<(f64, f64)> =
                find(&img, &settings)?.iter().map(|f| (f.x, f.y)).collect();
            let centre = (img.w as f64 / 2.0, img.h as f64 / 2.0);
            let l = fit_lens(
                &marks,
                columns,
                rows,
                centre,
                centre.0.hypot(centre.1),
                max_rms,
            )?;
            println!(
                "{} marks: k1 {:.5}, k2 {:.5}; before the correction they were {:.3} px from their lattice, after it {:.3} px",
                l.marks, l.k1, l.k2, l.rms_before, l.rms_after
            );
            println!("lens_k1 = {:.6}", l.k1);
            println!("lens_k2 = {:.6}", l.k2);
            Ok(())
        }
        "rings" => {
            let light = switch(&mut args, "--light");
            let flatten_px = flag(&mut args, "--flatten")?
                .map(|v| count(&v, "--flatten"))
                .transpose()?;
            let cfg = config();
            let mut s = cfg.ring_settings();
            if light {
                s.polarity = Polarity::Light;
            }
            if let Some(v) = flag(&mut args, "--min-size")? {
                s.min_diameter = num(&v, "--min-size")?;
            }
            if let Some(v) = flag(&mut args, "--max-size")? {
                s.max_diameter = num(&v, "--max-size")?;
            }
            if let Some(v) = flag(&mut args, "--split")? {
                s.split = num(&v, "--split")?;
            }
            finish(&args, 1)?;
            let path = args.first().ok_or("rings needs an image")?;
            let img = picture(path, &cfg, flatten_px)?;
            let found = find_rings(&img, &s)?;
            for (i, r) in found.iter().enumerate() {
                println!(
                    "{}: ({:.2}, {:.2}) px, outer {:.1} px, inner {:.1} px, size {}, score {:.2}, confidence {:.2}",
                    i + 1,
                    r.x,
                    r.y,
                    r.outer,
                    r.inner,
                    r.size.word(),
                    r.score,
                    r.confidence
                );
            }
            if found.is_empty() {
                println!("no rings found");
            }
            Ok(())
        }
        "squares" => {
            let mut s = ShapeSettings::default();
            if switch(&mut args, "--light") {
                s.polarity = Polarity::Light;
            }
            let flatten_px = flag(&mut args, "--flatten")?
                .map(|v| count(&v, "--flatten"))
                .transpose()?;
            let cfg = config();
            shape_settings(&mut s, &cfg, true);
            if let Some(v) = flag(&mut args, "--min-size")? {
                s.min_size = num(&v, "--min-size")?;
            }
            if let Some(v) = flag(&mut args, "--max-size")? {
                s.max_size = num(&v, "--max-size")?;
            }
            finish(&args, 1)?;
            let path = args.first().ok_or("squares needs an image")?;
            let img = picture(path, &cfg, flatten_px)?;
            let found = find_squares(&img, &s)?;
            for (i, q) in found.iter().enumerate() {
                println!(
                    "{}: ({:.2}, {:.2}) px, side {:.1} px, angle {:+.1} degrees, score {:.2}",
                    i + 1,
                    q.x,
                    q.y,
                    q.side,
                    q.angle_deg,
                    q.score
                );
            }
            if found.is_empty() {
                println!("no squares found");
            }
            Ok(())
        }
        "crosses" => {
            let turned = switch(&mut args, "--turned");
            let mut s = CrossSettings::default();
            if switch(&mut args, "--light") {
                s.polarity = Polarity::Light;
            }
            let flatten_px = flag(&mut args, "--flatten")?
                .map(|v| count(&v, "--flatten"))
                .transpose()?;
            let cfg = config();
            s.threshold = cfg.threshold;
            s.min_size = cfg.cross_min_size;
            s.max_size = cfg.cross_max_size;
            s.max_elongation = cfg.cross_max_elongation;
            s.max_spots = cfg.max_parts;
            if let Some(v) = flag(&mut args, "--min-size")? {
                s.min_size = num(&v, "--min-size")?;
            }
            if let Some(v) = flag(&mut args, "--max-size")? {
                s.max_size = num(&v, "--max-size")?;
            }
            finish(&args, 1)?;
            let path = args.first().ok_or("crosses needs an image")?;
            let img = picture(path, &cfg, flatten_px)?;
            if turned {
                let mut shape = ShapeSettings {
                    polarity: s.polarity,
                    min_size: s.min_size,
                    max_size: s.max_size,
                    ..ShapeSettings::default()
                };
                shape_settings(&mut shape, &cfg, false);
                shape.polarity = s.polarity;
                shape.min_size = s.min_size;
                shape.max_size = s.max_size;
                let found = find_turned_crosses(&img, &shape)?;
                for (i, c) in found.iter().enumerate() {
                    println!(
                        "{}: ({:.2}, {:.2}) px, size {:.0} px, arm {:.1} px, angle {:+.1} degrees, score {:.2}",
                        i + 1,
                        c.x,
                        c.y,
                        c.size,
                        c.arm,
                        c.angle_deg,
                        c.score
                    );
                }
                if found.is_empty() {
                    println!("no crosses found");
                }
                return Ok(());
            }
            let found = find_crosses(&img, &s)?;
            for (i, c) in found.iter().enumerate() {
                println!(
                    "{}: ({:.2}, {:.2}) px, size {:.0} px, arm {:.1} px, score {:.2}",
                    i + 1,
                    c.x,
                    c.y,
                    c.size,
                    c.arm,
                    c.score
                );
            }
            if found.is_empty() {
                println!("no crosses found");
            }
            Ok(())
        }
        "measure" => {
            let mut s = PartSettings::default();
            if switch(&mut args, "--light") {
                s.polarity = Polarity::Light;
            }
            let replace = switch(&mut args, "--replace");
            let flatten_px = flag(&mut args, "--flatten")?
                .map(|v| count(&v, "--flatten"))
                .transpose()?;
            let mm_per_px = flag(&mut args, "--mm-per-px")?
                .map(|v| num(&v, "--mm-per-px"))
                .transpose()?;
            let format = flag(&mut args, "--format")?.unwrap_or_else(|| "csv".to_string());
            let output = flag(&mut args, "--output")?;
            finish(&args, 1)?;
            if !matches!(format.as_str(), "csv" | "json") {
                return Err(format!("--format '{format}' is not csv or json"));
            }
            let path = args.first().ok_or("measure needs an image")?;
            let cfg = config();
            let replace = replace || cfg.replace_output;
            let mm = mm_per_px.unwrap_or(cfg.mm_per_px);
            if mm <= 0.0 {
                return Err(
                    "measure needs the scale: --mm-per-px, or mm_per_px in vision.cfg (the command calibrate works it out)"
                        .into(),
                );
            }
            s.threshold = cfg.threshold;
            s.min_area_px = cfg.part_min_area;
            s.max_area_px = cfg.part_max_area;
            s.max_parts = cfg.max_parts;
            let img = picture(path, &cfg, flatten_px)?;
            let scale = Scale { mm_per_px: mm };
            let parts = measure_parts(&img, &s, scale)?;
            let text = if format == "json" {
                to_json(&parts, img.w, img.h, scale)
            } else {
                to_csv(&parts)
            };
            deliver(&text, output, replace)
        }
        "correlate" => {
            let cfg = config();
            let mut s = cfg.correlate_settings();
            if switch(&mut args, "--light") {
                s.polarity = Polarity::Light;
            }
            if let Some(v) = flag(&mut args, "--diameter")? {
                s.diameter = num(&v, "--diameter")?;
            }
            if let Some(v) = flag(&mut args, "--min-score")? {
                s.min_score = num(&v, "--min-score")?;
            }
            let flatten_px = flag(&mut args, "--flatten")?
                .map(|v| count(&v, "--flatten"))
                .transpose()?;
            finish(&args, 1)?;
            let path = args.first().ok_or("correlate needs an image")?;
            let img = picture(path, &cfg, flatten_px)?;
            let found = correlate(&img, &s)?;
            print!("{}", correlate_csv(&found));
            eprintln!(
                "{} marks of {} pixels with a correlation of {} or more",
                found.len(),
                s.diameter,
                s.min_score
            );
            Ok(())
        }
        "rectify" => {
            let replace = switch(&mut args, "--replace");
            let cfg = config();
            let corners = parse_corners(
                &flag(&mut args, "--corners")?
                    .ok_or("rectify needs --corners X1,Y1,X2,Y2,X3,Y3,X4,Y4")?,
            )?;
            let size = flag(&mut args, "--size")?
                .map(|v| {
                    let (w, h) = v
                        .split_once('x')
                        .ok_or_else(|| format!("--size '{v}' is not WIDTHxHEIGHT"))?;
                    Ok::<_, String>((count(w, "--size")?, count(h, "--size")?))
                })
                .transpose()?;
            let fill = flag(&mut args, "--fill")?
                .map(|v| {
                    v.trim()
                        .parse::<u8>()
                        .map_err(|_| format!("--fill '{v}' is not from 0 to 255"))
                })
                .transpose()?
                .unwrap_or(0);
            let output = flag(&mut args, "--output")?.ok_or("rectify needs --output FILE.pgm")?;
            finish(&args, 1)?;
            let path = args.first().ok_or("rectify needs an image")?;
            let img = picture(path, &cfg, Some(0))?;
            let out = rectify(&img, &corners, size, fill, cfg.rectify_max_side)?;
            write_bytes(
                Path::new(&output),
                &to_pgm(&out),
                replace || cfg.replace_output,
            )?;
            eprintln!("{} by {} pixels written to {output}", out.w, out.h);
            Ok(())
        }
        "pads" => {
            let replace = switch(&mut args, "--replace");
            let cfg = config();
            let format = flag(&mut args, "--format")?.unwrap_or_else(|| "csv".to_string());
            let output = flag(&mut args, "--output")?;
            if !matches!(format.as_str(), "csv" | "json") {
                return Err(format!("--format '{format}' is not csv or json"));
            }
            let (text, passes, note) = pads_run(&mut args, &cfg, format == "json")?;
            deliver(&text, output, replace || cfg.replace_output)?;
            eprintln!("{note}");
            if passes {
                Ok(())
            } else {
                Err("the row of pads is not good".into())
            }
        }
        "diff" => {
            let replace = switch(&mut args, "--replace");
            let cfg = config();
            let format = flag(&mut args, "--format")?.unwrap_or_else(|| "csv".to_string());
            let output = flag(&mut args, "--output")?;
            if !matches!(format.as_str(), "csv" | "json") {
                return Err(format!("--format '{format}' is not csv or json"));
            }
            let replace = replace || cfg.replace_output;
            let (text, same, note) = diff_run(&mut args, &cfg, format == "json", replace)?;
            deliver(&text, output, replace)?;
            eprintln!("{note}");
            if same {
                Ok(())
            } else {
                Err("the pictures are not the same".into())
            }
        }
        "server" => {
            let replace = switch(&mut args, "--replace");
            let cfg = config();
            let settings = server_settings(&mut args, &cfg)?;
            let output = flag(&mut args, "--output")?;
            if args.is_empty() {
                return Err("server needs health, config or send".into());
            }
            let sub = args.remove(0);
            let mut client = Client::tcp(settings).map_err(|e| describe(&e))?;
            match sub.as_str() {
                "health" => {
                    finish(&args, 0)?;
                    client.health().map_err(|e| describe(&e))?;
                    println!("ok");
                    Ok(())
                }
                "config" => {
                    finish(&args, 0)?;
                    let pairs = client.fetch_config().map_err(|e| describe(&e))?;
                    let (_, done) = apply_remote(&cfg, &pairs)?;
                    let taken: Vec<(String, String)> = pairs
                        .into_iter()
                        .filter(|p| done.applied.contains(&p.0))
                        .collect();
                    deliver(&config_lines(&taken), output, replace || cfg.replace_output)?;
                    eprintln!(
                        "{} values from the server, {} left out{}",
                        done.applied.len(),
                        done.ignored.len(),
                        if done.ignored.is_empty() {
                            String::new()
                        } else {
                            format!(" ({})", done.ignored.join(", "))
                        }
                    );
                    Ok(())
                }
                "send" => {
                    if args.is_empty() {
                        return Err("server send needs quality, offset, pads or diff".into());
                    }
                    let kind = args.remove(0);
                    let (json, passes) = match kind.as_str() {
                        "quality" => {
                            finish(&args, 1)?;
                            let path = args.first().ok_or("server send quality needs an image")?;
                            let img = picture(path, &cfg, Some(0))?;
                            let q = quality::measure(&img)?;
                            let why = quality::reasons(&q, &cfg.quality_settings())?;
                            (quality::to_json(&q, &why), why.is_empty())
                        }
                        "offset" => {
                            let opts = pick_options("offset", &mut args, &cfg)?;
                            finish(&args, 1)?;
                            let path = args.first().ok_or("server send offset needs an image")?;
                            let (mm, body) = pick_inputs("offset", &opts, &cfg)?;
                            let parts = picked_parts(path, &cfg, &opts, mm)?;
                            let held = pick_offset(&parts, &body, &opts.settings)?;
                            (offset_json(&held, &body), held.passes())
                        }
                        "pads" => {
                            let (text, passes, _) = pads_run(&mut args, &cfg, true)?;
                            (text, passes)
                        }
                        "diff" => {
                            let (text, same, _) = diff_run(&mut args, &cfg, true, replace)?;
                            (text, same)
                        }
                        other => {
                            return Err(format!(
                                "{other}: server send takes quality, offset, pads or diff"
                            ))
                        }
                    };
                    let id = client.send_result(&kind, &json).map_err(|e| describe(&e))?;
                    eprintln!(
                        "the {kind} result was sent{}",
                        id.map(|i| format!(" as {i}")).unwrap_or_default()
                    );
                    // a result that is refused is an error for a script, after it was sent
                    if passes {
                        Ok(())
                    } else {
                        Err(format!("the {kind} result is not good"))
                    }
                }
                other => Err(format!("{other}: server takes health, config or send")),
            }
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
