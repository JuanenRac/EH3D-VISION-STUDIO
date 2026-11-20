//! The card of calibration to print: the board as a vector drawing whose units are millimetres, so that printed at 100 % a square measures what the design says. The guide printed on the
//! card says in the language asked for to print without fitting to the page and to measure the distance between the two outer corners of a row, which is the number that the calibration
//! is checked against.

use super::{BoardSpec, Dictionary, BITS, CELLS, MARKER_SHARE};
use crate::lang::{Lang, Msg};

/// The white margin round the board, in millimetres.
pub const MARGIN_MM: f64 = 6.0;

fn num(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The drawing of the board, with its margin and the guide in the language.
pub fn svg(spec: &BoardSpec, dict: &Dictionary, lang: Lang) -> Result<String, String> {
    spec.check()?;
    if dict.codes.len() < spec.marker_count() {
        return Err("the dictionary has fewer markers than the board".into());
    }
    let (bw, bh) = spec.size_mm();
    let (w, h) = (bw + 2.0 * MARGIN_MM, bh + 2.0 * MARGIN_MM + 5.0);
    let s = spec.square_mm;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}mm\" height=\"{}mm\" viewBox=\"0 0 {} {}\">\n",
        num(w),
        num(h),
        num(w),
        num(h)
    ));
    out.push_str(&format!(
        "<title>{} x {} squares of {} mm</title>\n",
        spec.cols,
        spec.rows,
        num(s)
    ));
    out.push_str(&format!(
        "<rect width=\"{}\" height=\"{}\" fill=\"#fff\"/>\n",
        num(w),
        num(h)
    ));
    out.push_str(&format!(
        "<g transform=\"translate({0} {0})\" shape-rendering=\"crispEdges\">\n",
        num(MARGIN_MM)
    ));
    for j in 0..spec.rows {
        for i in 0..spec.cols {
            if (i + j) % 2 == 0 {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#000\"/>\n",
                    num(i as f64 * s),
                    num(j as f64 * s),
                    num(s),
                    num(s)
                ));
            } else if let Some(id) = spec.marker_id(i, j) {
                let side = MARKER_SHARE * s;
                let (x0, y0) = (
                    (i as f64 + 0.5) * s - side / 2.0,
                    (j as f64 + 0.5) * s - side / 2.0,
                );
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#000\"/>\n",
                    num(x0),
                    num(y0),
                    num(side),
                    num(side)
                ));
                let cell = side / CELLS as f64;
                for r in 0..BITS {
                    for c in 0..BITS {
                        if (dict.codes[id] >> (r * BITS + c)) & 1 == 1 {
                            out.push_str(&format!(
                                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#fff\"/>\n",
                                num(x0 + (c + 1) as f64 * cell),
                                num(y0 + (r + 1) as f64 * cell),
                                num(cell),
                                num(cell)
                            ));
                        }
                    }
                }
            }
        }
    }
    out.push_str("</g>\n");
    let guide = Msg::CardGuide.with(lang, &[num(s), num((spec.cols as f64 - 2.0) * s)]);
    let guide = guide
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"2\" fill=\"#000\">{}</text>\n",
        num(MARGIN_MM),
        num(h - 2.0),
        guide
    ));
    out.push_str("</svg>\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(lang: Lang) -> String {
        let spec = BoardSpec::default();
        svg(&spec, &Dictionary::for_board(&spec).unwrap(), lang).unwrap()
    }

    #[test]
    fn the_card_is_drawn_in_millimetres_at_the_size_of_the_board_and_its_margin() {
        let t = card(Lang::En);
        // 8 squares of 5 mm and 2 margins of 6 mm; 6 squares and the line of the guide
        assert!(
            t.contains("width=\"52mm\" height=\"47mm\""),
            "{}",
            &t[..200]
        );
        assert!(t.contains("viewBox=\"0 0 52 47\""));
        assert!(t.starts_with("<svg ") && t.trim_end().ends_with("</svg>"));
        assert_eq!(t.matches("http://").count(), 1, "only the namespace");
    }

    #[test]
    fn it_has_the_black_squares_and_the_markers_of_the_board() {
        let t = card(Lang::En);
        let black_squares = t.matches("width=\"5\" height=\"5\" fill=\"#000\"").count();
        assert_eq!(black_squares, 24);
        let dict = Dictionary::for_board(&BoardSpec::default()).unwrap();
        let ones: u32 = dict.codes.iter().map(|c| c.count_ones()).sum();
        assert_eq!(
            t.matches("fill=\"#fff\"/>").count() as u32,
            ones + 1,
            "the white cells and the paper"
        );
        assert_eq!(
            t.matches("width=\"3\" height=\"3\" fill=\"#000\"").count(),
            24
        );
    }

    #[test]
    fn the_same_board_gives_the_same_card_and_the_guide_follows_the_language() {
        assert_eq!(card(Lang::Es), card(Lang::Es));
        let mut seen = std::collections::HashSet::new();
        for l in Lang::ALL {
            let t = card(l);
            assert!(t.contains("5"), "{}", l.code());
            seen.insert(
                t.lines()
                    .find(|x| x.starts_with("<text"))
                    .unwrap()
                    .to_string(),
            );
        }
        assert_eq!(seen.len(), 8, "eight different guides");
        assert!(card(Lang::Pt).contains("régua"));
    }

    #[test]
    fn the_guide_gives_the_square_and_the_distance_to_measure() {
        let t = card(Lang::En);
        assert!(t.contains("5 mm") && t.contains("30 mm"), "{t}");
    }

    #[test]
    fn a_dictionary_that_is_too_short_or_a_board_that_cannot_be_made_is_refused() {
        let spec = BoardSpec::default();
        let short = Dictionary::new(1, 3).unwrap();
        assert!(svg(&spec, &short, Lang::En).is_err());
        let bad = BoardSpec { cols: 1, ..spec };
        assert!(svg(&bad, &Dictionary::new(1, 3).unwrap(), Lang::En).is_err());
    }
}
