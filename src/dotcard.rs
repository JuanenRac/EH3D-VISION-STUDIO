//! The card of calibration to print: a grid of round dots at a known distance on a page of a known size, as a drawing in millimetres (SVG) that is the same every time it is made. The
//! pitch between the dots is what `calibrate --pitch-mm` and `lens` read from a picture of the card, the dot that is larger than the others says which corner is the top left, and a bar of a
//! known length under the grid lets a person check with a ruler that the page was printed at 100 %.
//!
//! The size of the grid, the dots, the margins and the page are settings (the `card_*` keys); a card that does not fit the page inside its margins is refused with `card_too_big`
//! instead of being drawn smaller, because a card drawn smaller has not the pitch it says. Nothing here reads the clock or the machine: the same settings and language give the same bytes.

use crate::lang::{Lang, Msg};

/// The code of the refusal of a card that does not fit its page inside the margins.
pub const TOO_BIG: &str = "card_too_big";
/// The code of the refusal of settings that make no sense.
pub const BAD_SETTING: &str = "card_bad_setting";

/// The factor by which the dot of the top left corner is larger than the others.
pub const CORNER_FACTOR: f64 = 1.6;

/// What is asked of the card, in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardSettings {
    /// The dots along each side (3 to 40).
    pub columns: usize,
    pub rows: usize,
    /// The distance between the centres of neighbouring dots.
    pub pitch_mm: f64,
    /// The diameter of a dot.
    pub dot_mm: f64,
    /// The margin that is left free around the grid and its bar, on every side.
    pub margin_mm: f64,
    /// The size of the page.
    pub paper_w_mm: f64,
    pub paper_h_mm: f64,
}

impl Default for CardSettings {
    fn default() -> Self {
        CardSettings {
            columns: 9,
            rows: 7,
            pitch_mm: 10.0,
            dot_mm: 4.0,
            margin_mm: 10.0,
            paper_w_mm: 210.0,
            paper_h_mm: 297.0,
        }
    }
}

/// Whether the settings make sense; the reason when they do not.
pub fn check(s: &CardSettings) -> Result<(), String> {
    let bad = |why: &str| Err(format!("{BAD_SETTING}: {why}"));
    if !(3..=40).contains(&s.columns) || !(3..=40).contains(&s.rows) {
        return bad("the dots of a card must be from 3 to 40 on a side");
    }
    let finite = [
        s.pitch_mm,
        s.dot_mm,
        s.margin_mm,
        s.paper_w_mm,
        s.paper_h_mm,
    ]
    .iter()
    .all(|v| v.is_finite());
    if !finite {
        return bad("a size of the card is not a number");
    }
    if !(2.0..=100.0).contains(&s.pitch_mm) {
        return bad("the pitch of the dots must be from 2 to 100 mm");
    }
    if !(0.3..=s.pitch_mm / CORNER_FACTOR / 1.2).contains(&s.dot_mm) {
        return bad("the dot must be from 0.3 mm to a little under half the pitch, so that the corner dot does not touch its neighbours");
    }
    if !(0.0..=50.0).contains(&s.margin_mm) {
        return bad("the margin must be from 0 to 50 mm");
    }
    if !(50.0..=2000.0).contains(&s.paper_w_mm) || !(50.0..=2000.0).contains(&s.paper_h_mm) {
        return bad("the page must be from 50 to 2000 mm on a side");
    }
    Ok(())
}

/// The length of the bar under the grid: the largest of 10, 20, 50 and 100 mm that is not longer than the grid, or `None` when the grid is shorter than 10 mm.
pub fn bar_mm(s: &CardSettings) -> Option<f64> {
    let width = (s.columns - 1) as f64 * s.pitch_mm;
    [100.0, 50.0, 20.0, 10.0].into_iter().find(|&b| b <= width)
}

/// The centres of the dots on the page in millimetres, row after row, the grid being centred in the width of the page; the first is the corner dot.
pub fn dots(s: &CardSettings) -> Result<Vec<(f64, f64)>, String> {
    check(s)?;
    let gw = (s.columns - 1) as f64 * s.pitch_mm;
    let gh = (s.rows - 1) as f64 * s.pitch_mm;
    let corner = s.dot_mm * CORNER_FACTOR;
    let text_room = 14.0;
    let need_w = gw + corner + 2.0 * s.margin_mm;
    let need_h = gh + corner + 2.0 * s.margin_mm + text_room;
    if need_w > s.paper_w_mm || need_h > s.paper_h_mm {
        return Err(format!(
            "{TOO_BIG}: the card needs {need_w:.1} by {need_h:.1} mm with its margins and its guide, and the page is {:.1} by {:.1} mm",
            s.paper_w_mm, s.paper_h_mm
        ));
    }
    let x0 = (s.paper_w_mm - gw) / 2.0;
    let y0 = s.margin_mm + corner / 2.0;
    let mut out = Vec::with_capacity(s.columns * s.rows);
    for r in 0..s.rows {
        for c in 0..s.columns {
            out.push((x0 + c as f64 * s.pitch_mm, y0 + r as f64 * s.pitch_mm));
        }
    }
    Ok(out)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn num(v: f64) -> String {
    let t = format!("{v:.3}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    if t.is_empty() || t == "-0" {
        "0".to_string()
    } else {
        t.to_string()
    }
}

/// The card as an SVG drawing in millimetres, with the guide in the language.
pub fn svg(s: &CardSettings, lang: Lang) -> Result<String, String> {
    let centres = dots(s)?;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}mm\" height=\"{h}mm\" viewBox=\"0 0 {w} {h}\">\n",
        w = num(s.paper_w_mm),
        h = num(s.paper_h_mm)
    ));
    out.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"#ffffff\"/>\n",
        num(s.paper_w_mm),
        num(s.paper_h_mm)
    ));
    for (i, (x, y)) in centres.iter().enumerate() {
        let d = if i == 0 {
            s.dot_mm * CORNER_FACTOR
        } else {
            s.dot_mm
        };
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#000000\"/>\n",
            num(*x),
            num(*y),
            num(d / 2.0)
        ));
    }
    let gh = (s.rows - 1) as f64 * s.pitch_mm;
    let bar_y =
        s.margin_mm + s.dot_mm * CORNER_FACTOR / 2.0 + gh + s.dot_mm * CORNER_FACTOR / 2.0 + 5.0;
    let bar = bar_mm(s);
    if let Some(b) = bar {
        let x0 = (s.paper_w_mm - b) / 2.0;
        out.push_str(&format!(
            "<path d=\"M{} {}H{}M{} {}v-2M{} {}v-2\" stroke=\"#000000\" stroke-width=\"0.3\" fill=\"none\"/>\n",
            num(x0),
            num(bar_y),
            num(x0 + b),
            num(x0),
            num(bar_y),
            num(x0 + b),
            num(bar_y)
        ));
    }
    let guide = Msg::CardDotsGuide.with(
        lang,
        &[
            s.columns.to_string(),
            s.rows.to_string(),
            num(s.pitch_mm),
            bar.map_or_else(|| "-".to_string(), num),
        ],
    );
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"3\" text-anchor=\"middle\" fill=\"#000000\">{}</text>\n",
        num(s.paper_w_mm / 2.0),
        num(bar_y + 6.0),
        escape(&guide)
    ));
    out.push_str("</svg>\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fiducial::{find, Settings};
    use crate::image::Gray;

    #[test]
    fn the_card_is_the_same_every_time_and_has_a_dot_for_each_place() {
        let s = CardSettings::default();
        let a = svg(&s, Lang::En).unwrap();
        assert_eq!(a, svg(&s, Lang::En).unwrap());
        assert_eq!(a.matches("<circle").count(), 63);
        assert!(a.starts_with("<svg ") && a.trim_end().ends_with("</svg>"));
        assert!(a.contains("width=\"210mm\" height=\"297mm\""));
        assert!(!a.contains("\r"));
    }

    #[test]
    fn the_guide_is_in_each_language_and_the_drawing_does_not_change() {
        let s = CardSettings::default();
        let en = svg(&s, Lang::En).unwrap();
        for l in Lang::ALL {
            let t = svg(&s, l).unwrap();
            assert!(t.contains("<text"), "{}", l.code());
            let strip = |x: &str| x.split("<text").next().unwrap().to_string();
            assert_eq!(strip(&t), strip(&en));
        }
        assert_ne!(svg(&s, Lang::Es).unwrap(), en);
    }

    #[test]
    fn the_dots_found_in_a_picture_of_the_card_are_where_the_pitch_says() {
        let s = CardSettings {
            columns: 5,
            rows: 4,
            pitch_mm: 10.0,
            dot_mm: 4.0,
            ..CardSettings::default()
        };
        let centres = dots(&s).unwrap();
        let scale = 4.0;
        let mut g = Gray::blank(840, 400, 255).unwrap();
        for (i, (x, y)) in centres.iter().enumerate() {
            let d = if i == 0 { 4.0 * CORNER_FACTOR } else { 4.0 };
            g.disc(x * scale - 0.0, y * scale, d * scale / 2.0, 0);
        }
        let found = find(
            &g,
            &Settings {
                max_diameter: 40.0,
                ..Settings::default()
            },
        )
        .unwrap();
        assert_eq!(found.len(), 20);
        let biggest = found
            .iter()
            .max_by(|a, b| a.diameter.partial_cmp(&b.diameter).unwrap())
            .unwrap();
        let first = centres[0];
        assert!(
            (biggest.x - first.0 * scale).abs() < 0.5 && (biggest.y - first.1 * scale).abs() < 0.5
        );
    }

    #[test]
    fn a_card_that_does_not_fit_its_page_is_refused_with_its_code() {
        let s = CardSettings {
            columns: 40,
            pitch_mm: 10.0,
            ..CardSettings::default()
        };
        assert!(svg(&s, Lang::En).unwrap_err().starts_with(TOO_BIG));
        let tall = CardSettings {
            rows: 40,
            ..CardSettings::default()
        };
        assert!(dots(&tall).unwrap_err().starts_with(TOO_BIG));
    }

    #[test]
    fn settings_that_make_no_sense_are_refused() {
        let d = CardSettings::default();
        for bad in [
            CardSettings { columns: 2, ..d },
            CardSettings { rows: 41, ..d },
            CardSettings {
                pitch_mm: f64::NAN,
                ..d
            },
            CardSettings { pitch_mm: 1.0, ..d },
            CardSettings { dot_mm: 9.0, ..d },
            CardSettings {
                margin_mm: -1.0,
                ..d
            },
            CardSettings {
                paper_w_mm: 10.0,
                ..d
            },
        ] {
            assert!(check(&bad).unwrap_err().starts_with(BAD_SETTING), "{bad:?}");
        }
    }

    #[test]
    fn the_bar_is_the_largest_round_length_that_fits_the_grid() {
        let s = CardSettings::default();
        assert_eq!(bar_mm(&s), Some(50.0));
        let small = CardSettings {
            columns: 3,
            pitch_mm: 4.0,
            dot_mm: 1.0,
            ..s
        };
        assert_eq!(bar_mm(&small), None);
    }
}
