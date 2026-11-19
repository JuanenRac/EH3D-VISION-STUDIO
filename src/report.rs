//! The report of a folder of pictures as one HTML file that stands alone: a table of the pictures with their numbers, a small drawing of each picture with a circle on every mark that
//! was found, and the list of the warnings (a picture that could not be used, a picture without marks, a mark that scores low). The file holds no script, no link and no other file:
//! the style and the drawings are in it, and its policy of content allows nothing from outside, so it can be mailed or opened anywhere. Everything that comes from a file name or from
//! a message is escaped.

use crate::batch::{Checked, NO_MARKS, OK};
use crate::lang::{Lang, Msg};

/// The most marks that are drawn on one picture (the table still counts all of them).
pub const MAX_MARKS_DRAWN: usize = 200;
/// The width in pixels at which each picture is drawn.
pub const THUMB_WIDTH: usize = 160;

/// A text as HTML text or as the value of an attribute.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if (c as u32) < 0x20 && c != '\n' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

fn number(v: Option<f64>, decimals: usize) -> String {
    v.map(|v| format!("{v:.decimals$}")).unwrap_or_default()
}

/// The small drawing of a picture: a grey frame of the proportions of the picture and a circle on every mark.
fn thumbnail(item: &Checked, label: &str) -> String {
    if item.width == 0 || item.height == 0 {
        return String::new();
    }
    let (w, h) = (item.width as f64, item.height as f64);
    let shown_h = (THUMB_WIDTH as f64 * h / w).round().clamp(8.0, 600.0);
    // the line is as thick as one pixel of the drawing, at least a pixel of the picture
    let line = (w / THUMB_WIDTH as f64).max(1.0);
    let mut svg = format!(
        "<svg viewBox=\"0 0 {} {}\" width=\"{THUMB_WIDTH}\" height=\"{shown_h}\" role=\"img\" aria-label=\"{}\"><rect width=\"{}\" height=\"{}\" class=\"frame\"/>",
        item.width,
        item.height,
        escape(label),
        item.width,
        item.height
    );
    for m in item.marks.iter().take(MAX_MARKS_DRAWN) {
        svg.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" stroke-width=\"{line:.1}\" class=\"mark\"/>",
            m.x,
            m.y,
            (m.diameter / 2.0).max(line)
        ));
    }
    svg.push_str("</svg>");
    svg
}

/// The warnings of a report: one sentence for each picture that could not be used, each picture without marks and each picture whose lowest score is below `low_score`.
pub fn warnings(items: &[Checked], lang: Lang, low_score: f64) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        let r = &item.row;
        if r.status == NO_MARKS {
            out.push(Msg::ReportWarnNoMarks.with(lang, &[&r.file]));
        } else if r.status != OK {
            out.push(Msg::ReportWarnFailed.with(lang, &[&r.file, &r.status, &r.detail]));
        } else if let Some(min) = r.min_score.filter(|m| *m < low_score) {
            out.push(Msg::ReportWarnLowScore.with(
                lang,
                &[&r.file, &format!("{min:.2}"), &format!("{low_score:.2}")],
            ));
        }
    }
    out
}

const STYLE: &str = ":root{--bg:#fbf8f3;--fg:#241d16;--dim:#6f6354;--line:#d9cfc0;--accent:#b36b00;--frame:#e9e1d3;--bad:#b3261e}@media (prefers-color-scheme:dark){:root{--bg:#171310;--fg:#f4ece0;--dim:#a99b88;--line:#3a3128;--accent:#ffb000;--frame:#2a231c;--bad:#ff6b7a}}body{margin:0 auto;max-width:1100px;padding:24px 16px;background:var(--bg);color:var(--fg);font:15px/1.5 system-ui,'Segoe UI','Noto Sans',sans-serif}h1{font-size:1.5rem;margin:0 0 4px}h2{font-size:1.1rem;margin:24px 0 8px}p.dim{color:var(--dim);margin:0 0 8px}ul.warn{margin:0;padding-left:20px}ul.warn li{color:var(--bad)}table{border-collapse:collapse;width:100%}th,td{border-bottom:1px solid var(--line);padding:6px 8px;text-align:left;vertical-align:middle}th{color:var(--dim);font-weight:600}td.n{text-align:right;font-variant-numeric:tabular-nums}.frame{fill:var(--frame)}.mark{fill:none;stroke:var(--accent)}td.bad{color:var(--bad)}@media (max-width:640px){body{padding:16px}th,td{padding:4px}}";

/// The report as the text of an HTML file, in the language. `low_score` is the score below which a picture gets a warning.
pub fn html(items: &[Checked], lang: Lang, low_score: f64) -> String {
    let with = items.iter().filter(|i| i.row.status == OK).count();
    let without = items.iter().filter(|i| i.row.status == NO_MARKS).count();
    let summary = Msg::ReportSummary.with(
        lang,
        &[
            items.len().to_string(),
            with.to_string(),
            without.to_string(),
            (items.len() - with - without).to_string(),
        ],
    );
    let title = Msg::ReportTitle.tr(lang);
    let mut out = format!(
        "<!doctype html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'\">\n<title>{}</title>\n<style>{STYLE}</style>\n</head>\n<body>\n<h1>{}</h1>\n<p class=\"dim\">{}</p>\n<p class=\"dim\">{}</p>\n",
        lang.code(),
        escape(title),
        escape(title),
        escape(&summary),
        escape(Msg::ReportLegend.tr(lang))
    );
    out.push_str(&format!(
        "<h2>{}</h2>\n",
        escape(Msg::ReportWarnings.tr(lang))
    ));
    let list = warnings(items, lang, low_score);
    if list.is_empty() {
        out.push_str(&format!(
            "<p class=\"dim\">{}</p>\n",
            escape(Msg::ReportNoWarnings.tr(lang))
        ));
    } else {
        out.push_str("<ul class=\"warn\">\n");
        for w in &list {
            out.push_str(&format!("<li>{}</li>\n", escape(w)));
        }
        out.push_str("</ul>\n");
    }
    out.push_str(&format!(
        "<table>\n<thead><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr></thead>\n<tbody>\n",
        escape(Msg::ReportPicture.tr(lang)),
        escape(Msg::ReportFile.tr(lang)),
        escape(Msg::ReportStatus.tr(lang)),
        escape(Msg::ReportMarks.tr(lang)),
        escape(Msg::ReportMeanScore.tr(lang)),
        escape(Msg::ReportMinScore.tr(lang)),
        escape(Msg::ReportDiameter.tr(lang)),
        escape(Msg::ReportDetail.tr(lang))
    ));
    for item in items {
        let r = &item.row;
        let class = if r.status == OK { "" } else { " class=\"bad\"" };
        out.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td{class}>{}</td><td class=\"n\">{}</td><td class=\"n\">{}</td><td class=\"n\">{}</td><td class=\"n\">{}</td><td>{}</td></tr>\n",
            thumbnail(item, &r.file),
            escape(&r.file),
            escape(&r.status),
            r.marks,
            number(r.mean_score, 3),
            number(r.min_score, 3),
            number(r.mean_diameter, 2),
            escape(&r.detail)
        ));
    }
    out.push_str("</tbody>\n</table>\n</body>\n</html>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batch::{check_detailed, Row};
    use crate::fiducial::{Fiducial, Settings};
    use crate::image::Gray;
    use crate::sim::to_pgm;
    use std::fs;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("eh3d_vision_report_{}_{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn item(file: &str, status: &str, marks: usize, min: Option<f64>) -> Checked {
        Checked {
            row: Row {
                file: file.into(),
                status: status.into(),
                marks,
                mean_score: min,
                min_score: min,
                mean_diameter: Some(20.0),
                detail: if status == OK || status == NO_MARKS {
                    String::new()
                } else {
                    "the reason".into()
                },
            },
            width: if marks > 0 { 200 } else { 0 },
            height: if marks > 0 { 100 } else { 0 },
            marks: (0..marks)
                .map(|i| Fiducial {
                    x: 30.0 + i as f64 * 50.0,
                    y: 50.0,
                    diameter: 20.0,
                    score: 0.95,
                })
                .collect(),
        }
    }

    #[test]
    fn what_comes_from_a_file_is_escaped() {
        assert_eq!(
            escape("<script>alert('x') & \"y\"</script>"),
            "&lt;script&gt;alert(&#39;x&#39;) &amp; &quot;y&quot;&lt;/script&gt;"
        );
        assert_eq!(escape("a\u{1}b"), "a b");
        let items = [item("<img src=x onerror=alert(1)>.pgm", OK, 1, Some(0.9))];
        let page = html(&items, Lang::En, 0.6);
        assert!(!page.contains("<img"), "{page}");
        assert!(page.contains("&lt;img src=x onerror=alert(1)&gt;.pgm"));
        assert!(!page.contains("<script"));
    }

    #[test]
    fn the_report_stands_alone_and_allows_nothing_from_outside() {
        let items = [
            item("a.pgm", OK, 2, Some(0.95)),
            item("b.pgm", NO_MARKS, 0, None),
        ];
        let page = html(&items, Lang::En, 0.6);
        assert!(page.starts_with("<!doctype html>\n<html lang=\"en\">"));
        assert!(page.contains("default-src 'none'; style-src 'unsafe-inline'"));
        for outside in [
            "http://", "https://", "src=", "href=", "<script", "<link", "@import", "url(",
        ] {
            assert!(!page.contains(outside), "{outside} in the report");
        }
        // the drawing of the first picture has a circle for each mark, the second none
        assert_eq!(page.matches("<circle").count(), 2);
        assert_eq!(page.matches("<svg").count(), 1);
        assert!(page.contains("viewBox=\"0 0 200 100\""));
        assert!(page.contains("<tr><td><svg"));
        assert!(page.contains("2 pictures: 1 with marks, 1 without, 0 that could not be used"));
    }

    #[test]
    fn the_warnings_name_the_pictures_that_are_not_well() {
        let items = [
            item("good.pgm", OK, 2, Some(0.95)),
            item("low.pgm", OK, 1, Some(0.4)),
            item("none.pgm", NO_MARKS, 0, None),
            item("bad.pgm", "picture_unreadable", 0, None),
        ];
        let w = warnings(&items, Lang::En, 0.6);
        assert_eq!(
            w,
            [
                "low.pgm: the lowest score is 0.40, below 0.60.",
                "none.pgm: no marks were found.",
                "bad.pgm: could not be used (picture_unreadable: the reason)."
            ]
        );
        assert!(warnings(&items[..1], Lang::En, 0.6).is_empty());
        let page = html(&items[..1], Lang::En, 0.6);
        assert!(page.contains("No warnings: every picture was read and has marks."));
        let page = html(&items, Lang::En, 0.6);
        assert!(page.contains("<ul class=\"warn\">") && page.matches("<li>").count() == 3);
        // a picture that could not be used is in red in the table
        assert!(page.contains("<td class=\"bad\">picture_unreadable</td>"));
    }

    #[test]
    fn the_report_speaks_each_of_the_eight_languages() {
        let items = [
            item("a.pgm", OK, 1, Some(0.5)),
            item("b.pgm", NO_MARKS, 0, None),
        ];
        let mut seen: Vec<String> = Vec::new();
        for lang in Lang::ALL {
            let page = html(&items, lang, 0.6);
            assert!(page.contains(&format!("<html lang=\"{}\">", lang.code())));
            assert!(
                page.contains(&escape(Msg::ReportTitle.tr(lang))),
                "{}",
                lang.code()
            );
            assert!(page.contains(&escape(Msg::ReportMinScore.tr(lang))));
            assert!(!page.contains("{0}") && !page.contains("{1}") && !page.contains("{2}"));
            assert!(!seen.contains(&page), "{} repeats a language", lang.code());
            seen.push(page);
        }
        assert!(html(&items, Lang::Ja, 0.6).contains("警告"));
    }

    #[test]
    fn at_most_the_marks_that_fit_are_drawn_and_a_flat_frame_is_kept() {
        let mut big = item("many.pgm", OK, 1, Some(0.9));
        big.marks = (0..MAX_MARKS_DRAWN + 50)
            .map(|i| Fiducial {
                x: (i % 100) as f64,
                y: (i / 100) as f64 * 10.0,
                diameter: 6.0,
                score: 0.9,
            })
            .collect();
        let page = html(&[big], Lang::En, 0.6);
        assert_eq!(page.matches("<circle").count(), MAX_MARKS_DRAWN);
        // a picture much wider than tall keeps its proportions
        let mut wide = item("wide.pgm", OK, 1, Some(0.9));
        wide.width = 1600;
        wide.height = 100;
        assert!(html(&[wide], Lang::En, 0.6).contains("height=\"10\""));
    }

    #[test]
    fn a_folder_read_from_disk_gives_a_report_with_the_marks_of_each_picture() {
        let dir = scratch("disk");
        let mut g = Gray::blank(200, 100, 220).unwrap();
        g.disc(50.0, 50.0, 10.0, 30);
        g.disc(120.0, 50.0, 10.0, 30);
        fs::write(dir.join("two.pgm"), to_pgm(&g)).unwrap();
        fs::write(dir.join("broken.pgm"), b"not a picture").unwrap();
        let items: Vec<Checked> = ["broken.pgm", "two.pgm"]
            .iter()
            .map(|n| check_detailed(&dir.join(n), 64, &Settings::default(), None, 0))
            .collect();
        assert_eq!(
            (items[1].width, items[1].height, items[1].marks.len()),
            (200, 100, 2)
        );
        assert_eq!((items[0].width, items[0].marks.len()), (0, 0));
        let page = html(&items, Lang::En, 0.6);
        assert_eq!(page.matches("<circle").count(), 2);
        assert!(page.contains("cx=\"50.0\" cy=\"50.0\""), "{page}");
        let _ = fs::remove_dir_all(&dir);
    }
}
