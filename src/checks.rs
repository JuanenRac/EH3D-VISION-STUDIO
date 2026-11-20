//! The panels of the window for the search by correlation, the straightened picture, the row of pads and the comparison of two pictures: what the commands `correlate`, `rectify`, `pads` and
//! `diff` do on the command line, with the values of the fields of the window. Each one is a method of `Window` that sets the status and keeps its result, so that all that the window decides is
//! tested without a screen; `gui.rs` only draws it. The files that are written are named after the open picture and are never written over a file that is there unless `replace_output` says so.

use crate::correlate::correlate;
use crate::diff::{compare, mask, to_csv as diff_csv};
use crate::image::from_bytes;
use crate::lang::Msg;
use crate::limit::{write_bytes, write_text};
use crate::pads::{measure_row, to_csv as pads_csv};
use crate::perspective::rectify;
use crate::sim::to_pgm;
use crate::window::{file_name, show, Loaded, Status, Window};
use std::path::Path;

/// The grey level of the places of a straightened picture that come from outside the picture.
const RECTIFY_FILL: u8 = 255;

impl Window {
    /// Slides a disc of the diameter of the field over the picture and keeps the places where the correlation is at least the least one of the field, as `eh3d-vision correlate` does.
    pub fn find_matches(&mut self) -> bool {
        if self.image.is_none() {
            self.status = Status::NeedImage;
            return false;
        }
        let settings = self.cfg.correlate_settings();
        match self.working().and_then(|gray| correlate(&gray, &settings)) {
            Ok(found) => {
                self.status = Status::MatchesFound(found.len());
                self.matches = found;
                true
            }
            Err(e) => {
                self.matches.clear();
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// One line for each place found by correlation: the number, the place in pixels and the score.
    pub fn match_lines(&self) -> Vec<String> {
        self.matches
            .iter()
            .enumerate()
            .map(|(i, m)| {
                Msg::MatchLine.with(
                    self.lang,
                    &[
                        (i + 1).to_string(),
                        show(m.x, 2),
                        show(m.y, 2),
                        show(m.score, 3),
                    ],
                )
            })
            .collect()
    }

    /// Draws the picture again as seen from straight above, taking the four marks that were found as the corners of a rectangle (top left, top right, bottom right, bottom left, however the
    /// marks are listed), and writes it as the PGM file `<picture>-rectified.pgm` beside the picture. The largest side of the result is `rectify_max_side`.
    pub fn rectify_picture(&mut self) -> bool {
        let Some((folder, stem)) = self.beside() else {
            self.status = Status::NeedImage;
            return false;
        };
        if self.marks.len() != 4 {
            self.status = Status::NeedFourMarks(self.marks.len());
            return false;
        }
        let mut places: Vec<(f64, f64)> = self.marks.iter().map(|m| self.mark_px(m)).collect();
        places.sort_by(|a, b| a.1.total_cmp(&b.1));
        let (mut top, mut bottom) = (places[..2].to_vec(), places[2..].to_vec());
        top.sort_by(|a, b| a.0.total_cmp(&b.0));
        bottom.sort_by(|a, b| a.0.total_cmp(&b.0));
        let corners = [top[0], top[1], bottom[1], bottom[0]];
        let max_side = self.cfg.rectify_max_side;
        let result = self
            .working()
            .and_then(|gray| rectify(&gray, &corners, None, RECTIFY_FILL, max_side));
        match result.and_then(|out| {
            let name = format!("{stem}-rectified.pgm");
            write_bytes(&folder.join(&name), &to_pgm(&out), self.cfg.replace_output).map(|()| name)
        }) {
            Ok(name) => {
                self.status = Status::Exported(name);
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Judges the parts that were measured as the pads of a row, as `eh3d-vision pads` does: the pitch, how much it varies and how far the pads lie from the row (`pads_tolerance`).
    pub fn judge_row(&mut self) -> bool {
        if self.image.is_none() {
            self.status = Status::NeedImage;
            return false;
        }
        if self.parts.is_empty() {
            self.status = Status::NeedParts;
            return false;
        }
        match measure_row(&self.parts, &self.cfg.pad_settings()) {
            Ok(row) => {
                self.status = if row.passes() {
                    Status::PadsRow {
                        count: row.count,
                        pitch: row.pitch_mm,
                    }
                } else {
                    Status::PadsRefused(row.reasons.clone())
                };
                self.row = Some(row);
                true
            }
            Err(e) => {
                self.row = None;
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// The numbers of the last row of pads, in the language of the window.
    pub fn row_line(&self) -> Option<String> {
        let row = self.row.as_ref()?;
        Some(Msg::PadsLine.with(
            self.lang,
            &[
                show(row.pitch_mm, 4),
                show(row.pitch_min_mm, 4),
                show(row.pitch_max_mm, 4),
                show(row.row_deviation_mm, 4),
                show(row.row_angle_deg, 2),
            ],
        ))
    }

    /// Writes the row as the CSV file `<picture>-pads.csv` beside the picture, the columns of `eh3d-vision pads`.
    pub fn save_row(&mut self) -> bool {
        let Some((folder, stem)) = self.beside() else {
            self.status = Status::NeedImage;
            return false;
        };
        let Some(row) = &self.row else {
            self.status = Status::NeedParts;
            return false;
        };
        let name = format!("{stem}-pads.csv");
        match write_text(&folder.join(&name), &pads_csv(row), self.cfg.replace_output) {
            Ok(()) => {
                self.status = Status::Exported(name);
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Opens the other picture of a comparison. It is only read: the open picture, its marks and its parts do not change.
    pub fn open_other(&mut self, path: &Path) -> bool {
        let name = file_name(path);
        let bytes = match self.read_limited(path) {
            Ok(b) => b,
            Err(s) => {
                self.status = s;
                return false;
            }
        };
        match from_bytes(&bytes) {
            Ok(gray) => {
                let (w, h) = (gray.w, gray.h);
                self.other = Some(Loaded {
                    name: name.clone(),
                    path: path.to_path_buf(),
                    gray,
                });
                self.changes = None;
                self.status = Status::OtherLoaded { name, w, h };
                true
            }
            Err(e) => {
                self.status = Status::Failed(format!("{name}: {e}"));
                false
            }
        }
    }

    /// Compares the open picture (before) with the other one (after) as `eh3d-vision diff` does, with the threshold, the least area and the share of the fields.
    pub fn compare_pictures(&mut self) -> bool {
        let Some(before) = self.image.as_ref() else {
            self.status = Status::NeedImage;
            return false;
        };
        let Some(after) = self.other.as_ref() else {
            self.status = Status::NeedOther;
            return false;
        };
        match compare(&before.gray, &after.gray, &self.cfg.diff_settings()) {
            Ok(d) => {
                let percent = d.changed_share * 100.0;
                self.status = if d.same {
                    Status::DiffSame { percent }
                } else {
                    Status::DiffChanged {
                        percent,
                        regions: d.regions,
                    }
                };
                self.changes = Some(d);
                true
            }
            Err(e) => {
                self.changes = None;
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// The numbers of the last comparison, in the language of the window.
    pub fn diff_line(&self) -> Option<String> {
        let d = self.changes.as_ref()?;
        Some(Msg::DiffLine.with(
            self.lang,
            &[
                d.changed_px.to_string(),
                d.total_px.to_string(),
                d.max_abs.to_string(),
            ],
        ))
    }

    /// Writes the comparison as the CSV file `<picture>-diff.csv` and the picture of the changes as `<picture>-changes.pgm`, both beside the picture.
    pub fn save_changes(&mut self) -> bool {
        let Some((folder, stem)) = self.beside() else {
            self.status = Status::NeedImage;
            return false;
        };
        let (Some(before), Some(after), Some(d)) = (
            self.image.as_ref(),
            self.other.as_ref(),
            self.changes.as_ref(),
        ) else {
            self.status = Status::NeedOther;
            return false;
        };
        let replace = self.cfg.replace_output;
        let name = format!("{stem}-diff.csv");
        let written = write_text(&folder.join(&name), &diff_csv(d), replace).and_then(|()| {
            let picture = mask(&before.gray, &after.gray, &self.cfg.diff_settings())?;
            write_bytes(
                &folder.join(format!("{stem}-changes.pgm")),
                &to_pgm(&picture),
                replace,
            )
        });
        match written {
            Ok(()) => {
                self.status = Status::Exported(name);
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::image::Gray;
    use crate::lang::Lang;
    use std::fs;
    use std::path::PathBuf;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let dir = std::env::temp_dir()
                .join(format!("eh3d_vision_checks_{}_{name}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn pgm(g: &Gray) -> Vec<u8> {
        let mut out = format!("P5\n{} {}\n255\n", g.w, g.h).into_bytes();
        out.extend_from_slice(&g.data);
        out
    }

    /// A light board with a dark disc of radius 10 at each place.
    fn discs(w: usize, h: usize, places: &[(f64, f64)]) -> Gray {
        let mut g = Gray::blank(w, h, 220).unwrap();
        for p in places {
            g.disc(p.0, p.1, 10.0, 20);
        }
        g
    }

    /// A light board with `count` dark pads of 10 by 22 pixels in a row, `pitch` pixels apart; `moved` shifts one of them along the row.
    fn pads(count: usize, pitch: usize, moved: Option<(usize, usize)>) -> Gray {
        let (w, h) = (140usize, 60usize);
        let mut g = Gray::blank(w, h, 220).unwrap();
        for k in 0..count {
            let x0 = 8 + k * pitch + moved.filter(|m| m.0 == k).map_or(0, |m| m.1);
            for y in 19..41 {
                for x in x0..x0 + 10 {
                    g.data[y * w + x] = 30;
                }
            }
        }
        g
    }

    const FOUR: [(f64, f64); 4] = [(60.0, 60.0), (240.0, 60.0), (240.0, 140.0), (60.0, 140.0)];

    fn window() -> Window {
        Window::new(Lang::En, Config::default(), None)
    }

    fn opened(s: &Scratch, name: &str, g: &Gray) -> Window {
        let path = s.path(name);
        fs::write(&path, pgm(g)).unwrap();
        let mut w = window();
        assert!(w.open_image(&path));
        w
    }

    #[test]
    fn the_correlation_finds_the_discs_and_lists_them() {
        let s = Scratch::new("matches");
        let mut w = opened(
            &s,
            "two.pgm",
            &discs(300, 200, &[(80.0, 70.0), (200.0, 120.0)]),
        );
        assert!(w.set_field(field(&w, "correlate_diameter"), "20".into()));
        assert!(w.find_matches());
        assert_eq!(w.status, Status::MatchesFound(2));
        let lines = w.match_lines();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("1: ("), "{lines:?}");
        assert!(lines[0].contains("score"));
        // nothing to find in a flat picture, and a picture is needed
        let mut flat = opened(&s, "flat.pgm", &Gray::blank(100, 100, 120).unwrap());
        assert!(flat.find_matches());
        assert_eq!(flat.status, Status::MatchesFound(0));
        assert!(flat.match_lines().is_empty());
        let mut none = window();
        assert!(!none.find_matches());
        assert_eq!(none.status, Status::NeedImage);
    }

    #[test]
    fn a_disc_that_does_not_fit_the_picture_is_a_failure_and_clears_the_old_list() {
        let s = Scratch::new("matches_bad");
        let mut w = opened(&s, "small.pgm", &discs(300, 200, &[(80.0, 70.0)]));
        assert!(w.find_matches());
        assert!(!w.matches.is_empty());
        // a disc larger than the picture
        let tiny_path = s.path("tiny.pgm");
        fs::write(&tiny_path, pgm(&Gray::blank(10, 10, 100).unwrap())).unwrap();
        assert!(w.open_image(&tiny_path));
        assert!(!w.find_matches());
        assert!(matches!(w.status, Status::Failed(_)), "{:?}", w.status);
        assert!(w.matches.is_empty());
    }

    #[test]
    fn four_marks_are_straightened_into_a_file_beside_the_picture() {
        let s = Scratch::new("rectify");
        let mut w = opened(&s, "four.pgm", &discs(300, 200, &FOUR));
        assert_eq!(w.marks.len(), 4);
        assert!(w.rectify_picture());
        assert_eq!(w.status, Status::Exported("four-rectified.pgm".into()));
        let bytes = fs::read(s.path("four-rectified.pgm")).unwrap();
        // the rectangle of the marks is about 180 by 80 pixels, and the corners are the centres of the first and the last pixel
        let header = String::from_utf8_lossy(&bytes[..14]).to_string();
        let mut words = header.split_whitespace().skip(1);
        let (cols, rows): (usize, usize) = (
            words.next().unwrap().parse().unwrap(),
            words.next().unwrap().parse().unwrap(),
        );
        assert!(
            (180..=182).contains(&cols) && (80..=82).contains(&rows),
            "{header}"
        );
        assert_eq!(bytes.len(), 14 + cols * rows);
        // never over a file that is there, unless replace_output says so
        assert!(!w.rectify_picture());
        assert!(matches!(w.status, Status::Failed(_)));
        w.cfg.replace_output = true;
        assert!(w.rectify_picture());
    }

    #[test]
    fn straightening_needs_exactly_four_marks_and_a_picture() {
        let s = Scratch::new("rectify_few");
        let mut w = opened(&s, "three.pgm", &discs(300, 200, &FOUR[..3]));
        assert!(!w.rectify_picture());
        assert_eq!(w.status, Status::NeedFourMarks(3));
        assert!(!s.path("three-rectified.pgm").exists());
        let mut none = window();
        assert!(!none.rectify_picture());
        assert_eq!(none.status, Status::NeedImage);
    }

    #[test]
    fn marks_listed_in_any_order_give_the_same_picture() {
        let s = Scratch::new("rectify_order");
        let mut a = opened(&s, "a.pgm", &discs(300, 200, &FOUR));
        let mut b = opened(&s, "b.pgm", &discs(300, 200, &FOUR));
        b.marks.reverse();
        assert!(a.rectify_picture() && b.rectify_picture());
        let fa = fs::read(s.path("a-rectified.pgm")).unwrap();
        let fb = fs::read(s.path("b-rectified.pgm")).unwrap();
        assert_eq!(fa, fb);
    }

    #[test]
    fn a_row_of_pads_is_judged_and_saved() {
        let s = Scratch::new("row");
        let mut w = opened(&s, "row.pgm", &pads(6, 15, None));
        assert!(w.set_field(field(&w, "mm_per_px"), "0.04".into()));
        assert!(!w.judge_row());
        assert_eq!(w.status, Status::NeedParts);
        assert!(w.measure());
        assert_eq!(w.parts.len(), 6);
        assert!(w.judge_row());
        match &w.status {
            Status::PadsRow { count, pitch } => {
                assert_eq!(*count, 6);
                assert!((pitch - 0.6).abs() < 0.001, "{pitch}");
            }
            other => panic!("{other:?}"),
        }
        assert!(w.row_line().unwrap().starts_with("Pitch 0.6"));
        assert!(w.save_row());
        let csv = fs::read_to_string(s.path("row-pads.csv")).unwrap();
        assert!(csv.lines().nth(1).unwrap().ends_with("pass,"), "{csv}");
        assert!(!w.save_row());
    }

    #[test]
    fn a_pad_out_of_place_refuses_the_row_with_the_reason() {
        let s = Scratch::new("row_bad");
        let mut w = opened(&s, "row.pgm", &pads(6, 15, Some((3, 6))));
        assert!(w.set_field(field(&w, "mm_per_px"), "0.04".into()));
        assert!(w.measure());
        assert!(w.judge_row());
        match &w.status {
            Status::PadsRefused(why) => assert!(why.contains(&"pitch"), "{why:?}"),
            other => panic!("{other:?}"),
        }
        assert!(w.status.is_problem());
        // two pads are not a row
        let mut two = opened(&s, "two.pgm", &pads(2, 15, None));
        assert!(two.set_field(field(&two, "mm_per_px"), "0.04".into()));
        assert!(two.measure());
        assert!(!two.judge_row());
        assert!(matches!(two.status, Status::Failed(_)));
        assert!(two.row_line().is_none());
        let mut none = window();
        assert!(!none.judge_row());
        assert!(!none.save_row());
    }

    #[test]
    fn two_pictures_are_compared_and_the_changes_are_saved() {
        let s = Scratch::new("diff");
        let before = discs(120, 80, &[(30.0, 30.0)]);
        let same = before.clone();
        let after = discs(120, 80, &[(30.0, 30.0), (80.0, 50.0)]);
        let mut w = opened(&s, "before.pgm", &before);
        assert!(!w.compare_pictures());
        assert_eq!(w.status, Status::NeedOther);
        fs::write(s.path("same.pgm"), pgm(&same)).unwrap();
        assert!(w.open_other(&s.path("same.pgm")));
        assert_eq!(
            w.status,
            Status::OtherLoaded {
                name: "same.pgm".into(),
                w: 120,
                h: 80
            }
        );
        assert!(w.compare_pictures());
        assert!(
            matches!(w.status, Status::DiffSame { .. }),
            "{:?}",
            w.status
        );
        assert!(!w.status.is_problem());
        fs::write(s.path("after.pgm"), pgm(&after)).unwrap();
        assert!(w.open_other(&s.path("after.pgm")));
        assert!(w.changes.is_none());
        assert!(w.compare_pictures());
        match &w.status {
            Status::DiffChanged { percent, regions } => {
                assert!(*percent > 1.0, "{percent}");
                assert_eq!(*regions, 1);
            }
            other => panic!("{other:?}"),
        }
        assert!(w.status.is_problem());
        assert!(w.diff_line().unwrap().contains("pixels changed"));
        assert!(w.save_changes());
        assert_eq!(w.status, Status::Exported("before-diff.csv".into()));
        assert!(fs::read_to_string(s.path("before-diff.csv"))
            .unwrap()
            .contains(",changed"));
        let mask = fs::read(s.path("before-changes.pgm")).unwrap();
        assert!(mask.starts_with(b"P5\n120 80\n255\n"));
        // not over files that are there
        assert!(!w.save_changes());
    }

    #[test]
    fn pictures_of_different_sizes_cannot_be_compared() {
        let s = Scratch::new("diff_size");
        let mut w = opened(&s, "a.pgm", &discs(120, 80, &[(30.0, 30.0)]));
        fs::write(s.path("b.pgm"), pgm(&discs(100, 80, &[(30.0, 30.0)]))).unwrap();
        assert!(w.open_other(&s.path("b.pgm")));
        assert!(!w.compare_pictures());
        assert!(
            matches!(&w.status, Status::Failed(e) if e.contains("size_mismatch")),
            "{:?}",
            w.status
        );
        assert!(w.changes.is_none() && w.diff_line().is_none());
        assert!(!w.save_changes());
        // a file that is not a picture
        fs::write(s.path("bad.pgm"), b"not a picture").unwrap();
        assert!(!w.open_other(&s.path("bad.pgm")));
        assert!(!w.open_other(&s.path("missing.pgm")));
        let mut none = window();
        assert!(!none.compare_pictures());
        assert_eq!(none.status, Status::NeedImage);
    }

    #[test]
    fn a_new_picture_forgets_the_old_results_of_these_panels() {
        let s = Scratch::new("forget");
        let mut w = opened(&s, "row.pgm", &pads(6, 15, None));
        assert!(w.set_field(field(&w, "mm_per_px"), "0.04".into()));
        assert!(w.find_matches());
        assert!(w.measure() && w.judge_row());
        fs::write(s.path("o.pgm"), pgm(&pads(6, 15, None))).unwrap();
        assert!(w.open_other(&s.path("o.pgm")) && w.compare_pictures());
        assert!(w.row.is_some() && w.changes.is_some());
        let path = s.path("row.pgm");
        assert!(w.open_image(&path));
        assert!(w.matches.is_empty() && w.row.is_none() && w.changes.is_none());
        // the other picture stays: it is the one to compare with
        assert!(w.other.is_some());
    }

    #[test]
    fn the_fields_of_the_panels_take_the_values_that_make_sense_only() {
        let mut w = window();
        for (key, bad, good) in [
            ("correlate_diameter", "3", "16"),
            ("correlate_min_score", "1.5", "0.7"),
            ("rectify_max_side", "1", "2048"),
            ("pads_tolerance", "0", "0.1"),
            ("diff_threshold", "255", "40"),
            ("diff_min_area", "0", "9"),
            ("diff_max_changed", "2", "0.05"),
        ] {
            let i = field(&w, key);
            assert!(!w.set_field(i, bad.into()), "{key} {bad}");
            assert!(matches!(w.status, Status::BadValue(_)), "{key}");
            assert!(w.set_field(i, good.into()), "{key} {good}");
        }
        assert_eq!(w.cfg.diff_threshold, 40);
        assert_eq!(w.cfg.rectify_max_side, 2048);
    }

    #[test]
    fn every_status_of_the_panels_is_said_in_the_eight_languages() {
        let statuses = [
            Status::MatchesFound(3),
            Status::MatchesFound(0),
            Status::NeedFourMarks(2),
            Status::PadsRow {
                count: 6,
                pitch: 0.5,
            },
            Status::PadsRefused(vec!["pitch", "row"]),
            Status::OtherLoaded {
                name: "x.pgm".into(),
                w: 10,
                h: 20,
            },
            Status::NeedOther,
            Status::DiffSame { percent: 0.0 },
            Status::DiffChanged {
                percent: 12.5,
                regions: 4,
            },
        ];
        for st in &statuses {
            let english = st.text(Lang::En);
            assert!(!english.contains('{'), "{st:?}: {english}");
            for l in Lang::ALL {
                let t = st.text(l);
                assert!(
                    !t.trim().is_empty() && !t.contains('{'),
                    "{st:?} in {}: {t}",
                    l.code()
                );
                if l != Lang::En {
                    assert_ne!(t, english, "{st:?} in {}", l.code());
                }
            }
        }
        assert!(Status::PadsRefused(vec!["pitch", "row"])
            .text(Lang::Pt)
            .contains("passo"));
        assert!(Status::NeedFourMarks(2).text(Lang::De).contains('2'));
    }

    /// The index of the field of a key.
    fn field(w: &Window, key: &str) -> usize {
        w.fields.iter().position(|f| f.key == key).unwrap()
    }
}
