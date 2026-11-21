//! The window without a screen at its edges: what it says when a picture is missing, what it keeps when a file is refused, the fields of the edge and the check against a reference, and the
//! words of the status line in the eight languages.

use eh3d_vision_studio::config::Config;
use eh3d_vision_studio::image::Gray;
use eh3d_vision_studio::lang::{fill, Lang, Msg};
use eh3d_vision_studio::readings::{EDGE_LABELS, HISTOGRAM_BARS};
use eh3d_vision_studio::window::{Loaded, Status, Window};
use std::path::{Path, PathBuf};

fn window() -> Window {
    Window::new(Lang::En, Config::default(), None)
}

fn dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("product-window-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn pgm(w: usize, h: usize, pixel: impl Fn(usize, usize) -> u8) -> Vec<u8> {
    let mut bytes = format!("P5\n{w} {h}\n255\n").into_bytes();
    for y in 0..h {
        for x in 0..w {
            bytes.push(pixel(x, y));
        }
    }
    bytes
}

fn halves() -> Vec<u8> {
    pgm(64, 64, |x, _| if x < 32 { 20 } else { 220 })
}

fn gray_of(pixel: impl Fn(usize, usize) -> u8) -> Gray {
    let mut data = Vec::new();
    for y in 0..64 {
        for x in 0..64 {
            data.push(pixel(x, y));
        }
    }
    Gray::new(64, 64, data).unwrap()
}

fn loaded(name: &str, gray: Gray) -> Loaded {
    Loaded {
        name: name.to_string(),
        path: PathBuf::from(name),
        gray,
    }
}

fn opened(name: &str) -> (Window, PathBuf) {
    let d = dir(name);
    let path = d.join("halves.pgm");
    std::fs::write(&path, halves()).unwrap();
    let mut w = window();
    assert!(w.open_image(&path));
    (w, path)
}

// ---- no picture ----

#[test]
fn every_reading_asks_for_a_picture_first() {
    let mut w = window();
    assert!(!w.read_histogram());
    assert_eq!(w.status, Status::NeedImage);
    assert!(w.histogram_report().is_none());
    assert!(w.histogram_bars().is_none());
    assert!(w.histogram_of().is_none());
    w.status = Status::Ready;
    assert!(!w.measure_edge());
    assert_eq!(w.status, Status::NeedImage);
    w.status = Status::Ready;
    assert!(!w.check_reference());
    assert_eq!(w.status, Status::NeedImage);
}

#[test]
fn the_check_against_a_reference_asks_for_the_other_picture_second() {
    let (mut w, _) = opened("need-other");
    assert!(!w.check_reference());
    assert_eq!(w.status, Status::NeedOther);
    assert!(w.reference.is_none());
}

// ---- the fields of the edge ----

#[test]
fn a_field_of_the_edge_that_is_not_there_is_left_alone() {
    let mut w = window();
    let before = w.edge_line.clone();
    w.set_edge_field(4, "5".to_string());
    w.set_edge_field(usize::MAX, "5".to_string());
    assert_eq!(w.edge_line, before);
    w.set_edge_field(3, "7.5".to_string());
    assert_eq!(w.edge_line[3], "7.5");
}

#[test]
fn the_four_labels_of_the_edge_are_distinct_and_exist_in_every_language() {
    for (i, a) in EDGE_LABELS.iter().enumerate() {
        for b in &EDGE_LABELS[i + 1..] {
            assert_ne!(a, b);
        }
        for lang in Lang::ALL {
            assert!(!a.tr(lang).is_empty());
        }
    }
}

#[test]
fn an_edge_field_that_is_not_a_finite_number_is_named_and_stops_the_measure() {
    let (mut w, _) = opened("bad-edge");
    for (k, label) in EDGE_LABELS.iter().enumerate() {
        for bad in ["", "abc", "NaN", "inf", "-inf", "1,5"] {
            let mut fields = ["10", "32", "50", "32"].map(str::to_string);
            fields[k] = bad.to_string();
            for (i, f) in fields.iter().enumerate() {
                w.set_edge_field(i, f.clone());
            }
            assert!(!w.measure_edge(), "{k} {bad:?}");
            assert_eq!(w.status, Status::BadValue(*label), "{k} {bad:?}");
            assert!(w.edge.is_none());
        }
    }
}

#[test]
fn typing_in_a_field_forgets_the_edge_that_was_measured() {
    let (mut w, _) = opened("forget-edge");
    for (i, v) in ["10", "32", "54", "32"].iter().enumerate() {
        w.set_edge_field(i, v.to_string());
    }
    if w.measure_edge() {
        assert!(w.edge.is_some());
        w.set_edge_field(0, "11".to_string());
        assert!(w.edge.is_none());
    } else {
        assert!(w.edge.is_none());
    }
}

// ---- the histogram ----

#[test]
fn the_histogram_of_a_picture_in_two_halves_counts_every_pixel() {
    let (mut w, _) = opened("histogram");
    assert!(w.read_histogram());
    assert!(matches!(w.status, Status::Histogram { .. }));
    assert!(w.histogram_report().is_some());
    assert!(w.histogram_of().is_some());
    if let Some(bars) = w.histogram_bars() {
        assert_eq!(bars.len(), HISTOGRAM_BARS);
        assert_eq!(bars.iter().sum::<u64>(), 64 * 64);
    }
}

// ---- opening pictures ----

#[test]
fn opening_a_picture_raises_the_serial_and_says_its_name_and_size() {
    let (mut w, path) = opened("serial");
    assert_eq!(w.image_serial, 1);
    assert_eq!(
        w.status,
        Status::Opened {
            name: "halves.pgm".to_string(),
            w: 64,
            h: 64
        }
    );
    assert!(w.open_image(&path));
    assert_eq!(w.image_serial, 2);
}

#[test]
fn a_file_that_is_not_a_picture_is_refused_and_the_picture_in_use_stays() {
    let (mut w, _) = opened("refused");
    let text = dir("refused").join("text.pgm");
    std::fs::write(&text, b"this is not a picture").unwrap();
    assert!(!w.open_image(&text));
    assert!(matches!(&w.status, Status::Failed(m) if m.starts_with("text.pgm")));
    assert_eq!(w.image_serial, 1);
    assert!(w.image.is_some());
    assert!(!w.open_image(&dir("refused").join("missing.pgm")));
    assert!(matches!(w.status, Status::Failed(_)));
    assert_eq!(w.image_serial, 1);
}

#[test]
fn a_file_over_the_limit_in_megabytes_is_refused_by_its_size_and_one_at_the_limit_is_read() {
    let d = dir("limit");
    let mut w = window();
    w.cfg.max_file_mib = 1;
    let at = d.join("at.bin");
    std::fs::write(&at, vec![0u8; 1_048_576]).unwrap();
    assert!(!w.open_image(&at));
    assert!(matches!(w.status, Status::Failed(_)));
    let over = d.join("over.bin");
    std::fs::write(&over, vec![0u8; 1_048_577]).unwrap();
    assert!(!w.open_image(&over));
    assert_eq!(
        w.status,
        Status::TooLarge {
            name: "over.bin".to_string(),
            mib: 1
        }
    );
    assert!(w.image.is_none());
}

// ---- the mask ----

#[test]
fn a_mask_is_loaded_by_its_name_and_clearing_it_forgets_the_check() {
    let d = dir("mask");
    let path = d.join("mask.pgm");
    std::fs::write(&path, pgm(64, 64, |_, _| 255)).unwrap();
    let mut w = window();
    assert!(w.open_mask(&path));
    assert_eq!(w.status, Status::MaskLoaded("mask.pgm".to_string()));
    assert!(w.mask.is_some());
    w.clear_mask();
    assert!(w.mask.is_none() && w.reference.is_none());
}

#[test]
fn a_mask_that_is_not_a_picture_leaves_the_old_mask_where_it_was() {
    let d = dir("mask-bad");
    let good = d.join("good.pgm");
    std::fs::write(&good, pgm(64, 64, |_, _| 255)).unwrap();
    let bad = d.join("bad.pgm");
    std::fs::write(&bad, b"nope").unwrap();
    let mut w = window();
    assert!(w.open_mask(&good));
    assert!(!w.open_mask(&bad));
    assert!(matches!(&w.status, Status::Failed(m) if m.starts_with("bad.pgm")));
    assert_eq!(w.mask.as_ref().map(|m| m.name.as_str()), Some("good.pgm"));
    assert!(!w.open_mask(&d.join("none.pgm")));
}

// ---- the reference ----

#[test]
fn a_picture_checked_against_itself_is_the_same() {
    let (mut w, _) = opened("same");
    w.other = Some(loaded(
        "again.pgm",
        gray_of(|x, _| if x < 32 { 20 } else { 220 }),
    ));
    assert!(w.check_reference());
    assert!(matches!(w.status, Status::ReferenceSame { .. }));
    assert!(w.reference.is_some());
}

#[test]
fn a_picture_checked_against_its_opposite_is_different() {
    let (mut w, _) = opened("different");
    w.other = Some(loaded(
        "opposite.pgm",
        gray_of(|x, _| if x < 32 { 220 } else { 20 }),
    ));
    assert!(w.check_reference());
    assert!(matches!(w.status, Status::ReferenceDifferent { .. }));
}

// ---- the words ----

#[test]
fn every_state_without_numbers_has_words_in_the_eight_languages() {
    let states = [
        Status::Ready,
        Status::NeedImage,
        Status::NeedOther,
        Status::NeedMark,
        Status::NeedScale,
        Status::NeedDesign,
        Status::NeedAccepted,
        Status::NeedPitch,
        Status::BatchCancelled,
        Status::BatchBusy,
        Status::SameFile,
        Status::NeedBody,
    ];
    for s in &states {
        for lang in Lang::ALL {
            assert!(!s.text(lang).is_empty(), "{s:?} {lang:?}");
        }
        assert_ne!(s.text(Lang::En), s.text(Lang::Ja), "{s:?}");
        assert_ne!(s.text(Lang::En), s.text(Lang::Zh), "{s:?}");
    }
}

#[test]
fn the_numbers_and_the_names_reach_the_text_in_every_language() {
    for lang in Lang::ALL {
        let t = Status::TooLarge {
            name: "big.png".to_string(),
            mib: 77,
        }
        .text(lang);
        assert!(t.contains("big.png") && t.contains("77"), "{lang:?} {t}");
        let t = Status::Opened {
            name: "x.png".to_string(),
            w: 640,
            h: 480,
        }
        .text(lang);
        assert!(
            t.contains("x.png") && t.contains("640") && t.contains("480"),
            "{lang:?} {t}"
        );
        let t = Status::Failed("boom".to_string()).text(lang);
        assert!(t.contains("boom"), "{lang:?} {t}");
        assert!(!Status::BadValue(Msg::EdgeFromX).text(lang).is_empty());
    }
}

#[test]
fn the_codes_of_the_languages_are_read_and_an_unknown_one_is_english() {
    let codes = ["en", "es", "de", "fr", "it", "ja", "zh", "pt"];
    for (code, lang) in codes.iter().zip(Lang::ALL) {
        assert_eq!(Lang::from_code(code), Some(lang), "{code}");
        assert_eq!(Lang::from_setting(code), lang, "{code}");
    }
    assert_eq!(Lang::from_setting("xx"), Lang::En);
    assert_eq!(Lang::from_setting(""), Lang::En);
    assert_eq!(Lang::from_code("xx"), None);
}

#[test]
fn the_places_of_a_text_are_filled_in_order_and_the_missing_ones_stay() {
    assert_eq!(fill("{0} and {1}", &["a", "b"]), "a and b");
    assert_eq!(fill("{0}{0}", &["ab"]), "abab");
    assert_eq!(fill("{0} {1}", &["a"]), "a {1}");
    assert_eq!(fill("no places", &["a"]), "no places");
    assert_eq!(fill("{1}", &["a", "b"]), "b");
    assert_eq!(fill("", &["a"]), "");
}
