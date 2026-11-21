// EH3D-VISION-STUDIO - the commands that train a mark, find it, pair the marks for any turn, check the light, find a tape, print a card and read a frame.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//
// Every command here ends in one of three ways: 0 (done), 1 (the command could not be carried out: a wrong option, a file that cannot be read or written, a limit passed) and 2
// (the command was carried out and the check it makes was not passed: a mark that looks like itself, marks that cannot be told apart, a light that is not good). Text goes to the
// output in the language of `language` of vision.cfg (or `--lang`), JSON (`--json` or `--format json`) has keys that do not change, and a file that is written is never written over a
// file that exists unless `--replace` (or `--overwrite`) says so.

use crate::cli_args::{finish, flag, num, switch};
use crate::{config, count, deliver, picture, points};
use eh3d_vision_studio::align::pair_nearest;
use eh3d_vision_studio::dotcard;
use eh3d_vision_studio::fiducial::Polarity;
use eh3d_vision_studio::lang::{Lang, Msg};
use eh3d_vision_studio::light;
use eh3d_vision_studio::limit::{read_limited, write_bytes, write_text};
use eh3d_vision_studio::mjpeg;
use eh3d_vision_studio::sim::to_pgm;
use eh3d_vision_studio::tape;
use eh3d_vision_studio::train::{
    self, ambiguity, locate, train as train_mark, Rect, Template, MAX_FILE,
};
use eh3d_vision_studio::turnpair::{pair_turned, Pairing, PAIR_AMBIGUOUS, PAIR_FEW, PAIR_NONE};
use std::path::Path;

/// The commands of this file.
pub const COMMANDS: [&str; 7] = [
    "train",
    "locate",
    "pair",
    "light-check",
    "tape-locate",
    "calib-card",
    "jpeg",
];

/// The code at the start of a refusal (`pair_none: ...` gives `pair_none`).
fn code_of(error: &str) -> &str {
    error.split(':').next().unwrap_or(error)
}

/// Text in quotes for JSON, with what must be escaped.
fn js(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The language of the text: `--lang` or `language` of the configuration file.
fn language(
    args: &mut Vec<String>,
    cfg: &eh3d_vision_studio::config::Config,
) -> Result<Lang, String> {
    match flag(args, "--lang")? {
        Some(code) => Lang::from_code(&code)
            .ok_or_else(|| format!("--lang '{code}' is not one of en, es, de, fr, it, ja, zh, pt")),
        None => Ok(Lang::from_setting(&cfg.language)),
    }
}

/// How the answer is written: `--json`, or `--format text|json` (and `csv` where the command has it).
fn format_of(args: &mut Vec<String>, allow_csv: bool) -> Result<&'static str, String> {
    let json = switch(args, "--json");
    let named = flag(args, "--format")?;
    let format = match named.as_deref() {
        None => None,
        Some("text") => Some("text"),
        Some("json") => Some("json"),
        Some("csv") if allow_csv => Some("csv"),
        Some(other) => {
            return Err(format!(
                "--format '{other}' is not {}",
                if allow_csv {
                    "text, json or csv"
                } else {
                    "text or json"
                }
            ))
        }
    };
    match (json, format) {
        (true, Some(f)) if f != "json" => Err("--json and --format disagree".into()),
        (true, _) => Ok("json"),
        (false, Some(f)) => Ok(f),
        (false, None) => Ok("text"),
    }
}

/// Whether a written file may replace one that exists: `--replace` or `--overwrite`; `--create` says the opposite (a new file only, which is the default) and cannot be given with them.
fn replace_of(args: &mut Vec<String>) -> Result<bool, String> {
    let create = switch(args, "--create");
    let replace = switch(args, "--replace");
    let overwrite = switch(args, "--overwrite");
    if create && (replace || overwrite) {
        return Err("--create and --replace (--overwrite) cannot be given together".into());
    }
    Ok(replace || overwrite)
}

fn rect(text: &str) -> Result<Rect, String> {
    let parts: Vec<&str> = text.split(',').collect();
    let n = |i: usize| -> Result<usize, String> {
        let v: usize = parts
            .get(i)
            .and_then(|t| t.trim().parse().ok())
            .ok_or_else(|| format!("--rect '{text}' is not X,Y,WIDTH,HEIGHT in whole pixels"))?;
        if v > 1_000_000 {
            return Err(format!("--rect '{text}' has a number above 1000000"));
        }
        Ok(v)
    };
    if parts.len() != 4 {
        return Err(format!(
            "--rect '{text}' is not X,Y,WIDTH,HEIGHT in whole pixels"
        ));
    }
    Ok(Rect {
        x: n(0)?,
        y: n(1)?,
        w: n(2)?,
        h: n(3)?,
    })
}

/// Whether a refusal of the training or the search is a check that was not passed (exit 2) and not a command that could not be carried out (exit 1).
fn train_verification(code: &str) -> bool {
    [
        train::SAMPLES_DISAGREE,
        train::MARK_AMBIGUOUS,
        train::SAMPLE_FLAT,
        train::TOO_MANY,
    ]
    .contains(&code)
}

/// Says that a check was not passed, in text on the error output or as JSON on the output, and gives the code 2.
fn not_passed(command: &str, error: &str, json: bool, lang: Lang) -> Result<u8, String> {
    if json {
        println!(
            "{{\"command\":{},\"ok\":false,\"code\":{},\"detail\":{}}}",
            js(command),
            js(code_of(error)),
            js(error)
        );
    } else {
        eprintln!("{}", Msg::VerifyFailed.with(lang, &[error]));
    }
    Ok(2)
}

/// Runs one of the commands of this file; the exit code (0 or 2), or the error that gives the exit code 1.
pub fn run(cmd: &str, mut args: Vec<String>) -> Result<u8, String> {
    match cmd {
        "train" => train_run(&mut args),
        "locate" => locate_run(&mut args),
        "pair" => pair_run(&mut args),
        "light-check" => light_run(&mut args),
        "tape-locate" => tape_run(&mut args),
        "calib-card" => card_run(&mut args),
        "jpeg" => jpeg_run(&mut args),
        other => Err(format!("{other}: unknown command (see help)")),
    }
}

fn train_run(args: &mut Vec<String>) -> Result<u8, String> {
    let cfg = config();
    let lang = language(args, &cfg)?;
    let format = format_of(args, false)?;
    let output = flag(args, "--output")?;
    let replace = replace_of(args)?;
    let mut rects: Vec<Rect> = Vec::new();
    while let Some(v) = flag(args, "--rect")? {
        rects.push(rect(&v)?);
        if rects.len() > train::MAX_SAMPLES {
            return Err(format!(
                "{}: more than {} examples",
                train::SAMPLE_COUNT,
                train::MAX_SAMPLES
            ));
        }
    }
    let flatten_px = flag(args, "--flatten")?
        .map(|v| count(&v, "--flatten"))
        .transpose()?;
    finish(args, 1)?;
    let path = args.first().ok_or("train needs an image")?.clone();
    let out_path = output.ok_or("train needs --output FILE (the file of the mark)")?;
    let img = picture(&path, &cfg, flatten_px)?;
    let settings = cfg.train_settings();
    let template = match train_mark(&img, &rects, &settings) {
        Ok(t) => t,
        Err(e) if train_verification(code_of(&e)) => {
            return not_passed("train", &e, format == "json", lang);
        }
        Err(e) => return Err(e),
    };
    // a file of a mark that exists is never written over unless it was asked for
    write_bytes(Path::new(&out_path), &template.to_bytes(), replace)?;
    if format == "json" {
        println!(
            "{{\"command\":\"train\",\"ok\":true,\"width\":{},\"height\":{},\"samples\":{},\"output\":{}}}",
            template.w,
            template.h,
            template.samples,
            js(&out_path)
        );
    } else {
        println!(
            "{}",
            Msg::TrainWrote.with(
                lang,
                &[
                    out_path,
                    template.w.to_string(),
                    template.h.to_string(),
                    template.samples.to_string()
                ]
            )
        );
    }
    Ok(0)
}

fn locate_run(args: &mut Vec<String>) -> Result<u8, String> {
    let cfg = config();
    let lang = language(args, &cfg)?;
    let format = format_of(args, false)?;
    let json = format == "json";
    let mark =
        flag(args, "--mark")?.ok_or("locate needs --mark FILE (the file of a trained mark)")?;
    let mut settings = cfg.train_settings();
    if let Some(v) = flag(args, "--turn")? {
        settings.turn_max_deg = num(&v, "--turn")?;
    }
    if let Some(v) = flag(args, "--min-score")? {
        settings.min_score = num(&v, "--min-score")?;
    }
    let flatten_px = flag(args, "--flatten")?
        .map(|v| count(&v, "--flatten"))
        .transpose()?;
    finish(args, 1)?;
    let path = args.first().ok_or("locate needs an image")?.clone();
    // a file of a mark is small: it is read within 1 MiB, then within its own limit
    let bytes = read_limited(Path::new(&mark), 1)?;
    if bytes.len() > MAX_FILE {
        return Err(format!(
            "{}: the file is larger than the limit",
            train::BAD_FILE
        ));
    }
    let template = Template::from_bytes(&bytes)?;
    let amb = ambiguity(&template.data, template.w, template.h);
    if amb > settings.max_ambiguity {
        let e = format!(
            "{}: the mark looks like itself {amb:.2} when it is moved a few pixels, more than the {} that is allowed",
            train::MARK_AMBIGUOUS,
            settings.max_ambiguity
        );
        return not_passed("locate", &e, json, lang);
    }
    let img = picture(&path, &cfg, flatten_px)?;
    let found = match locate(&img, &template, &settings) {
        Ok(f) => f,
        Err(e) if train_verification(code_of(&e)) => return not_passed("locate", &e, json, lang),
        Err(e) => return Err(e),
    };
    if json {
        let rows: Vec<String> = found
            .iter()
            .map(|f| {
                format!(
                    "{{\"x\":{:.3},\"y\":{:.3},\"angle_deg\":{:.2},\"score\":{:.4}}}",
                    f.x, f.y, f.angle_deg, f.score
                )
            })
            .collect();
        println!(
            "{{\"command\":\"locate\",\"ok\":{},\"count\":{},\"found\":[{}]}}",
            !found.is_empty(),
            found.len(),
            rows.join(",")
        );
    } else if found.is_empty() {
        println!("{}", Msg::LocateNone.tr(lang));
    } else {
        println!(
            "{}",
            Msg::LocateCount.with(lang, &[found.len().to_string()])
        );
        for (i, f) in found.iter().enumerate() {
            println!(
                "{}: ({:.3}, {:.3}) px, angle {:.2} degrees, score {:.3}",
                i + 1,
                f.x,
                f.y,
                f.angle_deg,
                f.score
            );
        }
    }
    if found.is_empty() {
        if !json {
            eprintln!(
                "{}",
                Msg::VerifyFailed.with(lang, &[Msg::LocateNone.tr(lang)])
            );
        }
        return Ok(2);
    }
    Ok(0)
}

fn pair_run(args: &mut Vec<String>) -> Result<u8, String> {
    let cfg = config();
    let lang = language(args, &cfg)?;
    let format = format_of(args, false)?;
    let json = format == "json";
    let any_turn = switch(args, "--any-turn");
    let mut settings = cfg.pair_settings();
    if let Some(v) = flag(args, "--max-turn")? {
        settings.max_turn_deg = num(&v, "--max-turn")?;
    }
    if let Some(v) = flag(args, "--tolerance")? {
        settings.tolerance_mm = num(&v, "--tolerance")?;
    }
    if let Some(v) = flag(args, "--min-marks")? {
        settings.min_marks = count(&v, "--min-marks")?;
    }
    finish(args, 2)?;
    if args.len() < 2 {
        return Err("pair needs the design points and the measured points".into());
    }
    let design = points(&args[0])?;
    let measured = points(&args[1])?;
    let mode = if any_turn { "any-turn" } else { "nearest" };
    let found: Pairing = if any_turn {
        match pair_turned(&design, &measured, &settings) {
            Ok(p) => p,
            Err(e) if [PAIR_AMBIGUOUS, PAIR_NONE, PAIR_FEW].contains(&code_of(&e)) => {
                return not_passed("pair", &e, json, lang);
            }
            Err(e) => return Err(e),
        }
    } else {
        let ordered = pair_nearest(&design, &measured)?;
        let mut used = vec![false; measured.len()];
        let pairs: Vec<(usize, usize)> = ordered
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                let j = measured
                    .iter()
                    .enumerate()
                    .position(|(j, m)| !used[j] && m == p)?;
                used[j] = true;
                Some((i, j))
            })
            .collect();
        Pairing {
            pairs,
            missing: Vec::new(),
            extra: Vec::new(),
            turn_deg: 0.0,
            dx: 0.0,
            dy: 0.0,
            rms_mm: 0.0,
        }
    };
    let Pairing {
        pairs,
        missing,
        extra,
        turn_deg: turn,
        dx,
        dy,
        rms_mm: rms,
    } = found;
    if json {
        let list = |v: &[usize]| {
            v.iter()
                .map(|i| (i + 1).to_string())
                .collect::<Vec<_>>()
                .join(",")
        };
        let rows: Vec<String> = pairs
            .iter()
            .map(|&(i, j)| format!("[{},{}]", i + 1, j + 1))
            .collect();
        println!(
            "{{\"command\":\"pair\",\"ok\":true,\"mode\":{},\"paired\":[{}],\"missing\":[{}],\"extra\":[{}],\"turn_deg\":{:.4},\"dx_mm\":{:.4},\"dy_mm\":{:.4},\"rms_mm\":{:.4}}}",
            js(mode),
            rows.join(","),
            list(&missing),
            list(&extra),
            turn,
            dx,
            dy,
            rms
        );
        return Ok(0);
    }
    if any_turn {
        println!(
            "{}",
            Msg::PairResult.with(
                lang,
                &[
                    pairs.len().to_string(),
                    design.len().to_string(),
                    format!("{turn:+.3}"),
                    format!("{dx:+.4}"),
                    format!("{dy:+.4}"),
                    format!("{rms:.4}")
                ]
            )
        );
    }
    for (i, j) in &pairs {
        println!("design {} -> mark {}", i + 1, j + 1);
    }
    let list = |v: &[usize]| {
        v.iter()
            .map(|i| (i + 1).to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    if !missing.is_empty() {
        println!("{}", Msg::PairMissing.with(lang, &[list(&missing)]));
    }
    if !extra.is_empty() {
        println!("{}", Msg::PairExtra.with(lang, &[list(&extra)]));
    }
    Ok(0)
}

fn reason_text(code: &str, lang: Lang) -> &'static str {
    match code {
        light::SATURATED => Msg::LightWhySaturated.tr(lang),
        light::DIM => Msg::LightWhyDim.tr(lang),
        light::BRIGHT => Msg::LightWhyBright.tr(lang),
        light::UNEVEN => Msg::LightWhyUneven.tr(lang),
        light::TILTED => Msg::LightWhyTilted.tr(lang),
        tape::FEW_HOLES => Msg::TapeWhyFew.tr(lang),
        tape::HOLE_MISSING => Msg::TapeWhyMissing.tr(lang),
        tape::OFF_LINE => Msg::TapeWhyOffLine.tr(lang),
        tape::PITCH => Msg::TapeWhyPitch.tr(lang),
        tape::HOLE_SIZE => Msg::TapeWhySize.tr(lang),
        _ => "",
    }
}

fn light_run(args: &mut Vec<String>) -> Result<u8, String> {
    let cfg = config();
    let lang = language(args, &cfg)?;
    let format = format_of(args, true)?;
    let output = flag(args, "--output")?;
    let replace = replace_of(args)? || cfg.replace_output;
    let mut s = cfg.light_settings();
    if let Some(v) = flag(args, "--cells")? {
        s.cells = count(&v, "--cells")?;
    }
    if let Some(v) = flag(args, "--max-saturated")? {
        s.max_saturated = num(&v, "--max-saturated")?;
    }
    if let Some(v) = flag(args, "--max-uneven")? {
        s.max_uneven = num(&v, "--max-uneven")?;
    }
    if let Some(v) = flag(args, "--max-tilt")? {
        s.max_tilt = num(&v, "--max-tilt")?;
    }
    finish(args, 1)?;
    let path = args.first().ok_or("light-check needs an image")?;
    // the light is judged on the picture as it is: nothing flattens it first
    let img = picture(path, &cfg, Some(0))?;
    let r = light::analyse(&img, &s)?;
    let text = match format {
        "json" => light::to_json(&r),
        "csv" => light::to_csv(&r),
        _ => {
            let mut t = format!(
                "{}\n",
                Msg::LightSummary.with(
                    lang,
                    &[
                        format!("{:.1}", r.mean),
                        r.min.to_string(),
                        r.max.to_string(),
                        format!("{:.2}", r.saturated * 100.0),
                        format!("{:.1}", r.uneven * 100.0),
                        format!("{:+.1}", r.tilt_x * 100.0),
                        format!("{:+.1}", r.tilt_y * 100.0)
                    ]
                )
            );
            if r.good() {
                t.push_str(Msg::LightGood.tr(lang));
                t.push('\n');
            } else {
                for c in &r.reasons {
                    t.push_str(&format!("{c}: {}\n", reason_text(c, lang)));
                }
            }
            t
        }
    };
    deliver(&text, output, replace)?;
    if !r.good() {
        if format == "text" {
            eprintln!("{}", Msg::VerifyFailed.with(lang, &[r.reasons.join(", ")]));
        }
        return Ok(2);
    }
    Ok(0)
}

fn tape_run(args: &mut Vec<String>) -> Result<u8, String> {
    let cfg = config();
    let lang = language(args, &cfg)?;
    let format = format_of(args, false)?;
    let output = flag(args, "--output")?;
    let replace = replace_of(args)? || cfg.replace_output;
    let mut s = cfg.tape_settings();
    if switch(args, "--light") {
        s.polarity = Polarity::Light;
    }
    let scale = match flag(args, "--mm-per-px")? {
        Some(v) => num(&v, "--mm-per-px")?,
        None => cfg.mm_per_px,
    };
    if let Some(v) = flag(args, "--pitch-mm")? {
        s.pitch_mm = num(&v, "--pitch-mm")?;
    }
    if let Some(v) = flag(args, "--hole-mm")? {
        s.hole_mm = num(&v, "--hole-mm")?;
    }
    finish(args, 1)?;
    let path = args.first().ok_or("tape-locate needs an image")?;
    let img = picture(path, &cfg, Some(0))?;
    let r = tape::locate(&img, scale, &s)?;
    let text = if format == "json" {
        tape::to_json(&r)
    } else {
        let mut t = format!(
            "{}\n",
            Msg::TapeSummary.with(
                lang,
                &[
                    r.holes.len().to_string(),
                    format!("{:.3}", r.pitch_mm),
                    format!("{:+.3}", r.pitch_error_mm),
                    format!("{:.3}", r.line_error_mm),
                    format!("{:+.2}", r.angle_deg)
                ]
            )
        );
        for (i, h) in r.holes.iter().enumerate() {
            t.push_str(&format!(
                "hole {}: ({:.2}, {:.2}) px, {:.3} mm\n",
                i + 1,
                h.x,
                h.y,
                h.diameter_mm
            ));
        }
        for (i, p) in r.pockets.iter().enumerate() {
            t.push_str(&format!("pocket {}: ({:.2}, {:.2}) px\n", i + 1, p.0, p.1));
        }
        if let Some(i) = r.pick {
            let p = r.pockets[i];
            t.push_str(&format!(
                "{}\n",
                Msg::TapePick.with(
                    lang,
                    &[
                        (i + 1).to_string(),
                        format!("{:.2}", p.0),
                        format!("{:.2}", p.1)
                    ]
                )
            ));
        }
        if r.good() {
            t.push_str(Msg::TapeGood.tr(lang));
            t.push('\n');
        } else {
            for c in &r.reasons {
                t.push_str(&format!("{c}: {}\n", reason_text(c, lang)));
            }
        }
        t
    };
    deliver(&text, output, replace)?;
    if !r.good() {
        if format == "text" {
            eprintln!("{}", Msg::VerifyFailed.with(lang, &[r.reasons.join(", ")]));
        }
        return Ok(2);
    }
    Ok(0)
}

fn card_run(args: &mut Vec<String>) -> Result<u8, String> {
    let cfg = config();
    let lang = language(args, &cfg)?;
    let output = flag(args, "--output")?;
    let replace = replace_of(args)? || cfg.replace_output;
    let mut s = cfg.card_settings();
    if let Some(v) = flag(args, "--columns")? {
        s.columns = count(&v, "--columns")?;
    }
    if let Some(v) = flag(args, "--rows")? {
        s.rows = count(&v, "--rows")?;
    }
    if let Some(v) = flag(args, "--pitch-mm")? {
        s.pitch_mm = num(&v, "--pitch-mm")?;
    }
    finish(args, 0)?;
    let svg = dotcard::svg(&s, lang)?;
    match output {
        Some(path) => write_text(Path::new(&path), &svg, replace)?,
        None => print!("{svg}"),
    }
    Ok(0)
}

fn jpeg_run(args: &mut Vec<String>) -> Result<u8, String> {
    let cfg = config();
    let lang = language(args, &cfg)?;
    let format = format_of(args, false)?;
    let output = flag(args, "--output")?;
    let replace = replace_of(args)? || cfg.replace_output;
    finish(args, 1)?;
    let path = args.first().ok_or("jpeg needs a frame")?;
    let s = cfg.mjpeg_settings();
    // the file is read within the limit of bytes of the reader, which is smaller than max_file_mib unless that is set smaller
    let bytes = read_limited(Path::new(path), cfg.max_file_mib)?;
    let img = mjpeg::decode(&bytes, &s)?;
    if let Some(out) = output {
        write_bytes(Path::new(&out), &to_pgm(&img), replace)?;
    }
    let mean = img.data.iter().map(|&v| f64::from(v)).sum::<f64>() / img.data.len() as f64;
    if format == "json" {
        println!(
            "{{\"command\":\"jpeg\",\"ok\":true,\"width\":{},\"height\":{},\"mean\":{:.3}}}",
            img.w, img.h, mean
        );
    } else {
        println!(
            "{}",
            Msg::JpegRead.with(
                lang,
                &[img.w.to_string(), img.h.to_string(), format!("{mean:.1}")]
            )
        );
    }
    Ok(0)
}
