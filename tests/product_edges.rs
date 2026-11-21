//! Edge cases of the parts that read text and numbers: the configuration file, what the server keeps and sends, the texts of the window, the files of points and parts, the table of
//! packages and its verdicts, the fit of a board, the scale, the zoom of the picture and the plan of the pictures of a big part. Nothing here needs a camera, a screen or a network.

use eh3d_vision_studio::align::{accept, fit, isocam_args, pair_nearest, Transform};
use eh3d_vision_studio::batch::{csv_field, list_pictures, TOO_MANY_FILES};
use eh3d_vision_studio::config::{Config, KEYS};
use eh3d_vision_studio::inspect::{
    find_package, judge, parse_table, summary, to_csv as inspect_csv, to_json as inspect_json,
    Package, MAX_PACKAGES, REASON_ANGLE, REASON_LENGTH, REASON_WIDTH,
};
use eh3d_vision_studio::lang::{fill, Lang};
use eh3d_vision_studio::limit::{
    read_limited, read_text_limited, write_bytes, write_text, NOT_TEXT, OUTPUT_EXISTS, TOO_LARGE,
    UNREADABLE, UNWRITABLE,
};
use eh3d_vision_studio::measure::{fixed, parse_csv, to_csv as parts_csv, Part, CSV_HEADER};
use eh3d_vision_studio::points;
use eh3d_vision_studio::report::escape;
use eh3d_vision_studio::scale::{fit_k1, from_pitch, undistort, Scale, MAX_PITCH_MARKS};
use eh3d_vision_studio::server::{
    apply_remote, base64_decode, base64_encode, config_lines, config_name, link_settings,
    parse_config_text, parse_object, plain_token_warning, safe_segment, API, APP, CONFIG_FORMAT,
    RESULT_FORMAT,
};
use eh3d_vision_studio::shots::{plan, to_csv as shots_csv, CSV_HEADER as SHOTS_HEADER, MAX_SHOTS};
use eh3d_vision_studio::viewport::{Viewport, MAX_ZOOM, MIN_ZOOM};
use std::path::{Path, PathBuf};

/// A small seeded generator, so every run sees the same numbers.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = (self.0 * 1_103_515_245 + 12_345) % 2_147_483_648;
        self.0 >> 4
    }

    fn unit(&mut self) -> f64 {
        (self.next() % 1_000_000) as f64 / 1_000_000.0
    }

    fn byte(&mut self) -> u8 {
        (self.next() % 256) as u8
    }
}

fn dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("product-edges-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn close(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

fn part(length: f64, width: f64, angle: f64) -> Part {
    Part {
        x_mm: 1.5,
        y_mm: -2.5,
        angle_deg: angle,
        length_mm: length,
        width_mm: width,
        area_mm2: length * width,
    }
}

fn package(name: &str, long: f64, short: f64) -> Package {
    Package {
        name: name.to_string(),
        long_mm: long,
        short_mm: short,
        tolerance_mm: None,
    }
}

// ---------------------------------------------------------------- the configuration file

#[test]
fn an_empty_text_and_a_text_of_comments_are_the_defaults() {
    assert_eq!(Config::parse("").unwrap(), Config::default());
    assert_eq!(
        Config::parse("# a\n\n   \n# b\n").unwrap(),
        Config::default()
    );
}

#[test]
fn the_default_values_make_sense_together() {
    assert_eq!(Config::default().validate(), Ok(()));
}

#[test]
fn the_list_of_keys_has_no_key_twice_and_only_plain_names() {
    let mut seen = std::collections::HashSet::new();
    for key in KEYS {
        assert!(seen.insert(key), "{key} is listed twice");
        assert!(
            key.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
            "{key}"
        );
    }
    assert_eq!(KEYS.len(), 168);
}

#[test]
fn every_key_of_the_list_is_known_to_the_setter() {
    for key in KEYS {
        if let Err(why) = Config::default().set(key, "?") {
            assert!(!why.ends_with("unknown key"), "{key}: {why}");
        }
    }
}

#[test]
fn a_key_that_is_not_in_the_list_is_refused_and_the_case_counts() {
    assert_eq!(
        Config::default().set("nope", "1"),
        Err("nope: unknown key".to_string())
    );
    assert_eq!(
        Config::default().set("Polarity", "dark"),
        Err("Polarity: unknown key".to_string())
    );
    assert_eq!(
        Config::default().set("", "1"),
        Err(": unknown key".to_string())
    );
}

#[test]
fn the_eight_languages_and_auto_are_taken_and_nothing_else() {
    for word in ["auto", "en", "es", "de", "fr", "it", "ja", "zh", "pt"] {
        let mut c = Config::default();
        c.set("language", word).unwrap();
        assert_eq!(c.language, word);
    }
    for word in ["EN", "pt-PT", "", "english", "xx"] {
        let why = Config::default().set("language", word).unwrap_err();
        assert!(why.contains("is not auto"), "{word:?}: {why}");
    }
}

#[test]
fn the_blanks_around_a_value_are_dropped() {
    let mut c = Config::default();
    c.set("language", "  es \t").unwrap();
    assert_eq!(c.language, "es");
}

#[test]
fn the_polarity_is_dark_or_light_in_lower_case() {
    let mut c = Config::default();
    c.set("polarity", "light").unwrap();
    c.set("polarity", "dark").unwrap();
    assert_eq!(
        c.set("polarity", "Dark"),
        Err("polarity 'Dark' is not dark or light".to_string())
    );
}

#[test]
fn the_threshold_is_auto_or_a_byte() {
    let mut c = Config::default();
    c.set("threshold", "0").unwrap();
    assert_eq!(c.threshold, Some(0));
    c.set("threshold", "255").unwrap();
    assert_eq!(c.threshold, Some(255));
    c.set("threshold", "auto").unwrap();
    assert_eq!(c.threshold, None);
    for bad in ["256", "-1", "1.5", "", "AUTO", "0x10"] {
        assert_eq!(
            c.set("threshold", bad),
            Err(format!("threshold '{bad}' is not auto or 0 to 255"))
        );
    }
}

#[test]
fn a_number_that_is_not_finite_or_not_a_number_is_refused() {
    for bad in ["nan", "inf", "-inf", "1e999", "", "x", "1,5", "1 2"] {
        let why = Config::default().set("min_diameter", bad).unwrap_err();
        assert!(why.contains("is not a number"), "{bad:?}: {why}");
    }
    let mut c = Config::default();
    c.set("min_diameter", "1e1").unwrap();
    assert_eq!(c.min_diameter, 10.0);
}

#[test]
fn a_count_is_a_whole_number_with_no_sign_point_or_exponent() {
    for bad in ["-1", "2.0", "1e1", "", "+x", "two"] {
        let why = Config::default().set("grid_columns", bad).unwrap_err();
        assert!(why.contains("is not a whole number"), "{bad:?}: {why}");
    }
    let mut c = Config::default();
    c.set("grid_columns", "7").unwrap();
    assert_eq!(c.grid_columns, 7);
}

#[test]
fn a_switch_takes_yes_no_true_false_on_off_in_lower_case() {
    for (word, value) in [
        ("yes", true),
        ("true", true),
        ("on", true),
        ("no", false),
        ("false", false),
        ("off", false),
    ] {
        let mut c = Config::default();
        c.set("allow_scale", word).unwrap();
        assert_eq!(c.allow_scale, value, "{word}");
    }
    for bad in ["YES", "1", "0", "", "y"] {
        assert!(Config::default().set("allow_scale", bad).is_err(), "{bad}");
    }
}

#[test]
fn the_size_of_a_file_is_a_whole_number_that_validation_then_limits() {
    let mut c = Config::default();
    c.set("max_file_mib", "0").unwrap();
    assert!(c.validate().unwrap_err().contains("max_file_mib"));
    c.set("max_file_mib", "1025").unwrap();
    assert!(c.validate().is_err());
    c.set("max_file_mib", "18446744073709551615").unwrap();
    assert!(c.validate().is_err());
    assert!(c
        .set("max_file_mib", "18446744073709551616")
        .unwrap_err()
        .contains("is not a whole number"));
    c.set("max_file_mib", "1024").unwrap();
    assert_eq!(c.validate(), Ok(()));
    c.set("max_file_mib", "1").unwrap();
    assert_eq!(c.validate(), Ok(()));
}

#[test]
fn the_grid_has_from_two_to_fifty_columns_and_rows() {
    for text in [
        "grid_columns = 2",
        "grid_columns = 50",
        "grid_rows = 2",
        "grid_rows = 50",
    ] {
        assert!(Config::parse(text).is_ok(), "{text}");
    }
    for text in [
        "grid_columns = 1",
        "grid_columns = 51",
        "grid_rows = 1",
        "grid_rows = 51",
    ] {
        assert!(Config::parse(text).is_err(), "{text}");
    }
}

#[test]
fn the_fill_and_the_contrast_have_their_ends() {
    assert!(Config::parse("min_fill = 1").is_ok());
    assert!(Config::parse("min_fill = 0").is_err());
    assert!(Config::parse("min_fill = 1.0001").is_err());
    assert!(Config::parse("full_contrast = 1").is_ok());
    assert!(Config::parse("full_contrast = 0").is_err());
    assert!(Config::parse("max_elongation = 1").is_ok());
    assert!(Config::parse("max_elongation = 0.99").is_err());
    assert!(Config::parse("known_mm = 0").is_err());
    assert!(Config::parse("known_mm = 0.001").is_ok());
}

#[test]
fn a_mistake_names_the_line_even_after_comments_and_blank_lines() {
    assert_eq!(
        Config::parse("# c\n\nbad").unwrap_err(),
        "line 3: it must be key = value"
    );
    assert_eq!(
        Config::parse("# c\n\nthreshold = 1\n\n  threshold = 2").unwrap_err(),
        "line 5: threshold is written twice"
    );
}

#[test]
fn a_key_written_twice_is_refused_whatever_the_blanks_and_the_value() {
    assert_eq!(
        Config::parse("max_rms=1\n max_rms = 1").unwrap_err(),
        "line 2: max_rms is written twice"
    );
}

#[test]
fn a_comment_after_a_value_is_part_of_the_value() {
    let why = Config::parse("threshold = 5 # five").unwrap_err();
    assert!(why.contains("threshold '5 # five'"), "{why}");
}

#[test]
fn a_file_with_a_byte_order_mark_is_read_as_if_it_had_none() {
    assert_eq!(
        Config::parse("\u{feff}polarity = light").unwrap().polarity,
        Config::parse("polarity = light").unwrap().polarity
    );
    assert_eq!(
        Config::parse("\u{feff}# first line\nthreshold = 9")
            .unwrap()
            .threshold,
        Some(9)
    );
    assert_eq!(Config::parse("\u{feff}").unwrap(), Config::default());
}

#[test]
fn windows_line_ends_and_tabs_are_blanks() {
    let c = Config::parse("threshold = 7\r\n\tpolarity\t=\tlight\r\n").unwrap();
    assert_eq!(c.threshold, Some(7));
}

#[test]
fn a_line_with_an_empty_key_is_an_unknown_key() {
    let why = Config::parse("= 3").unwrap_err();
    assert!(why.contains("unknown key"), "{why}");
}

#[test]
fn a_value_with_letters_from_everywhere_does_not_stop_the_reader() {
    for text in [
        "language = é",
        "language = 日本語",
        "threshold = ٣",
        "polarity = ☀",
    ] {
        assert!(Config::parse(text).is_err(), "{text}");
    }
}

#[test]
fn texts_made_of_a_small_alphabet_are_read_without_a_panic() {
    let alphabet = b"abc =\n#_09.-\r\t";
    let mut rng = Lcg(7);
    for _ in 0..200 {
        let n = (rng.next() % 60) as usize;
        let text: String = (0..n)
            .map(|_| alphabet[(rng.next() % alphabet.len() as u64) as usize] as char)
            .collect();
        let _ = Config::parse(&text);
    }
}

#[test]
fn a_file_is_read_with_the_name_in_every_refusal() {
    let d = dir("config-read");
    let good = d.join("good.cfg");
    std::fs::write(&good, "threshold = 4\n").unwrap();
    assert_eq!(Config::read(&good).unwrap().threshold, Some(4));
    let bad = d.join("bad.cfg");
    std::fs::write(&bad, "threshold = 400\n").unwrap();
    let why = Config::read(&bad).unwrap_err();
    assert!(why.contains("bad.cfg") && why.contains("line 1"), "{why}");
    let why = Config::read(&d.join("none.cfg")).unwrap_err();
    assert!(why.contains("none.cfg"), "{why}");
}

#[test]
fn a_file_of_more_than_sixty_four_kibibytes_is_refused_and_one_of_exactly_that_is_not() {
    let d = dir("config-big");
    let exact = d.join("exact.cfg");
    let mut text = "# ".to_string();
    text.push_str(&"x".repeat(65_536 - 3));
    text.push('\n');
    assert_eq!(text.len(), 65_536);
    std::fs::write(&exact, &text).unwrap();
    assert!(Config::read(&exact).is_ok());
    let over = d.join("over.cfg");
    text.push('\n');
    std::fs::write(&over, &text).unwrap();
    assert!(Config::read(&over).unwrap_err().contains("64 KiB"));
}

#[test]
fn a_file_with_a_byte_order_mark_is_read() {
    let d = dir("config-bom");
    let f = d.join("bom.cfg");
    std::fs::write(&f, "\u{feff}threshold = 12\r\n").unwrap();
    assert_eq!(Config::read(&f).unwrap().threshold, Some(12));
}

// ---------------------------------------------------------------- what the server keeps and sends

#[test]
fn the_names_the_program_gives_the_server_do_not_change() {
    assert_eq!(API, "/api/v1");
    assert_eq!(APP, "vision-studio");
    assert_eq!(CONFIG_FORMAT, "text/x-eh3d-vision-config");
    assert_eq!(RESULT_FORMAT, "application/json");
    assert_eq!(config_name("bench-1"), "vision-bench-1.cfg");
}

#[test]
fn a_piece_of_a_path_is_letters_digits_underscore_and_hyphen_up_to_sixty_four() {
    let longest = "x".repeat(64);
    for good in ["a", "A_b-9", "0", longest.as_str()] {
        assert!(safe_segment(good), "{good}");
    }
    let long = "x".repeat(65);
    for bad in [
        "",
        "a b",
        "a/b",
        "a\\b",
        "é",
        "a.b",
        "..",
        "a\n",
        long.as_str(),
    ] {
        assert!(!safe_segment(bad), "{bad:?}");
    }
}

fn cfg_with(key: &str, value: &str) -> Config {
    let mut c = Config::default();
    c.set(key, value).unwrap();
    c
}

#[test]
fn no_server_is_a_link_that_is_fine() {
    assert!(link_settings(&Config::default()).is_ok());
}

#[test]
fn an_address_is_host_and_port_with_plain_characters() {
    for good in ["host:80", "127.0.0.1:18200", "[::1]:80", "a-b_c.d:65535"] {
        assert!(
            link_settings(&cfg_with("server_address", good)).is_ok(),
            "{good}"
        );
    }
    let long = format!("{}:80", "h".repeat(260));
    for bad in [
        "host",
        "host:0",
        "host:65536",
        ":80",
        "a b:1",
        "ho/st:1",
        "é:1",
        "host:x",
        "host:",
        long.as_str(),
    ] {
        assert!(
            link_settings(&cfg_with("server_address", bad)).is_err(),
            "{bad}"
        );
    }
}

#[test]
fn a_token_has_no_spaces_or_control_characters() {
    let mut c = cfg_with("server_address", "host:80");
    c.set("server_token", "abc.DEF-123").unwrap();
    assert!(link_settings(&c).is_ok());
    for bad in ["has space", "tab\there", "line\nbreak", "bell\u{7}"] {
        let mut c = cfg_with("server_address", "host:80");
        c.server_token = bad.to_string();
        assert!(link_settings(&c).is_err(), "{bad:?}");
    }
}

#[test]
fn a_machine_name_is_a_piece_of_a_path() {
    for good in ["m1", "bench_2-b", ""] {
        let mut c = cfg_with("server_address", "host:80");
        c.server_machine = good.to_string();
        assert!(link_settings(&c).is_ok(), "{good:?}");
    }
    for bad in ["a/b", "a b", "..", "é"] {
        let mut c = cfg_with("server_address", "host:80");
        c.server_machine = bad.to_string();
        assert!(link_settings(&c).is_err(), "{bad:?}");
    }
}

#[test]
fn the_waits_and_the_tries_of_the_link_have_their_ends() {
    for (key, good, bad) in [
        ("server_timeout_s", ["1", "120"], ["0", "121"]),
        ("server_retries", ["0", "5"], ["6", "100"]),
    ] {
        for value in good {
            assert!(
                link_settings(&cfg_with(key, value)).is_ok(),
                "{key} {value}"
            );
        }
        for value in bad {
            assert!(
                link_settings(&cfg_with(key, value)).is_err(),
                "{key} {value}"
            );
        }
    }
}

#[test]
fn a_token_to_an_address_that_is_not_local_is_warned_about_and_a_local_one_is_not() {
    let mut c = cfg_with("server_address", "8.8.8.8:80");
    c.server_token = "tok".into();
    let warning = plain_token_warning(&link_settings(&c).unwrap()).unwrap();
    assert!(warning.contains("8.8.8.8"), "{warning}");
    for local in ["127.0.0.1:80", "192.168.1.5:80", "10.0.0.2:80"] {
        let mut c = cfg_with("server_address", local);
        c.server_token = "tok".into();
        assert_eq!(plain_token_warning(&link_settings(&c).unwrap()), None);
    }
    let mut no_token = cfg_with("server_address", "8.8.8.8:80");
    no_token.server_token = String::new();
    assert_eq!(
        plain_token_warning(&link_settings(&no_token).unwrap()),
        None
    );
    assert_eq!(
        plain_token_warning(&link_settings(&Config::default()).unwrap()),
        None
    );
}

#[test]
fn base64_gives_the_known_texts_of_the_standard() {
    for (plain, coded) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(base64_encode(plain.as_bytes()), coded);
        assert_eq!(base64_decode(coded).unwrap(), plain.as_bytes());
    }
}

#[test]
fn base64_uses_plus_and_slash_for_the_last_two_digits() {
    assert_eq!(base64_encode(&[0xFB, 0xFF, 0xBF]), "+/+/");
    assert_eq!(base64_decode("////").unwrap(), [255, 255, 255]);
    assert_eq!(base64_decode("AA==").unwrap(), [0]);
    assert_eq!(base64_decode("AAA=").unwrap(), [0, 0]);
}

#[test]
fn bytes_of_any_length_go_through_base64_and_back() {
    let mut rng = Lcg(11);
    for n in 0usize..80 {
        let bytes: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let text = base64_encode(&bytes);
        assert_eq!(text.len(), n.div_ceil(3) * 4);
        assert_eq!(base64_decode(&text).unwrap(), bytes, "{n}");
    }
}

#[test]
fn a_text_that_is_not_base64_is_refused() {
    for bad in [
        "Zg=", "Zg==Zg==", "Z===", "====", "Zm9v!", "Zm9!", " Zg==", "Zm\u{e9}", "Zm9vYg=x",
        "Z-9v", "=AAA",
    ] {
        assert_eq!(base64_decode(bad), None, "{bad:?}");
    }
}

#[test]
fn the_lines_of_a_configuration_kept_on_the_server_are_read() {
    let pairs = parse_config_text("a = 1\n# c\n; d\n\nb=2 # note\nc = x#y\n").unwrap();
    assert_eq!(
        pairs,
        [
            ("a".to_string(), "1".to_string()),
            ("b".to_string(), "2".to_string()),
            ("c".to_string(), "x#y".to_string())
        ]
    );
}

#[test]
fn the_lines_of_a_configuration_with_a_mistake_name_the_line() {
    assert_eq!(
        parse_config_text("a = 1\na = 2").unwrap_err(),
        "line 2: \"a\" is written twice"
    );
    assert_eq!(
        parse_config_text("one").unwrap_err(),
        "line 1: there is no = in it"
    );
    assert_eq!(
        parse_config_text("# c\n\n= 3").unwrap_err(),
        "line 3: there is no key"
    );
}

#[test]
fn a_configuration_kept_on_the_server_may_start_with_a_byte_order_mark() {
    assert_eq!(
        parse_config_text("\u{feff}polarity = light").unwrap(),
        [("polarity".to_string(), "light".to_string())]
    );
    assert_eq!(
        parse_config_text("\u{feff}{\"polarity\": \"light\"}").unwrap(),
        [("polarity".to_string(), "light".to_string())]
    );
}

#[test]
fn a_configuration_with_windows_line_ends_is_read() {
    let pairs = parse_config_text("a = 1\r\nb = 2\r\n").unwrap();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[1].1, "2");
}

#[test]
fn a_configuration_as_a_json_object_is_read_in_the_order_it_is_written() {
    let pairs = parse_config_text("  {\"b\": 2, \"a\": \"x\", \"c\": true}").unwrap();
    let keys: Vec<&str> = pairs.iter().map(|p| p.0.as_str()).collect();
    assert_eq!(keys, ["b", "a", "c"]);
    assert_eq!(pairs[2].1, "true");
}

#[test]
fn the_pairs_of_a_configuration_are_written_one_to_a_line_with_no_line_break_inside() {
    let pairs = vec![
        ("a".to_string(), "1".to_string()),
        ("b".to_string(), "x\ny\r\nz".to_string()),
    ];
    assert_eq!(config_lines(&pairs), "a = 1\nb = x y  z\n");
    assert_eq!(config_lines(&[]), "");
}

#[test]
fn an_empty_object_and_blanks_around_it_have_no_pairs() {
    assert!(parse_object("{}").unwrap().is_empty());
    assert!(parse_object(" \n{ }\t ").unwrap().is_empty());
}

#[test]
fn the_escapes_of_a_json_string_are_made() {
    let pairs = parse_object(r#"{"a":"x\"y\\z\/\n\t\r\b\f"}"#).unwrap();
    assert_eq!(pairs[0].1, "x\"y\\z/\n\t\r\u{8}\u{c}");
}

/// An object with one string that holds the escape `\` and `code` (a backslash in the text of a test source can be read as a letter, so it is put in here).
fn escaped(code: &str) -> String {
    format!("{{\"a\":\"{}{code}\"}}", '\\')
}

#[test]
fn a_unicode_escape_gives_its_letter_and_a_pair_gives_one_letter() {
    assert_eq!(parse_object(&escaped("u00e9")).unwrap()[0].1, "é");
    assert_eq!(parse_object(&escaped("ud83d\\ude00")).unwrap()[0].1, "😀");
    assert_eq!(parse_object(&escaped("u65e5\\u672c")).unwrap()[0].1, "日本");
}

#[test]
fn half_of_a_pair_is_a_mark_and_the_letter_after_it_is_kept() {
    assert_eq!(parse_object(r#"{"a":"\ud83d"}"#).unwrap()[0].1, "\u{fffd}");
    assert_eq!(parse_object(r#"{"a":"\ude00"}"#).unwrap()[0].1, "\u{fffd}");
    assert_eq!(
        parse_object(&escaped("ud83d\\u0041")).unwrap()[0].1,
        "\u{fffd}A"
    );
    assert_eq!(
        parse_object(r#"{"a":"\ud83dx"}"#).unwrap()[0].1,
        "\u{fffd}x"
    );
}

#[test]
fn a_unicode_escape_has_four_hex_digits_and_nothing_else() {
    for bad in [
        r#"{"a":"\u+041"}"#,
        r#"{"a":"\u-041"}"#,
        r#"{"a":"\u004"}"#,
        r#"{"a":"\u00zz"}"#,
        r#"{"a":"\u 041"}"#,
        r#"{"a":"\u"#,
    ] {
        assert!(parse_object(bad).is_err(), "{bad}");
    }
}

#[test]
fn numbers_and_words_are_kept_as_they_are_written() {
    let pairs = parse_object(r#"{"n":1.5,"b":true,"c":false,"e":-2e3,"z":0}"#).unwrap();
    let values: Vec<&str> = pairs.iter().map(|p| p.1.as_str()).collect();
    assert_eq!(values, ["1.5", "true", "false", "-2e3", "0"]);
}

#[test]
fn null_and_nested_values_are_left_out() {
    let pairs =
        parse_object(r#"{"a":null,"b":{"x":[1,{"y":2}]},"c":["]}"],"d":"k","e":[]}"#).unwrap();
    assert_eq!(pairs, [("d".to_string(), "k".to_string())]);
}

#[test]
fn a_very_deep_list_is_skipped_without_using_the_stack() {
    let deep = format!(
        "{{\"a\":{}{},\"b\":\"k\"}}",
        "[".repeat(100_000),
        "]".repeat(100_000)
    );
    assert_eq!(
        parse_object(&deep).unwrap(),
        [("b".to_string(), "k".to_string())]
    );
}

#[test]
fn letters_from_everywhere_are_keys_and_values() {
    let pairs = parse_object("{\"é\":\"日本\"}").unwrap();
    assert_eq!(pairs, [("é".to_string(), "日本".to_string())]);
}

#[test]
fn a_member_written_twice_is_refused() {
    assert_eq!(
        parse_object(r#"{"a":1,"a":2}"#).unwrap_err(),
        "\"a\" is written twice"
    );
}

#[test]
fn text_that_is_not_an_object_is_refused_with_the_reason() {
    let cases = [
        ("", "it does not start with {"),
        ("[]", "it does not start with {"),
        (r#"{"a" 1}"#, "no : after \"a\""),
        (r#"{"a":}"#, "'' is not a value"),
        (r#"{"a":1 "b":2}"#, "a , or a } was expected"),
        (r#"{"a":1}x"#, "there is text after the object"),
        (r#"{"a":tru}"#, "'tru' is not a value"),
        (r#"{"a":1e999}"#, "'1e999' is not a value"),
        (r#"{"a":nan}"#, "'nan' is not a value"),
        (r#"{"a":"x"#, "a string is not closed"),
        (r#"{"a":"\q"}"#, "\\q is not an escape"),
        (r#"{"a":1,}"#, "a string was expected"),
    ];
    for (text, why) in cases {
        let e = parse_object(text).unwrap_err();
        assert!(e.contains(why), "{text}: {e}");
    }
}

#[test]
fn a_control_character_inside_a_string_is_refused() {
    let e = parse_object("{\"a\":\"x\ty\"}").unwrap_err();
    assert!(e.contains("control character"), "{e}");
}

#[test]
fn a_list_that_is_not_closed_or_closed_wrong_is_refused() {
    assert!(parse_object(r#"{"a":[1"#).is_err());
    assert!(parse_object(r#"{"a":[}"#).is_err());
    assert!(parse_object(r#"{"a":{"b":"#).is_err());
}

#[test]
fn texts_made_of_json_signs_are_read_without_a_panic() {
    let alphabet = b"{}[]\":,\\u0ab1 -.etrufals";
    let mut rng = Lcg(19);
    for _ in 0..300 {
        let n = (rng.next() % 40) as usize;
        let text: String = (0..n)
            .map(|_| alphabet[(rng.next() % alphabet.len() as u64) as usize] as char)
            .collect();
        let _ = parse_object(&text);
        let _ = parse_config_text(&text);
    }
}

#[test]
fn the_values_of_the_server_are_applied_and_the_link_and_unknown_keys_are_left_alone() {
    let pairs = vec![
        ("polarity".to_string(), "light".to_string()),
        ("server_address".to_string(), "evil:1".to_string()),
        ("find_server_max".to_string(), "9".to_string()),
        ("colour".to_string(), "red".to_string()),
        ("POLARITY".to_string(), "dark".to_string()),
        ("language".to_string(), "es".to_string()),
    ];
    let (config, done) = apply_remote(&Config::default(), &pairs).unwrap();
    assert_eq!(done.applied, ["polarity", "language"]);
    assert_eq!(
        done.ignored,
        ["server_address", "find_server_max", "colour", "POLARITY"]
    );
    assert_eq!(config.language, "es");
    assert_eq!(config.server_address, Config::default().server_address);
}

#[test]
fn a_value_of_the_server_that_is_refused_refuses_all_of_them() {
    let pairs = vec![
        ("polarity".to_string(), "light".to_string()),
        ("threshold".to_string(), "300".to_string()),
    ];
    let why = apply_remote(&Config::default(), &pairs).unwrap_err();
    assert!(
        why.contains("the server's value of threshold is refused"),
        "{why}"
    );
}

#[test]
fn values_of_the_server_that_do_not_go_together_are_refused() {
    let pairs = vec![
        ("min_diameter".to_string(), "30".to_string()),
        ("max_diameter".to_string(), "20".to_string()),
    ];
    let why = apply_remote(&Config::default(), &pairs).unwrap_err();
    assert!(why.contains("the server's values are refused"), "{why}");
}

#[test]
fn nothing_from_the_server_changes_nothing() {
    let (config, done) = apply_remote(&Config::default(), &[]).unwrap();
    assert_eq!(config, Config::default());
    assert!(done.applied.is_empty() && done.ignored.is_empty());
}

// ---------------------------------------------------------------- the texts of the window

#[test]
fn the_places_of_a_text_are_filled_in_one_pass() {
    assert_eq!(fill("{0} and {1}", &["a", "b"]), "a and b");
    assert_eq!(fill("{0}{0}", &["ab"]), "abab");
    assert_eq!(fill("no places", &["a"]), "no places");
    assert_eq!(fill("", &["a"]), "");
}

#[test]
fn a_value_that_looks_like_a_place_is_not_filled_again() {
    assert_eq!(fill("{0} {1}", &["{1}", "x"]), "{1} x");
    assert_eq!(fill("{1} {0}", &["{1}", "x"]), "x {1}");
    assert_eq!(fill("{0}", &["{0}"]), "{0}");
}

#[test]
fn a_place_with_no_value_and_signs_that_are_not_a_place_stay_as_they_are() {
    let none: [&str; 0] = [];
    assert_eq!(fill("{0}", &none), "{0}");
    assert_eq!(fill("{2} {0}", &["a"]), "{2} a");
    for text in ["a { b", "{}", "{a}", "{-1}", "{ 0}", "} {", "{"] {
        assert_eq!(fill(text, &["x"]), text, "{text}");
    }
    assert_eq!(fill("{{0}}", &["x"]), "{x}");
}

#[test]
fn values_with_letters_from_everywhere_are_filled_whole() {
    assert_eq!(fill("«{0}»", &["日本語 é"]), "«日本語 é»");
}

#[test]
fn every_language_has_its_two_letters_and_is_read_back_from_them() {
    assert_eq!(Lang::ALL.len(), 8);
    for l in Lang::ALL {
        assert_eq!(l.code().len(), 2);
        assert_eq!(Lang::from_code(l.code()), Some(l));
        assert!(!l.name().is_empty());
    }
}

#[test]
fn a_code_of_the_system_is_read_by_its_first_two_letters_in_any_case() {
    assert_eq!(Lang::from_code("es_ES.UTF-8"), Some(Lang::Es));
    assert_eq!(Lang::from_code("pt-PT"), Some(Lang::Pt));
    assert_eq!(Lang::from_code("PT"), Some(Lang::Pt));
    assert_eq!(Lang::from_code("zh_CN"), Some(Lang::Zh));
    for none in ["", "x", "xx", "é", "日本", "C", "--"] {
        assert_eq!(Lang::from_code(none), None, "{none:?}");
    }
}

#[test]
fn a_setting_of_the_language_is_a_code_and_a_wrong_one_is_english() {
    assert_eq!(Lang::from_setting("de"), Lang::De);
    assert_eq!(Lang::from_setting("xx"), Lang::En);
    assert_eq!(Lang::from_setting(""), Lang::En);
}

// ---------------------------------------------------------------- the report of a folder and the files of points and parts

#[test]
fn a_field_of_a_csv_file_is_quoted_when_it_needs_to_be() {
    assert_eq!(csv_field("abc"), "abc");
    assert_eq!(csv_field(""), "");
    assert_eq!(csv_field("a,b"), "\"a,b\"");
    assert_eq!(csv_field("a\"b"), "\"a\"\"b\"");
    assert_eq!(csv_field("a\nb"), "\"a\nb\"");
    assert_eq!(csv_field("日本,é"), "\"日本,é\"");
    assert_eq!(csv_field("a'b"), "a'b");
}

#[test]
fn a_field_that_a_spreadsheet_would_run_gets_a_quote_in_front() {
    for formula in ["=1+1", "+1", "-1", "@SUM(A1)"] {
        assert_eq!(csv_field(formula), format!("'{formula}"), "{formula}");
    }
    assert_eq!(csv_field("=a,b"), "\"'=a,b\"");
}

#[test]
fn a_field_that_starts_with_a_tab_or_a_line_end_is_guarded_too() {
    assert_eq!(csv_field("\tcmd"), "'\tcmd");
    assert_eq!(csv_field("\rcmd"), "\"'\rcmd\"");
}

#[test]
fn the_pictures_of_a_folder_are_the_png_and_pgm_files_by_name() {
    let d = dir("batch-list");
    for name in ["b.PGM", "a.png", "c.txt", "d.PNG", "e"] {
        std::fs::write(d.join(name), b"x").unwrap();
    }
    std::fs::create_dir_all(d.join("sub.png")).unwrap();
    let names: Vec<String> = list_pictures(&d, 10)
        .unwrap()
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["a.png", "b.PGM", "d.PNG"]);
}

#[test]
fn a_folder_with_more_pictures_than_the_limit_is_refused_and_one_at_the_limit_is_not() {
    let d = dir("batch-limit");
    for name in ["a.png", "b.png"] {
        std::fs::write(d.join(name), b"x").unwrap();
    }
    assert_eq!(list_pictures(&d, 2).unwrap().len(), 2);
    let why = list_pictures(&d, 1).unwrap_err();
    assert!(why.starts_with(TOO_MANY_FILES), "{why}");
    assert!(list_pictures(&d, 0).is_err());
}

#[test]
fn an_empty_folder_has_no_pictures_and_a_folder_that_is_not_there_is_unreadable() {
    let d = dir("batch-empty");
    assert!(list_pictures(&d, 5).unwrap().is_empty());
    let why = list_pictures(&d.join("none"), 5).unwrap_err();
    assert!(why.starts_with("file_unreadable"), "{why}");
}

#[test]
fn a_number_with_fixed_decimals_never_has_a_sign_when_it_rounds_to_nothing() {
    assert_eq!(fixed(-0.0001, 3), "0.000");
    assert_eq!(fixed(-0.0, 2), "0.00");
    assert_eq!(fixed(-0.4, 0), "0");
    assert_eq!(fixed(0.0, 0), "0");
    assert_eq!(fixed(-1.5, 1), "-1.5");
    assert_eq!(fixed(12.34567, 2), "12.35");
}

#[test]
fn a_number_that_is_not_finite_is_written_as_it_is_and_never_as_nothing() {
    assert_eq!(fixed(f64::NAN, 2), "NaN");
    assert_eq!(fixed(f64::INFINITY, 1), "inf");
    assert_eq!(fixed(f64::NEG_INFINITY, 1), "-inf");
}

#[test]
fn the_parts_go_through_csv_and_back() {
    let parts = vec![
        part(30.5, 15.5, 12.5),
        Part {
            x_mm: -3.25,
            ..part(2.0, 1.0, -45.0)
        },
    ];
    assert_eq!(parse_csv(&parts_csv(&parts)).unwrap(), parts);
}

#[test]
fn no_parts_are_a_header_alone_and_read_back_as_none() {
    assert_eq!(parts_csv(&[]), format!("{CSV_HEADER}\n"));
    assert!(parse_csv(&parts_csv(&[])).unwrap().is_empty());
}

#[test]
fn a_csv_of_parts_has_to_start_with_the_header() {
    for bad in ["", "index,x_mm\n1,2", "1,2,3,4,5,6,7", "\n\n"] {
        let e = parse_csv(bad).unwrap_err();
        assert!(e.starts_with("the first line must be"), "{bad:?}: {e}");
    }
}

#[test]
fn a_csv_of_parts_may_start_with_a_byte_order_mark_and_have_blank_lines_and_blanks() {
    let text = format!("\u{feff}{CSV_HEADER}\n\n 1 , 1.5 , 2 , 3 , 4 , 5 , 6 \n\n");
    let parts = parse_csv(&text).unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!((parts[0].x_mm, parts[0].area_mm2), (1.5, 6.0));
}

#[test]
fn a_line_of_a_csv_of_parts_that_is_wrong_is_named_by_its_real_line() {
    let text = format!("{CSV_HEADER}\n\n1,2,3,4,5,6,7\n\n2,2,3,4,5,6\n");
    assert_eq!(parse_csv(&text).unwrap_err(), "line 5: 6 fields, not 7");
    let text = format!("{CSV_HEADER}\n1,2,3,4,x,6,7\n");
    assert_eq!(
        parse_csv(&text).unwrap_err(),
        "line 2: a field is not a number"
    );
    for bad in ["nan", "inf", "-inf", "1e999", ""] {
        let text = format!("{CSV_HEADER}\n1,2,3,4,{bad},6,7\n");
        assert!(parse_csv(&text).is_err(), "{bad:?}");
    }
}

#[test]
fn the_points_of_a_file_are_read_without_comments_and_blank_lines() {
    let p = points::parse("# marks\n\n1,2\n 3.5 , -4 \r\n1e3,-2.5E-1\n", "f.csv").unwrap();
    assert_eq!(p, [(1.0, 2.0), (3.5, -4.0), (1000.0, -0.25)]);
    assert!(points::parse("", "f.csv").unwrap().is_empty());
}

#[test]
fn a_bad_point_is_named_by_its_real_line_in_the_file() {
    assert_eq!(
        points::parse("# c\n\n1,2\n3;4\n", "f.csv").unwrap_err(),
        "f.csv line 4: it must be x,y"
    );
    assert_eq!(
        points::parse("1,2\n\n# c\n3,x\n", "g.csv").unwrap_err(),
        "g.csv line 4: y 'x' is not a number"
    );
}

#[test]
fn a_file_of_points_may_start_with_a_byte_order_mark() {
    assert_eq!(
        points::parse("\u{feff}1,2\n", "f.csv").unwrap(),
        [(1.0, 2.0)]
    );
}

#[test]
fn a_point_with_a_third_value_or_a_number_that_is_not_finite_is_refused() {
    assert!(points::parse("1,2,3\n", "f").is_err());
    for bad in ["nan,1", "1,inf", "-inf,0", "1e999,0", ",1", "1,"] {
        assert!(points::parse(bad, "f").is_err(), "{bad}");
    }
}

#[test]
fn the_text_of_a_report_is_safe_as_html_text_and_as_an_attribute() {
    assert_eq!(escape("&<>\"'"), "&amp;&lt;&gt;&quot;&#39;");
    assert_eq!(escape(""), "");
    assert_eq!(escape("a\u{7}b"), "a b");
    assert_eq!(escape("a\nb"), "a\nb");
    assert_eq!(escape("日本 é"), "日本 é");
    assert_eq!(
        escape("<script>x</script>"),
        "&lt;script&gt;x&lt;/script&gt;"
    );
}

// ---------------------------------------------------------------- reading and writing files with a limit

#[test]
fn a_file_of_exactly_the_limit_is_read_and_one_byte_more_is_not() {
    let d = dir("limit-read");
    let f = d.join("one.bin");
    std::fs::write(&f, vec![7u8; 1_048_576]).unwrap();
    assert_eq!(read_limited(&f, 1).unwrap().len(), 1_048_576);
    std::fs::write(&f, vec![7u8; 1_048_577]).unwrap();
    assert!(read_limited(&f, 1).unwrap_err().starts_with(TOO_LARGE));
    assert_eq!(read_limited(&f, 2).unwrap().len(), 1_048_577);
}

#[test]
fn a_limit_of_nothing_takes_an_empty_file_and_refuses_any_byte() {
    let d = dir("limit-zero");
    let f = d.join("z.bin");
    std::fs::write(&f, b"").unwrap();
    assert!(read_limited(&f, 0).unwrap().is_empty());
    std::fs::write(&f, b"x").unwrap();
    assert!(read_limited(&f, 0).unwrap_err().starts_with(TOO_LARGE));
}

#[test]
fn the_biggest_limit_does_not_overflow() {
    let d = dir("limit-max");
    let f = d.join("m.bin");
    std::fs::write(&f, b"abc").unwrap();
    assert_eq!(read_limited(&f, u64::MAX).unwrap(), b"abc");
}

#[test]
fn a_file_that_is_not_there_is_unreadable_and_one_that_is_not_text_is_not_text() {
    let d = dir("limit-kinds");
    assert!(read_limited(&d.join("none"), 1)
        .unwrap_err()
        .starts_with(UNREADABLE));
    let f = d.join("b.bin");
    std::fs::write(&f, [0xFFu8, 0xFE, 0x00]).unwrap();
    assert!(read_text_limited(&f, 1).unwrap_err().starts_with(NOT_TEXT));
    std::fs::write(&f, "é日本").unwrap();
    assert_eq!(read_text_limited(&f, 1).unwrap(), "é日本");
}

#[test]
fn a_file_is_never_written_over_unless_it_is_asked() {
    let d = dir("limit-write");
    let f = d.join("out.txt");
    write_text(&f, "one", false).unwrap();
    assert!(write_text(&f, "two", false)
        .unwrap_err()
        .starts_with(OUTPUT_EXISTS));
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "one");
    write_text(&f, "two", true).unwrap();
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "two");
    write_text(&f, "3", true).unwrap();
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "3");
}

#[test]
fn a_file_in_a_folder_that_is_not_there_or_on_a_folder_is_unwritable() {
    let d = dir("limit-unwritable");
    assert!(write_bytes(&d.join("no").join("x"), b"a", false)
        .unwrap_err()
        .starts_with(UNWRITABLE));
    assert!(write_bytes(&d, b"a", true)
        .unwrap_err()
        .starts_with(UNWRITABLE));
}

// ---------------------------------------------------------------- the table of packages and the verdicts

const TABLE: &str = "package,body_long_mm,body_short_mm\n0603,1.6,0.8\n0805,2.0,1.25\n";

#[test]
fn a_table_is_read_with_its_columns_in_any_order_and_any_letter_case() {
    let t = parse_table("BODY_SHORT_MM, Package ,body_long_mm\n0.8,0603,1.6\n").unwrap();
    assert_eq!(t, [package("0603", 1.6, 0.8)]);
    assert_eq!(parse_table(TABLE).unwrap().len(), 2);
}

#[test]
fn a_table_may_have_comments_a_byte_order_mark_and_a_tolerance_column() {
    let text = "\u{feff}# the packages\n\npackage,body_long_mm,body_short_mm,tolerance_mm\n0603,1.6,0.8,0.05\n0805,2.0,1.25,\n";
    let t = parse_table(text).unwrap();
    assert_eq!(t[0].tolerance_mm, Some(0.05));
    assert_eq!(t[1].tolerance_mm, None);
}

#[test]
fn a_package_name_may_be_in_quotes_with_a_comma_in_it() {
    let t = parse_table("package,body_long_mm,body_short_mm\n\"a,\"\"b\"\"\",2,1\n").unwrap();
    assert_eq!(t[0].name, "a,\"b\"");
}

#[test]
fn a_table_that_is_empty_or_has_no_columns_is_refused() {
    assert!(parse_table("").unwrap_err().contains("is empty"));
    assert!(parse_table("# only\n\n").unwrap_err().contains("is empty"));
    let e = parse_table("package,body_long_mm\n").unwrap_err();
    assert!(e.contains("the columns must include"), "{e}");
}

#[test]
fn a_line_of_a_table_that_is_wrong_is_named_by_its_real_line() {
    let head = "package,body_long_mm,body_short_mm\n";
    let cases = [
        ("a,1\n", "line 2: 2 fields, the header has 3"),
        (",2,1\n", "line 2: the package has no name"),
        ("a,2,1\nA,3,2\n", "line 3: the package A is written twice"),
        (
            "a,1,2\n",
            "line 2: body_long_mm 1 is shorter than body_short_mm 2",
        ),
        (
            "a,0,0\n",
            "line 2: body_long_mm '0' is not a number above 0",
        ),
        ("a,-1,-2\n", "is not a number above 0"),
        (
            "a,x,1\n",
            "line 2: body_long_mm 'x' is not a number above 0",
        ),
        ("a,nan,1\n", "is not a number above 0"),
        ("a,inf,1\n", "is not a number above 0"),
        ("# c\n\na,2,1,\n", "line 4: 4 fields"),
        ("\"a,2,1\n", "line 2: a quote is not closed"),
    ];
    for (rows, why) in cases {
        let e = parse_table(&format!("{head}{rows}")).unwrap_err();
        assert!(e.contains(why), "{rows:?}: {e}");
    }
}

#[test]
fn a_table_has_at_most_ten_thousand_packages() {
    let mut text = String::from("package,body_long_mm,body_short_mm\n");
    for i in 0..MAX_PACKAGES {
        text.push_str(&format!("p{i},2,1\n"));
    }
    text.push_str("one-more,2,1\n");
    assert!(parse_table(&text).unwrap_err().contains("more than 10000"));
}

#[test]
fn a_package_is_found_by_its_name_in_any_letter_case_and_with_blanks() {
    let t = parse_table(TABLE).unwrap();
    assert_eq!(find_package(&t, " 0805 ").unwrap().long_mm, 2.0);
    let t = parse_table("package,body_long_mm,body_short_mm\nSOT-23,3,1.5\n").unwrap();
    assert_eq!(find_package(&t, "sot-23").unwrap().name, "SOT-23");
}

#[test]
fn a_package_that_is_not_there_names_some_that_are() {
    let t = parse_table(TABLE).unwrap();
    assert_eq!(
        find_package(&t, "x").unwrap_err(),
        "package_unknown: 'x' is not in the table (0603, 0805)"
    );
    let mut text = String::from("package,body_long_mm,body_short_mm\n");
    for i in 0..9 {
        text.push_str(&format!("p{i},2,1\n"));
    }
    let big = parse_table(&text).unwrap();
    assert!(find_package(&big, "x").unwrap_err().ends_with(", ...)"));
}

#[test]
fn the_tolerance_and_the_turn_that_are_asked_for_have_to_be_numbers_in_range() {
    let pkg = package("p", 2.0, 1.0);
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(judge(&[], &pkg, bad, 0.0).is_err(), "{bad}");
    }
    for bad in [-1.0, 91.0, f64::NAN, f64::INFINITY] {
        assert!(judge(&[], &pkg, 0.1, bad).is_err(), "{bad}");
    }
    assert!(judge(&[], &pkg, 0.1, 90.0).unwrap().is_empty());
}

#[test]
fn a_part_passes_up_to_the_tolerance_and_fails_one_step_beyond() {
    let pkg = package("p", 2.0, 1.0);
    let parts = [
        part(2.1, 1.0, 0.0),
        part(2.25, 1.0, 0.0),
        part(2.0, 0.75, 0.0),
    ];
    let j = judge(&parts, &pkg, 0.25, 0.0).unwrap();
    assert!(j[0].passes() && j[1].passes() && j[2].passes());
    let j = judge(&[part(2.2500001, 1.0, 0.0)], &pkg, 0.25, 0.0).unwrap();
    assert_eq!(j[0].reasons, [REASON_LENGTH]);
}

#[test]
fn the_reasons_come_in_the_same_order_every_time() {
    let pkg = package("p", 2.0, 1.0);
    let j = judge(&[part(3.0, 2.0, 40.0)], &pkg, 0.1, 10.0).unwrap();
    assert_eq!(j[0].reasons, [REASON_LENGTH, REASON_WIDTH, REASON_ANGLE]);
}

#[test]
fn a_turn_of_nothing_is_not_checked_and_a_limit_checks_both_directions() {
    let pkg = package("p", 2.0, 1.0);
    assert!(judge(&[part(2.0, 1.0, 80.0)], &pkg, 0.1, 0.0).unwrap()[0].passes());
    let j = judge(
        &[
            part(2.0, 1.0, -11.0),
            part(2.0, 1.0, 10.0),
            part(2.0, 1.0, 11.0),
        ],
        &pkg,
        0.1,
        10.0,
    )
    .unwrap();
    assert_eq!(
        j.iter().map(|x| x.passes()).collect::<Vec<_>>(),
        [false, true, false]
    );
}

#[test]
fn the_tolerance_of_a_package_wins_over_the_one_asked_for() {
    let mut pkg = package("p", 2.0, 1.0);
    pkg.tolerance_mm = Some(0.5);
    assert!(judge(&[part(2.4, 1.0, 0.0)], &pkg, 0.1, 0.0).unwrap()[0].passes());
    pkg.tolerance_mm = Some(0.01);
    assert!(!judge(&[part(2.1, 1.0, 0.0)], &pkg, 5.0, 0.0).unwrap()[0].passes());
}

#[test]
fn a_part_whose_size_is_not_a_number_never_passes() {
    let pkg = package("p", 2.0, 1.0);
    let j = judge(
        &[
            part(f64::NAN, 1.0, 0.0),
            part(2.0, f64::NAN, 0.0),
            part(2.0, 1.0, f64::NAN),
            part(f64::INFINITY, 1.0, 0.0),
        ],
        &pkg,
        0.1,
        10.0,
    )
    .unwrap();
    assert_eq!(j[0].reasons, [REASON_LENGTH]);
    assert_eq!(j[1].reasons, [REASON_WIDTH]);
    assert_eq!(j[2].reasons, [REASON_ANGLE]);
    assert_eq!(j[3].reasons, [REASON_LENGTH]);
}

#[test]
fn a_turn_that_is_not_a_number_is_not_judged_when_the_turn_is_not_checked() {
    let pkg = package("p", 2.0, 1.0);
    let j = judge(&[part(2.0, 1.0, f64::NAN)], &pkg, 0.1, 0.0).unwrap();
    assert!(j[0].passes());
}

#[test]
fn the_summary_counts_the_parts_that_pass() {
    let pkg = package("p", 2.0, 1.0);
    let j = judge(&[part(2.0, 1.0, 0.0), part(9.0, 1.0, 0.0)], &pkg, 0.1, 0.0).unwrap();
    assert_eq!(summary(&j), "2 parts: 1 pass, 1 fail");
    assert_eq!(summary(&[]), "0 parts: 0 pass, 0 fail");
}

#[test]
fn the_verdicts_as_csv_have_one_line_a_part_and_the_reasons_joined() {
    let pkg = package("a,b", 2.0, 1.0);
    let j = judge(&[part(3.0, 2.0, 0.0)], &pkg, 0.1, 0.0).unwrap();
    let csv = inspect_csv(&j, &pkg);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[1].starts_with("1,1.5000,-2.5000,0.000,3.0000,2.0000,\"a,b\",2.0000,1.0000,1.0000,1.0000,fail,length+width"));
}

#[test]
fn the_verdicts_as_json_name_the_format_and_escape_the_name() {
    let pkg = package("a\"b\\c\n", 2.0, 1.0);
    let j = judge(&[part(2.0, 1.0, 0.0)], &pkg, 0.1, 0.0).unwrap();
    let json = inspect_json(&j, &pkg, 0.1);
    assert!(json.contains("\"format\": \"eh3d-vision-inspection\""));
    assert!(json.contains("\"name\": \"a\\\"b\\\\c\\u000a\""), "{json}");
    assert!(json.contains("\"pass\": 1") && json.contains("\"fail\": 0"));
}

// ---------------------------------------------------------------- the fit of a board

#[test]
fn a_fit_that_is_not_a_number_is_never_accepted() {
    let t = Transform {
        dx: 0.0,
        dy: 0.0,
        theta_deg: 0.0,
        scale: 1.0,
        rms_mm: f64::NAN,
    };
    assert!(accept(&t, 1.0, 1.0).is_err());
    let t = Transform {
        rms_mm: 0.0,
        scale: f64::NAN,
        ..t
    };
    assert!(accept(&t, 1.0, 1.0).is_err());
    let t = Transform {
        scale: 1.0,
        rms_mm: f64::INFINITY,
        ..t
    };
    assert!(accept(&t, f64::INFINITY, 1.0).is_err());
}

#[test]
fn a_fit_is_accepted_up_to_its_limits_and_refused_beyond() {
    let t = Transform {
        dx: 0.0,
        dy: 0.0,
        theta_deg: 0.0,
        scale: 1.01,
        rms_mm: 0.1,
    };
    assert!(accept(&t, 0.1, 0.0101).is_ok());
    assert!(accept(&t, 0.0999, 0.0101)
        .unwrap_err()
        .contains("fit the design"));
    assert!(accept(&t, 0.1, 0.0099)
        .unwrap_err()
        .contains("bigger or smaller"));
}

#[test]
fn a_fit_needs_the_same_number_of_points_and_at_least_two_and_numbers() {
    let a = [(0.0, 0.0), (1.0, 0.0)];
    assert!(fit(&a, &a[..1], false).is_err());
    assert!(fit(&a[..1], &a[..1], false).is_err());
    assert!(fit(&[], &[], false).is_err());
    let nan = [(0.0, 0.0), (f64::NAN, 0.0)];
    assert_eq!(fit(&a, &nan, false).unwrap_err(), "a point is not a number");
    let same = [(1.0, 1.0), (1.0, 1.0)];
    assert!(fit(&same, &a, false)
        .unwrap_err()
        .contains("all the same point"));
}

#[test]
fn a_shift_a_turn_and_a_size_are_found() {
    let design = [(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)];
    let shifted: Vec<(f64, f64)> = design.iter().map(|p| (p.0 + 5.0, p.1 - 3.0)).collect();
    let t = fit(&design, &shifted, false).unwrap();
    assert!(close(t.dx, 5.0, 1e-9) && close(t.dy, -3.0, 1e-9) && close(t.theta_deg, 0.0, 1e-9));
    assert!(close(t.scale, 1.0, 1e-12) && t.rms_mm < 1e-9);
    let turned: Vec<(f64, f64)> = design.iter().map(|p| (-p.1, p.0)).collect();
    let t = fit(&design, &turned, false).unwrap();
    assert!(close(t.theta_deg, 90.0, 1e-9), "{t:?}");
    let bigger: Vec<(f64, f64)> = design.iter().map(|p| (p.0 * 1.02, p.1 * 1.02)).collect();
    let t = fit(&design, &bigger, true).unwrap();
    assert!(close(t.scale, 1.02, 1e-9), "{t:?}");
    assert!(fit(&design, &bigger, false).unwrap().scale == 1.0);
}

#[test]
fn a_point_goes_to_the_machine_and_back_to_the_design() {
    let mut rng = Lcg(23);
    for _ in 0..50 {
        let t = Transform {
            dx: rng.unit() * 100.0 - 50.0,
            dy: rng.unit() * 100.0 - 50.0,
            theta_deg: rng.unit() * 360.0 - 180.0,
            scale: 0.5 + rng.unit() * 2.0,
            rms_mm: 0.0,
        };
        let p = (rng.unit() * 200.0 - 100.0, rng.unit() * 200.0 - 100.0);
        let back = t.inverse(t.apply(p));
        assert!(
            close(back.0, p.0, 1e-9) && close(back.1, p.1, 1e-9),
            "{t:?}"
        );
    }
}

#[test]
fn marks_are_paired_with_the_design_in_any_order() {
    let design = [(0.0, 0.0), (50.0, 0.0), (50.0, 30.0), (0.0, 30.0)];
    let found = [(0.2, 30.1), (50.1, 0.0), (0.0, 0.1), (50.0, 29.9)];
    let paired = pair_nearest(&design, &found).unwrap();
    assert_eq!(paired, [(0.0, 0.1), (50.1, 0.0), (50.0, 29.9), (0.2, 30.1)]);
}

#[test]
fn marks_that_are_not_the_same_number_as_the_design_are_not_paired() {
    let design = [(0.0, 0.0), (10.0, 0.0)];
    assert!(pair_nearest(&design, &design[..1]).is_err());
    assert!(pair_nearest(&design[..1], &design[..1]).is_err());
    assert!(pair_nearest(&[], &[]).is_err());
}

#[test]
fn the_options_for_the_machine_have_a_line_for_each_pair_and_stop_at_the_shorter_list() {
    let args = isocam_args(
        &[(0.0, 0.0), (1.5, -2.0)],
        &[(0.25, 1.0), (3.0, 4.0), (9.0, 9.0)],
    );
    assert_eq!(args, ["--fiducial 0,0,0.25,1", "--fiducial 1.5,-2,3,4"]);
    assert!(isocam_args(&[], &[]).is_empty());
}

// ---------------------------------------------------------------- the scale and the lens

#[test]
fn a_scale_needs_a_mark_and_a_size_that_are_more_than_nothing() {
    let s = Scale::from_known_diameter(10.0, 1.0).unwrap();
    assert!(close(s.mm_per_px, 0.1, 1e-15));
    assert!(close(s.mm(25.0), 2.5, 1e-12));
    for (px, mm) in [
        (0.0, 1.0),
        (-1.0, 1.0),
        (1.0, 0.0),
        (1.0, -1.0),
        (f64::NAN, 1.0),
        (1.0, f64::NAN),
        (f64::INFINITY, 1.0),
        (1.0, f64::INFINITY),
    ] {
        assert!(Scale::from_known_diameter(px, mm).is_err(), "{px} {mm}");
    }
}

#[test]
fn a_pixel_goes_to_millimetres_from_the_centre_with_y_going_up() {
    let s = Scale { mm_per_px: 0.5 };
    assert_eq!(s.point(110.0, 40.0, (100.0, 50.0)), (5.0, 5.0));
    assert_eq!(s.point(100.0, 50.0, (100.0, 50.0)), (0.0, 0.0));
    assert_eq!(s.point(90.0, 60.0, (100.0, 50.0)), (-5.0, -5.0));
}

#[test]
fn no_distortion_leaves_a_point_where_it_is() {
    let mut rng = Lcg(29);
    for _ in 0..30 {
        let p = (rng.unit() * 640.0, rng.unit() * 480.0);
        let q = undistort(p, (320.0, 240.0), 0.0, 400.0);
        assert!(close(q.0, p.0, 1e-9) && close(q.1, p.1, 1e-9));
    }
    assert_eq!(undistort((5.0, 5.0), (5.0, 5.0), 0.7, 400.0), (5.0, 5.0));
}

#[test]
fn the_distortion_that_made_the_points_is_found_again() {
    let centre = (100.0, 80.0);
    let norm = 130.0;
    let mut rng = Lcg(31);
    let measured: Vec<(f64, f64)> = (0..12)
        .map(|_| (rng.unit() * 200.0, rng.unit() * 160.0))
        .collect();
    let ideal: Vec<(f64, f64)> = measured
        .iter()
        .map(|p| undistort(*p, centre, 0.2, norm))
        .collect();
    let k1 = fit_k1(&measured, &ideal, centre, norm).unwrap();
    assert!(close(k1, 0.2, 1e-9), "{k1}");
    assert_eq!(fit_k1(&measured, &measured, centre, norm).unwrap(), 0.0);
}

#[test]
fn a_distortion_cannot_be_measured_from_too_few_or_unequal_or_central_points() {
    let a = [(1.0, 2.0), (3.0, 4.0), (5.0, 6.0)];
    assert!(fit_k1(&a, &a[..2], (0.0, 0.0), 10.0).is_err());
    assert!(fit_k1(&a[..2], &a[..2], (0.0, 0.0), 10.0).is_err());
    for norm in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(fit_k1(&a, &a, (0.0, 0.0), norm).is_err(), "{norm}");
    }
    let centre = [(7.0, 7.0), (7.0, 7.0), (7.0, 7.0)];
    assert!(fit_k1(&centre, &centre, (7.0, 7.0), 10.0)
        .unwrap_err()
        .contains("all at the centre"));
}

fn lattice(columns: usize, rows: usize, pitch: f64) -> Vec<(f64, f64)> {
    (0..rows)
        .flat_map(|r| (0..columns).map(move |c| (30.0 + c as f64 * pitch, 25.0 + r as f64 * pitch)))
        .collect()
}

#[test]
fn the_scale_from_the_pitch_of_a_grid_is_the_pitch_over_the_distance_between_marks() {
    let p = from_pitch(&lattice(3, 3, 10.0), 2.0, 0.01).unwrap();
    assert!(close(p.scale.mm_per_px, 0.2, 1e-12));
    assert_eq!((p.marks, p.spread), (9, 0.0));
    let even = from_pitch(&lattice(2, 2, 40.0), 2.0, 0.01).unwrap();
    assert!(close(even.scale.mm_per_px, 0.05, 1e-12));
}

#[test]
fn a_pitch_scale_is_refused_for_numbers_that_make_no_sense() {
    let l = lattice(3, 3, 10.0);
    for pitch in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(from_pitch(&l, pitch, 0.05).is_err(), "{pitch}");
    }
    for tolerance in [-0.1, f64::NAN, f64::INFINITY] {
        assert!(from_pitch(&l, 1.0, tolerance).is_err(), "{tolerance}");
    }
    assert!(from_pitch(&l, 1.0, 0.0).is_ok());
}

#[test]
fn a_pitch_scale_needs_four_marks_and_at_most_two_thousand() {
    let l = lattice(3, 3, 10.0);
    assert!(from_pitch(&l[..3], 1.0, 0.05)
        .unwrap_err()
        .contains("at least 4"));
    assert!(from_pitch(&l[..4], 1.0, 0.5).is_ok());
    let many: Vec<(f64, f64)> = (0..=MAX_PITCH_MARKS).map(|i| (i as f64, 0.0)).collect();
    assert!(from_pitch(&many, 1.0, 0.05)
        .unwrap_err()
        .contains("2000 marks at most"));
}

#[test]
fn a_pitch_scale_is_refused_for_marks_that_are_not_numbers_or_are_in_one_place() {
    let mut l = lattice(3, 3, 10.0);
    l[4] = (f64::NAN, 1.0);
    assert!(from_pitch(&l, 1.0, 0.05)
        .unwrap_err()
        .contains("not a number"));
    let mut l = lattice(3, 3, 10.0);
    l[4] = l[3];
    assert!(from_pitch(&l, 1.0, 0.05)
        .unwrap_err()
        .contains("same place"));
}

#[test]
fn a_target_that_is_not_square_on_is_refused_and_not_averaged() {
    let mut l = lattice(3, 3, 10.0);
    for p in &mut l {
        p.0 *= 1.0 + p.1 / 40.0;
    }
    assert!(from_pitch(&l, 1.0, 0.001)
        .unwrap_err()
        .contains("differ by"));
    assert!(from_pitch(&l, 1.0, 50.0).is_ok());
}

// ---------------------------------------------------------------- the zoom and the plan of the pictures

#[test]
fn the_whole_picture_is_shown_with_its_margin_and_a_picture_with_no_size_gives_the_start() {
    let v = Viewport::fit(100.0, 50.0, 200.0, 100.0, 0.0);
    assert_eq!((v.centre_x, v.centre_y, v.zoom), (50.0, 25.0, 2.0));
    let v = Viewport::fit(100.0, 50.0, 220.0, 120.0, 10.0);
    assert_eq!(v.zoom, 2.0);
    for (w, h) in [
        (0.0, 5.0),
        (5.0, 0.0),
        (-1.0, 5.0),
        (f64::NAN, 5.0),
        (5.0, f64::INFINITY),
    ] {
        assert_eq!(Viewport::fit(w, h, 100.0, 100.0, 0.0), Viewport::default());
    }
}

#[test]
fn a_view_that_is_all_margin_still_has_a_zoom_inside_the_ends() {
    let v = Viewport::fit(100.0, 100.0, 10.0, 10.0, 100.0);
    assert_eq!(v.zoom, MIN_ZOOM);
    let v = Viewport::fit(1.0, 1.0, 100_000.0, 100_000.0, 0.0);
    assert_eq!(v.zoom, MAX_ZOOM);
    let v = Viewport::fit(1.0e9, 1.0e9, 100.0, 100.0, 0.0);
    assert_eq!(v.zoom, MIN_ZOOM);
    let v = Viewport::fit(100.0, 100.0, f64::NAN, f64::NAN, f64::NAN);
    assert!(v.zoom >= MIN_ZOOM && v.zoom <= MAX_ZOOM);
}

#[test]
fn a_point_goes_to_the_screen_and_back_to_the_picture() {
    let mut rng = Lcg(37);
    for _ in 0..50 {
        let v = Viewport {
            centre_x: rng.unit() * 1000.0,
            centre_y: rng.unit() * 1000.0,
            zoom: MIN_ZOOM + rng.unit() * 10.0,
        };
        let p = (rng.unit() * 2000.0 - 500.0, rng.unit() * 2000.0 - 500.0);
        let s = v.to_screen(p.0, p.1, 800.0, 600.0);
        let back = v.to_image(s.0, s.1, 800.0, 600.0);
        assert!(close(back.0, p.0, 1e-6) && close(back.1, p.1, 1e-6));
    }
}

#[test]
fn a_zoom_keeps_the_point_under_the_pointer_where_it_is() {
    let mut rng = Lcg(41);
    for _ in 0..40 {
        let mut v = Viewport::fit(640.0, 480.0, 800.0, 600.0, 10.0);
        let (sx, sy) = (rng.unit() * 800.0, rng.unit() * 600.0);
        let before = v.to_image(sx, sy, 800.0, 600.0);
        v.zoom_at(sx, sy, 800.0, 600.0, 0.5 + rng.unit() * 2.0);
        let after = v.to_image(sx, sy, 800.0, 600.0);
        assert!(close(before.0, after.0, 1e-6) && close(before.1, after.1, 1e-6));
    }
}

#[test]
fn a_factor_that_is_not_a_positive_number_does_not_move_the_view() {
    let start = Viewport::fit(640.0, 480.0, 800.0, 600.0, 10.0);
    for factor in [0.0, -2.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut v = start;
        v.zoom_at(100.0, 100.0, 800.0, 600.0, factor);
        assert_eq!(v, start, "{factor}");
    }
}

#[test]
fn the_zoom_never_leaves_its_ends() {
    let mut v = Viewport::default();
    for _ in 0..40 {
        v.zoom_at(0.0, 0.0, 100.0, 100.0, 10.0);
    }
    assert_eq!(v.zoom, MAX_ZOOM);
    for _ in 0..40 {
        v.zoom_at(0.0, 0.0, 100.0, 100.0, 0.1);
    }
    assert_eq!(v.zoom, MIN_ZOOM);
}

#[test]
fn dragging_the_picture_moves_the_centre_the_other_way_by_the_zoom() {
    let mut v = Viewport {
        centre_x: 100.0,
        centre_y: 100.0,
        zoom: 2.0,
    };
    v.pan_px(20.0, -10.0);
    assert_eq!((v.centre_x, v.centre_y), (90.0, 105.0));
}

#[test]
fn a_part_that_fits_the_field_is_one_picture_and_a_longer_one_is_covered_with_overlap() {
    let p = plan(4.0, 3.0, 8.0, 6.0, 0.0, 0.0).unwrap();
    assert_eq!((p.columns, p.rows, p.shots.len()), (1, 1, 1));
    let p = plan(10.0, 10.0, 6.0, 6.0, 0.1, 0.5).unwrap();
    assert_eq!((p.columns, p.rows), (2, 2));
    assert_eq!(
        p.shots,
        [(-2.5, 2.5), (2.5, 2.5), (2.5, -2.5), (-2.5, -2.5)]
    );
}

#[test]
fn the_plan_as_csv_has_one_line_a_picture_numbered_from_one() {
    let p = plan(10.0, 10.0, 6.0, 6.0, 0.1, 0.5).unwrap();
    assert_eq!(
        shots_csv(&p),
        format!("{SHOTS_HEADER}\n1,-2.5000,2.5000\n2,2.5000,2.5000\n3,2.5000,-2.5000\n4,-2.5000,-2.5000\n")
    );
    let one = plan(1.0, 1.0, 5.0, 5.0, 0.0, 0.0).unwrap();
    assert_eq!(
        shots_csv(&one),
        format!("{SHOTS_HEADER}\n1,0.0000,0.0000\n")
    );
}

#[test]
fn a_plan_is_refused_for_sizes_overlaps_and_margins_that_make_no_sense() {
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(plan(bad, 1.0, 1.0, 1.0, 0.1, 0.0).is_err(), "{bad}");
        assert!(plan(1.0, bad, 1.0, 1.0, 0.1, 0.0).is_err(), "{bad}");
        assert!(plan(1.0, 1.0, bad, 1.0, 0.1, 0.0).is_err(), "{bad}");
        assert!(plan(1.0, 1.0, 1.0, bad, 0.1, 0.0).is_err(), "{bad}");
    }
    for overlap in [-0.01, 0.91, f64::NAN, f64::INFINITY] {
        assert!(plan(1.0, 1.0, 1.0, 1.0, overlap, 0.0).is_err(), "{overlap}");
    }
    assert!(plan(5.0, 5.0, 2.0, 2.0, 0.9, 0.0).is_ok());
    assert!(plan(5.0, 5.0, 2.0, 2.0, 0.0, 0.0).is_ok());
    for margin in [-0.1, f64::NAN, f64::INFINITY] {
        assert!(plan(1.0, 1.0, 1.0, 1.0, 0.1, margin).is_err(), "{margin}");
    }
}

#[test]
fn a_plan_with_too_many_pictures_is_refused_and_never_allocated() {
    let e = plan(1.0e6, 1.0, 1.0, 1.0, 0.1, 0.0).unwrap_err();
    assert!(e.starts_with("too_many_shots"), "{e}");
    assert!(plan(1.0e300, 1.0e300, 1.0e-300, 1.0e-300, 0.5, 0.0).is_err());
    assert!(plan(f64::MAX, 1.0, 1.0, 1.0, 0.0, f64::MAX).is_err());
    assert_eq!(MAX_SHOTS, 10_000);
}

#[test]
fn every_picture_of_a_plan_is_inside_the_part_with_its_margin() {
    let mut rng = Lcg(43);
    for _ in 0..40 {
        let (w, h) = (1.0 + rng.unit() * 50.0, 1.0 + rng.unit() * 50.0);
        let (fw, fh) = (2.0 + rng.unit() * 8.0, 2.0 + rng.unit() * 8.0);
        let p = plan(w, h, fw, fh, 0.1, 0.5).unwrap();
        assert_eq!(p.shots.len(), p.columns * p.rows);
        for (x, y) in &p.shots {
            assert!(x.abs() <= (w + 1.0) / 2.0 + 1e-9, "{x} in {w}");
            assert!(y.abs() <= (h + 1.0) / 2.0 + 1e-9, "{y} in {h}");
        }
    }
}
