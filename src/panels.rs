//! The panels of the window for the lens, the parts against their packages and the pictures that a big part needs. Each one is a method of `Window` that sets the status and keeps its result,
//! so that all that the window decides is tested without a screen; `gui.rs` only draws it. The files that are written are named after the open picture and are never written over a file that
//! is there unless `replace_output` says so.

use crate::inspect::{
    find_package, judge, parse_table, to_csv as inspect_csv, REASON_ANGLE, REASON_LENGTH,
};
use crate::lang::Msg;
use crate::limit::{read_text_limited, write_text};
use crate::measure::measure_parts;
use crate::offset::{
    mirror_parts, pick_offsets, to_table_csv as offsets_csv, REASON_OFFSET, REASON_TURN,
};
use crate::quality;
use crate::shots::{plan, to_csv as shots_csv};
use crate::window::{show, Status, Window};
use std::path::Path;

impl Window {
    /// Writes the two factors of the lens as the file `<picture>-lens.cfg` beside the picture, in the lines of `vision.cfg` (`lens_k1 = ...` and `lens_k2 = ...`).
    pub fn save_lens(&mut self) -> bool {
        let Some((folder, stem)) = self.beside() else {
            self.status = Status::NeedImage;
            return false;
        };
        let Some(lens) = self.distortion else {
            self.status = Status::NeedLens;
            return false;
        };
        let name = format!("{stem}-lens.cfg");
        let text = format!("lens_k1 = {:.6}\nlens_k2 = {:.6}\n", lens.k1, lens.k2);
        match write_text(&folder.join(&name), &text, self.cfg.replace_output) {
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

    /// Reads the table of packages named by `package_table` (within `max_file_mib`): a table that cannot be read or has a bad line is the status, with its file and line.
    pub fn load_packages(&mut self) -> bool {
        let file = self.cfg.package_table.trim().to_string();
        if file.is_empty() {
            self.status = Status::NeedTable;
            return false;
        }
        let table = read_text_limited(Path::new(&file), self.cfg.max_file_mib)
            .and_then(|text| parse_table(&text).map_err(|e| format!("{file}: {e}")));
        match table {
            Ok(table) => {
                self.status = Status::PackagesLoaded(table.len());
                self.packages = table;
                self.judged.clear();
                self.judged_package = None;
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Judges the parts that were measured against the package `inspect_package` of the table, as `eh3d-vision inspect` does: the length and the width within the tolerance (`inspect_tolerance`,
    /// or the one of the package) and, when `inspect_max_angle` is above 0, the turn within it. The table is read again each time, so that a change of the file counts.
    pub fn inspect_parts(&mut self) -> bool {
        if self.image.is_none() {
            self.status = Status::NeedImage;
            return false;
        }
        if self.parts.is_empty() {
            self.status = Status::NeedParts;
            return false;
        }
        if !self.load_packages() {
            return false;
        }
        let name = self.cfg.inspect_package.trim().to_string();
        if name.is_empty() {
            self.status = Status::NeedPackage;
            return false;
        }
        let package = match find_package(&self.packages, &name) {
            Ok(p) => p.clone(),
            Err(e) => {
                self.status = Status::Failed(e);
                return false;
            }
        };
        match judge(
            &self.parts,
            &package,
            self.cfg.inspect_tolerance,
            self.cfg.inspect_max_angle,
        ) {
            Ok(judged) => {
                let pass = judged.iter().filter(|j| j.passes()).count();
                self.status = Status::InspectDone {
                    pass,
                    fail: judged.len() - pass,
                };
                self.judged = judged;
                self.judged_package = Some(package);
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// One line per part with its verdict (true when it passes) and, for a part that fails, why (length, width, turn), in the language of the window.
    pub fn inspect_lines(&self) -> Vec<(String, bool)> {
        self.judged
            .iter()
            .enumerate()
            .map(|(i, j)| {
                let (n, l, w) = (
                    (i + 1).to_string(),
                    show(j.part.length_mm, 3),
                    show(j.part.width_mm, 3),
                );
                if j.passes() {
                    (Msg::InspectPass.with(self.lang, &[&n, &l, &w]), true)
                } else {
                    let why: Vec<&str> = j
                        .reasons
                        .iter()
                        .map(|r| match *r {
                            REASON_LENGTH => Msg::ReasonLength.tr(self.lang),
                            REASON_ANGLE => Msg::ReasonAngle.tr(self.lang),
                            _ => Msg::ReasonWidth.tr(self.lang),
                        })
                        .collect();
                    (
                        Msg::InspectFail.with(self.lang, &[&n, &why.join(", "), &l, &w]),
                        false,
                    )
                }
            })
            .collect()
    }

    /// Writes the verdicts as the CSV file `<picture>-inspection.csv` beside the picture, the columns of `eh3d-vision inspect`.
    pub fn save_inspection(&mut self) -> bool {
        let Some((folder, stem)) = self.beside() else {
            self.status = Status::NeedImage;
            return false;
        };
        let Some(package) = self
            .judged_package
            .clone()
            .filter(|_| !self.judged.is_empty())
        else {
            self.status = Status::NeedParts;
            return false;
        };
        let name = format!("{stem}-inspection.csv");
        match write_text(
            &folder.join(&name),
            &inspect_csv(&self.judged, &package),
            self.cfg.replace_output,
        ) {
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

    /// Works out how each part of the picture is held, as `eh3d-vision offset --each` does: the parts are measured from the picture (with the picture check first when `quality_check` is on, and
    /// turned over left to right when `pick_mirror` is on), each is judged against the package `inspect_package` of the table, and for each one the offset from the axis of the nozzle, the
    /// turn from `nominal_angle_deg` and the correction that puts it right are kept. The table is read again each time.
    pub fn offset_parts(&mut self) -> bool {
        if self.refused_for_the_configuration() {
            self.held.clear();
            self.held_package = None;
            return false;
        }
        if self.image.is_none() {
            self.status = Status::NeedImage;
            return false;
        }
        let Some(scale) = self.measure_scale() else {
            self.status = Status::NeedScale;
            return false;
        };
        if !self.load_packages() {
            return false;
        }
        let name = self.cfg.inspect_package.trim().to_string();
        if name.is_empty() {
            self.status = Status::NeedPackage;
            return false;
        }
        let package = match find_package(&self.packages, &name) {
            Ok(p) => p.clone(),
            Err(e) => {
                self.status = Status::Failed(e);
                return false;
            }
        };
        let settings = self.part_settings();
        let check = self.cfg.quality_check.then(|| self.cfg.quality_settings());
        // the picture is checked as it was taken, before the light is flattened
        let raw_check = match (&check, self.image.as_ref()) {
            (Some(q), Some(img)) => quality::require_good(&img.gray, q).map(|_| ()),
            _ => Ok(()),
        };
        let measured = raw_check.and_then(|()| {
            self.working()
                .and_then(|g| measure_parts(&g, &settings, scale))
        });
        self.held.clear();
        self.held_package = None;
        let mut parts = match measured {
            Ok(p) => p,
            Err(e) => {
                self.status = Status::Failed(e);
                return false;
            }
        };
        if parts.is_empty() {
            self.status = Status::PartsFound(0);
            return false;
        }
        if self.cfg.pick_mirror {
            parts = mirror_parts(&parts);
        }
        match pick_offsets(&parts, &package, &self.cfg.offset_settings()) {
            Ok(held) => {
                let pass = held.iter().filter(|o| o.passes()).count();
                self.status = Status::OffsetDone {
                    pass,
                    fail: held.len() - pass,
                };
                self.held = held;
                self.held_package = Some(package);
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// One line per part with its correction and, for a part that fails, why (length, width, too far from the nozzle, turned too much), in the language of the window; true when it passes.
    pub fn offset_lines(&self) -> Vec<(String, bool)> {
        self.held
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let (cx, cy, cr) = o.correction();
                let (n, x, y, r) = ((i + 1).to_string(), show(cx, 3), show(cy, 3), show(cr, 2));
                if o.passes() {
                    (Msg::OffsetPass.with(self.lang, &[&n, &x, &y, &r]), true)
                } else {
                    let why: Vec<&str> = o
                        .reasons
                        .iter()
                        .map(|reason| match *reason {
                            REASON_LENGTH => Msg::ReasonLength.tr(self.lang),
                            REASON_OFFSET => Msg::ReasonOffset.tr(self.lang),
                            REASON_TURN => Msg::ReasonTurn.tr(self.lang),
                            _ => Msg::ReasonWidth.tr(self.lang),
                        })
                        .collect();
                    (
                        Msg::OffsetFail.with(self.lang, &[&n, &why.join(", "), &x, &y, &r]),
                        false,
                    )
                }
            })
            .collect()
    }

    /// Writes the table of corrections as the CSV file `<picture>-offsets.csv` beside the picture, the columns of `eh3d-vision offset --each`.
    pub fn save_offsets(&mut self) -> bool {
        let Some((folder, stem)) = self.beside() else {
            self.status = Status::NeedImage;
            return false;
        };
        let Some(package) = self.held_package.clone().filter(|_| !self.held.is_empty()) else {
            self.status = Status::NeedParts;
            return false;
        };
        let name = format!("{stem}-offsets.csv");
        match write_text(
            &folder.join(&name),
            &offsets_csv(&self.held, &package),
            self.cfg.replace_output,
        ) {
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

    /// Measures the sharpness, the contrast and the mean grey level of the open picture and says whether it is good enough to measure from, with the values of the panel (the picture as it
    /// was opened, not with the light flattened).
    pub fn check_quality(&mut self) -> bool {
        let Some(img) = self.image.as_ref() else {
            self.status = Status::NeedImage;
            return false;
        };
        let result = quality::measure(&img.gray)
            .and_then(|q| quality::reasons(&q, &self.cfg.quality_settings()).map(|why| (q, why)));
        match result {
            Ok((q, why)) => {
                self.quality = Some(q);
                self.status = if why.is_empty() {
                    Status::QualityGood
                } else {
                    Status::QualityPoor(why)
                };
                true
            }
            Err(e) => {
                self.quality = None;
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// The numbers of the last check of the picture, in the language of the window.
    pub fn quality_line(&self) -> Option<String> {
        let q = self.quality?;
        Some(Msg::QualityLine.with(
            self.lang,
            &[show(q.sharpness, 2), show(q.contrast, 2), show(q.mean, 2)],
        ))
    }

    /// The size in millimetres of the part the pictures are planned for: `shots_part_width` by `shots_part_height` when both are above 0, else the body of the package `inspect_package`
    /// of the table.
    fn shots_part(&mut self) -> Result<(f64, f64), Status> {
        if self.cfg.shots_part_width > 0.0 && self.cfg.shots_part_height > 0.0 {
            return Ok((self.cfg.shots_part_width, self.cfg.shots_part_height));
        }
        if self.cfg.inspect_package.trim().is_empty() {
            return Err(Status::NeedBody);
        }
        if !self.load_packages() {
            return Err(self.status.clone());
        }
        let name = self.cfg.inspect_package.trim().to_string();
        find_package(&self.packages, &name)
            .map(|p| (p.long_mm, p.short_mm))
            .map_err(Status::Failed)
    }

    /// Plans the pictures a part bigger than the field of the camera needs, as `eh3d-vision shots` does: the field is the open picture in millimetres (its size in pixels times the scale),
    /// the part is the size of the configuration or the body of the package, and `shots_overlap` and `shots_margin` are the values of the panel.
    pub fn plan_shots(&mut self) -> bool {
        let Some(img) = self.image.as_ref() else {
            self.status = Status::NeedImage;
            return false;
        };
        let (w_px, h_px) = (img.gray.w as f64, img.gray.h as f64);
        let Some(scale) = self.measure_scale() else {
            self.status = Status::NeedScale;
            return false;
        };
        let (width, height) = match self.shots_part() {
            Ok(size) => size,
            Err(status) => {
                self.status = status;
                return false;
            }
        };
        match plan(
            width,
            height,
            w_px * scale.mm_per_px,
            h_px * scale.mm_per_px,
            self.cfg.shots_overlap,
            self.cfg.shots_margin,
        ) {
            Ok(p) => {
                self.status = Status::ShotsPlanned {
                    pictures: p.shots.len(),
                    columns: p.columns,
                    rows: p.rows,
                };
                self.plan = Some(p);
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// The lines of the plan of pictures: where the camera goes for each one, in millimetres from the centre of the part.
    pub fn shots_lines(&self) -> Vec<String> {
        self.plan.as_ref().map_or_else(Vec::new, |p| {
            p.shots
                .iter()
                .enumerate()
                .map(|(i, (x, y))| {
                    Msg::ShotLine.with(self.lang, &[(i + 1).to_string(), show(*x, 3), show(*y, 3)])
                })
                .collect()
        })
    }

    /// Writes the plan as the CSV file `<picture>-shots.csv` beside the picture, the columns of `eh3d-vision shots`.
    pub fn save_shots(&mut self) -> bool {
        let Some((folder, stem)) = self.beside() else {
            self.status = Status::NeedImage;
            return false;
        };
        let Some(plan) = &self.plan else {
            self.status = Status::NeedBody;
            return false;
        };
        let name = format!("{stem}-shots.csv");
        match write_text(
            &folder.join(&name),
            &shots_csv(plan),
            self.cfg.replace_output,
        ) {
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
    use crate::lens::{distort2, fit_lens};
    use std::fs;
    use std::path::PathBuf;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let dir = std::env::temp_dir()
                .join(format!("eh3d-vision-panels-{}-{tag}", std::process::id()));
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

    fn window_with(cfg: &str) -> Window {
        Window::new(Lang::En, Config::parse(cfg).unwrap(), None)
    }

    /// A picture of 640 by 480 with a grid of 5 by 4 dark discs 100 pixels apart seen through a lens with the factors, saved as `lens.pgm`.
    fn lens_picture(s: &Scratch, k1: f64, k2: f64) -> PathBuf {
        let (w, h) = (640usize, 480usize);
        let centre = (w as f64 / 2.0, h as f64 / 2.0);
        let norm = centre.0.hypot(centre.1);
        let mut g = Gray::blank(w, h, 220).unwrap();
        for r in 0..4 {
            for c in 0..5 {
                let ideal = (
                    centre.0 + (c as f64 - 2.0) * 100.0,
                    centre.1 + (r as f64 - 1.5) * 100.0,
                );
                let m = distort2(ideal, centre, k1, k2, norm);
                g.disc(m.0, m.1, 10.0, 30);
            }
        }
        let path = s.path("lens.pgm");
        fs::write(&path, pgm(&g)).unwrap();
        path
    }

    #[test]
    fn the_two_factors_of_the_lens_come_from_the_grid_in_the_picture() {
        let s = Scratch::new("fit");
        let path = lens_picture(&s, -0.06, 0.02);
        let mut w = window_with("");
        assert!(w.open_image(&path));
        assert_eq!(w.marks.len(), 20);
        assert!(w.fit_grid());
        let d = w.distortion.unwrap();
        assert!(
            (d.k1 + 0.06).abs() < 0.01 && (d.k2 - 0.02).abs() < 0.01,
            "{d:?}"
        );
        assert!(d.rms_after < d.rms_before);
        assert!(matches!(w.status, Status::LensSet { .. }));
        assert!(w
            .status
            .text(Lang::En)
            .starts_with("Lens fitted: k1 = -0.0"));
        let line = w.distortion_line().unwrap();
        assert!(
            line.starts_with("k1 = -0.0") && line.contains("k2 = 0.0"),
            "{line}"
        );
        // the lens that was fitted is the one of `lens`, which uses the same code
        let raw: Vec<(f64, f64)> = w.marks.iter().map(|m| (m.x, m.y)).collect();
        let l = fit_lens(&raw, 5, 4, (320.0, 240.0), 400.0, 0.5).unwrap();
        assert_eq!((l.k1, l.k2), (d.k1, d.k2));
    }

    #[test]
    fn a_fit_that_leaves_the_marks_off_their_lattice_is_refused_with_the_value_of_the_window() {
        let s = Scratch::new("refuse");
        let path = lens_picture(&s, -0.06, 0.02);
        let mut w = window_with("lens_max_rms = 0.0001\n");
        assert!(w.open_image(&path));
        assert!(!w.fit_grid());
        assert!(
            matches!(&w.status, Status::Failed(e) if e.starts_with("lens_residual_too_large")),
            "{:?}",
            w.status
        );
        assert!(w.distortion.is_none());
        // the tolerance is a field of the panel and a value that makes no sense is refused
        let i = w
            .fields
            .iter()
            .position(|f| f.key == "lens_max_rms")
            .unwrap();
        assert!(w.set_field(i, "2".into()));
        assert!(w.fit_grid());
        assert!(!w.set_field(i, "0".into()));
        assert!(w.status.is_problem());
    }

    #[test]
    fn the_lattice_the_lens_predicts_lies_on_the_marks_and_follows_the_switch() {
        let s = Scratch::new("lattice");
        let path = lens_picture(&s, 0.05, -0.02);
        let mut w = window_with("");
        assert!(w.open_image(&path));
        // nothing is predicted before the lens is fitted from these marks
        assert!(w.lens_grid_px().is_empty());
        assert!(w.fit_grid());
        let places = w.lens_grid_px();
        assert_eq!(places.len(), 20);
        for place in &places {
            let near = w
                .marks
                .iter()
                .map(|m| (m.x - place.0).hypot(m.y - place.1))
                .fold(f64::MAX, f64::min);
            assert!(near < 0.6, "{place:?} is {near} px from the nearest mark");
        }
        // with the correction on the marks are drawn corrected and lie on the straight lattice
        w.set_use_distortion(true);
        let straight = w.lens_grid_px();
        assert_eq!(straight.len(), 20);
        for place in &straight {
            let near = w
                .marks
                .iter()
                .map(|m| {
                    let p = w.mark_px(m);
                    (p.0 - place.0).hypot(p.1 - place.1)
                })
                .fold(f64::MAX, f64::min);
            assert!(near < 0.6, "{place:?} is {near} px from the nearest mark");
        }
        w.set_use_distortion(false);
        w.cfg.show_lens_grid = false;
        assert!(w.lens_grid_px().is_empty());
        w.cfg.show_lens_grid = true;
        // looking for the marks again forgets the lens that was fitted from the old ones, and with it the lattice
        assert!(w.detect());
        assert!(w.lens_grid_px().is_empty());
    }

    #[test]
    fn the_lens_of_the_configuration_is_used_without_a_grid() {
        let s = Scratch::new("configured");
        let path = lens_picture(&s, 0.0, 0.0);
        let mut w = window_with("");
        assert!(w.open_image(&path));
        assert!(!w.use_configured_lens());
        assert_eq!(w.status, Status::NeedLens);
        assert!(w.status.is_problem());
        let mut w = window_with("lens_k1 = -0.04\nlens_k2 = 0.01\n");
        assert!(w.open_image(&path));
        assert!(w.use_configured_lens());
        assert_eq!(
            w.status,
            Status::LensSet {
                k1: -0.04,
                k2: 0.01
            }
        );
        assert_eq!(
            w.distortion_line().unwrap(),
            "k1 = -0.04000, k2 = 0.01000 (from the configuration)"
        );
        // with the correction on, a mark away from the centre moves; the marks of a lens that was not fitted draw no lattice
        let raw = (w.marks[0].x, w.marks[0].y);
        w.set_use_distortion(true);
        assert_ne!(w.mark_px(&w.marks[0]), raw);
        assert!(w.lens_grid_px().is_empty());
    }

    #[test]
    fn the_factors_are_saved_beside_the_picture_in_the_lines_of_the_configuration() {
        let s = Scratch::new("save");
        let path = lens_picture(&s, -0.06, 0.02);
        let mut w = window_with("");
        assert!(!w.save_lens());
        assert_eq!(w.status, Status::NeedImage);
        assert!(w.open_image(&path));
        assert!(!w.save_lens());
        assert_eq!(w.status, Status::NeedLens);
        assert!(w.fit_grid());
        assert!(w.save_lens());
        assert_eq!(w.status, Status::Exported("lens-lens.cfg".into()));
        let text = fs::read_to_string(s.path("lens-lens.cfg")).unwrap();
        let cfg = Config::parse(&text).unwrap();
        let d = w.distortion.unwrap();
        assert!((cfg.lens_k1 - d.k1).abs() < 1e-6 && (cfg.lens_k2 - d.k2).abs() < 1e-6);
        // the file is not written over unless the value says so
        assert!(!w.save_lens());
        assert!(matches!(&w.status, Status::Failed(e) if e.starts_with("file_exists")));
        w.cfg.replace_output = true;
        assert!(w.save_lens());
    }

    /// A table of two packages saved as `packages.csv`: the body of 0805 is 2 by 1.25 mm with a tolerance of 0.15, SOIC-8 is 4.9 by 3.9.
    fn table(s: &Scratch) -> PathBuf {
        let path = s.path("packages.csv");
        fs::write(
            &path,
            "package,body_long_mm,body_short_mm,tolerance_mm\n0805,2.0,1.25,0.15\nSOIC-8,4.9,3.9,0.25\n",
        )
        .unwrap();
        path
    }

    /// A picture with two dark rectangles, 40 by 25 pixels and 40 by 32 pixels, saved as `parts.pgm`; at 0.05 mm a pixel the first is the body of 0805 and the second is too wide.
    fn parts_picture(s: &Scratch) -> PathBuf {
        let mut g = Gray::blank(320, 160, 220).unwrap();
        for (x0, y0, w, h) in [(40usize, 50usize, 40usize, 25usize), (200, 50, 40, 32)] {
            for y in y0..y0 + h {
                for x in x0..x0 + w {
                    g.data[y * 320 + x] = 30;
                }
            }
        }
        let path = s.path("parts.pgm");
        fs::write(&path, pgm(&g)).unwrap();
        path
    }

    /// A window with the picture open, the scale and the table of `table`, and the package named.
    fn inspecting(s: &Scratch, package: &str) -> Window {
        let cfg = format!(
            "mm_per_px = 0.05\npackage_table = {}\ninspect_package = {package}\n",
            table(s).display()
        );
        let mut w = window_with(&cfg);
        assert!(w.open_image(&parts_picture(s)));
        w
    }

    #[test]
    fn the_parts_are_judged_against_the_package_and_the_verdicts_are_saved() {
        let s = Scratch::new("inspect");
        let mut w = inspecting(&s, "0805");
        // the parts have to be measured first
        assert!(!w.inspect_parts());
        assert_eq!(w.status, Status::NeedParts);
        assert!(w.measure());
        assert_eq!(w.parts.len(), 2);
        assert!(w.inspect_parts());
        assert_eq!(w.status, Status::InspectDone { pass: 1, fail: 1 });
        assert!(w.status.text(Lang::En).contains("1 parts pass, 1 fail"));
        let lines = w.inspect_lines();
        assert_eq!(lines.len(), 2);
        // the order of the parts is the one of `measure`: one passes and one fails
        let pass = lines.iter().find(|l| l.1).unwrap();
        let fail = lines.iter().find(|l| !l.1).unwrap();
        assert!(pass.0.contains(": passes (2.000 x 1.250 mm"), "{lines:?}");
        assert!(fail.0.contains(": fails by width ("), "{lines:?}");
        w.lang = Lang::Es;
        assert!(w
            .inspect_lines()
            .iter()
            .any(|l| l.0.contains("falla por ancho")));
        w.lang = Lang::En;
        // the same verdicts as the command line writes
        assert!(w.save_inspection());
        assert_eq!(w.status, Status::Exported("parts-inspection.csv".into()));
        let csv = fs::read_to_string(s.path("parts-inspection.csv")).unwrap();
        assert!(csv.starts_with(crate::inspect::CSV_HEADER), "{csv}");
        assert!(
            csv.contains(",0805,2.0000,1.2500,")
                && csv.contains(",pass,")
                && csv.contains(",fail,width")
        );
        assert!(!w.save_inspection());
        assert!(matches!(&w.status, Status::Failed(e) if e.starts_with("file_exists")));
        // a looser tolerance lets the wide part pass, and measuring again forgets the verdicts
        let i = w
            .fields
            .iter()
            .position(|f| f.key == "package_table")
            .unwrap();
        assert!(w.set_field(i, w.cfg.package_table.clone()));
        assert!(w.set_field(
            w.fields
                .iter()
                .position(|f| f.key == "inspect_tolerance")
                .unwrap(),
            "0.5".into()
        ));
        // the package has its own tolerance of 0.15, which wins over the field
        assert!(w.inspect_parts());
        assert_eq!(w.status, Status::InspectDone { pass: 1, fail: 1 });
        assert!(w.measure());
        assert!(w.judged.is_empty() && w.judged_package.is_none());
        assert!(!w.save_inspection());
        assert_eq!(w.status, Status::NeedParts);
    }

    #[test]
    fn what_cannot_be_judged_is_said_with_its_reason() {
        let s = Scratch::new("inspect_refuse");
        let mut w = inspecting(&s, "");
        assert!(w.measure());
        // no package named
        assert!(!w.inspect_parts());
        assert_eq!(w.status, Status::NeedPackage);
        // a package that is not in the table
        let name = w
            .fields
            .iter()
            .position(|f| f.key == "inspect_package")
            .unwrap();
        assert!(w.set_field(name, "9999".into()));
        assert!(!w.inspect_parts());
        assert!(
            matches!(&w.status, Status::Failed(e) if e.starts_with("package_unknown")),
            "{:?}",
            w.status
        );
        // a table that is not there, a table with a bad line and no table at all
        let table_field = w
            .fields
            .iter()
            .position(|f| f.key == "package_table")
            .unwrap();
        assert!(w.set_field(table_field, s.path("missing.csv").display().to_string()));
        assert!(!w.inspect_parts());
        assert!(
            matches!(&w.status, Status::Failed(e) if e.starts_with("file_unreadable")),
            "{:?}",
            w.status
        );
        fs::write(
            s.path("bad.csv"),
            "package,body_long_mm,body_short_mm\n0805,1,2\n",
        )
        .unwrap();
        assert!(w.set_field(table_field, s.path("bad.csv").display().to_string()));
        assert!(!w.load_packages());
        assert!(
            matches!(&w.status, Status::Failed(e) if e.contains("bad.csv") && e.contains("line 2")),
            "{:?}",
            w.status
        );
        assert!(w.set_field(table_field, String::new()));
        assert!(!w.load_packages());
        assert_eq!(w.status, Status::NeedTable);
        assert!(w.status.is_problem());
        // the table is read within the limit of the files
        assert!(w.set_field(table_field, table(&s).display().to_string()));
        assert!(w.load_packages());
        assert_eq!(w.status, Status::PackagesLoaded(2));
        assert_eq!(w.packages.len(), 2);
        // without a picture there is nothing to judge
        let mut empty = window_with("");
        assert!(!empty.inspect_parts());
        assert_eq!(empty.status, Status::NeedImage);
        assert!(!empty.save_inspection());
    }

    #[test]
    fn the_pictures_a_big_part_needs_are_planned_from_the_size_of_the_field() {
        let s = Scratch::new("shots");
        let mut w =
            window_with("mm_per_px = 0.05\nshots_part_width = 60\nshots_part_height = 30\n");
        assert!(!w.plan_shots());
        assert_eq!(w.status, Status::NeedImage);
        assert!(w.open_image(&parts_picture(&s)));
        // the picture is 320 by 160 pixels, 16 by 8 millimetres at 0.05 mm a pixel
        assert!(w.plan_shots());
        let p = w.plan.clone().unwrap();
        let expected = crate::shots::plan(60.0, 30.0, 16.0, 8.0, 0.1, 0.5).unwrap();
        assert_eq!(p, expected);
        assert!(p.shots.len() > 1);
        assert_eq!(
            w.status,
            Status::ShotsPlanned {
                pictures: p.shots.len(),
                columns: p.columns,
                rows: p.rows
            }
        );
        let lines = w.shots_lines();
        assert_eq!(lines.len(), p.shots.len());
        assert!(lines[0].starts_with("Picture 1: x "), "{lines:?}");
        // the file is the one of the command line
        assert!(w.save_shots());
        assert_eq!(w.status, Status::Exported("parts-shots.csv".into()));
        assert_eq!(
            fs::read_to_string(s.path("parts-shots.csv")).unwrap(),
            crate::shots::to_csv(&p)
        );
        assert!(!w.save_shots());
        assert!(matches!(&w.status, Status::Failed(e) if e.starts_with("file_exists")));
        // the overlap and the margin are the fields of the panel and the plan follows them
        let overlap = w
            .fields
            .iter()
            .position(|f| f.key == "shots_overlap")
            .unwrap();
        assert!(w.set_field(overlap, "0.5".into()));
        assert!(w.plan_shots());
        assert!(w.plan.as_ref().unwrap().shots.len() > p.shots.len());
        assert!(!w.set_field(overlap, "0.95".into()));
        assert!(w.status.is_problem());
    }

    #[test]
    fn the_part_of_the_plan_comes_from_the_package_when_no_size_is_given() {
        let s = Scratch::new("shots_package");
        let mut w = inspecting(&s, "SOIC-8");
        // the body of SOIC-8 is 4.9 by 3.9 mm; the field is 16 by 8 mm: one picture
        assert!(w.plan_shots());
        assert_eq!(w.plan.as_ref().unwrap().shots.len(), 1);
        // without a size and without a package the part is not known
        let mut none = window_with("mm_per_px = 0.05\n");
        assert!(none.open_image(&parts_picture(&s)));
        assert!(!none.plan_shots());
        assert_eq!(none.status, Status::NeedBody);
        assert!(!none.save_shots());
        assert_eq!(none.status, Status::NeedBody);
        // a package that is not there is the status
        let name = w
            .fields
            .iter()
            .position(|f| f.key == "inspect_package")
            .unwrap();
        assert!(w.set_field(name, "nope".into()));
        assert!(!w.plan_shots());
        assert!(matches!(&w.status, Status::Failed(e) if e.starts_with("package_unknown")));
        // without a scale there is no field in millimetres
        let mut unscaled = window_with("shots_part_width = 10\nshots_part_height = 10\n");
        assert!(unscaled.open_image(&parts_picture(&s)));
        assert!(!unscaled.plan_shots());
        assert_eq!(unscaled.status, Status::NeedScale);
        // the status sentences are spoken in every language
        for st in [
            Status::PackagesLoaded(3),
            Status::NeedTable,
            Status::NeedParts,
            Status::NeedPackage,
            Status::InspectDone { pass: 4, fail: 1 },
            Status::ShotsPlanned {
                pictures: 6,
                columns: 3,
                rows: 2,
            },
            Status::NeedBody,
        ] {
            let texts: Vec<String> = Lang::ALL.iter().map(|l| st.text(*l)).collect();
            assert!(
                texts.iter().all(|t| !t.is_empty() && !t.contains('{')),
                "{st:?}"
            );
            assert_ne!(texts[0], texts[5], "{st:?}");
        }
    }

    /// A window with the picture of two parts open, the scale, the table and the package `0805` named, and the lines of `extra` after them.
    fn holding(s: &Scratch, extra: &str) -> Window {
        let cfg = format!(
            "mm_per_px = 0.05\npackage_table = {}\ninspect_package = 0805\n{extra}",
            table(s).display()
        );
        let mut w = window_with(&cfg);
        assert!(w.open_image(&parts_picture(s)));
        w
    }

    fn field(w: &Window, key: &str) -> usize {
        w.fields.iter().position(|f| f.key == key).unwrap()
    }

    #[test]
    fn the_correction_of_every_part_is_worked_out_listed_and_saved_as_a_table() {
        let s = Scratch::new("offsets");
        let mut none = window_with("");
        assert!(!none.offset_parts());
        assert_eq!(none.status, Status::NeedImage);
        assert!(!none.save_offsets());
        assert_eq!(none.status, Status::NeedImage);
        let mut w = holding(&s, "nozzle_x_mm = -5\nnominal_angle_deg = 0\n");
        assert!(!w.save_offsets());
        assert_eq!(w.status, Status::NeedParts);
        assert!(w.offset_parts());
        // the wider part is the larger one and comes first; it is too wide for the package
        assert_eq!(w.status, Status::OffsetDone { pass: 1, fail: 1 });
        assert_eq!(w.held.len(), 2);
        assert_eq!(w.held[0].reasons, vec!["width"]);
        assert!(w.held[1].passes());
        // the offset is the place of the part from the axis of the nozzle, and the correction the opposite
        let o = &w.held[1];
        assert!((o.x_mm - (o.part.x_mm + 5.0)).abs() < 1e-9, "{o:?}");
        assert!((o.y_mm - o.part.y_mm).abs() < 1e-9);
        let lines = w.offset_lines();
        assert_eq!(lines.len(), 2);
        assert!(!lines[0].1 && lines[1].1);
        assert!(
            lines[0].0.starts_with("Part 1 fails (width): correction ("),
            "{lines:?}"
        );
        let (cx, cy, cr) = o.correction();
        assert_eq!(
            lines[1].0,
            format!(
                "Part 2: correction ({}, {}) mm, turn {} degrees",
                show(cx, 3),
                show(cy, 3),
                show(cr, 2)
            )
        );
        // the table is the one of the command line: a column with the number of the part and a row for each
        assert!(w.save_offsets());
        assert_eq!(w.status, Status::Exported("parts-offsets.csv".into()));
        let csv = fs::read_to_string(s.path("parts-offsets.csv")).unwrap();
        assert_eq!(
            csv,
            crate::offset::to_table_csv(&w.held, w.held_package.as_ref().unwrap())
        );
        assert!(csv.starts_with(crate::offset::TABLE_HEADER), "{csv}");
        assert_eq!(csv.lines().count(), 3);
        assert!(!w.save_offsets());
        assert!(matches!(&w.status, Status::Failed(e) if e.starts_with("file_exists")));
        // a limit that is too tight fails the part with its reason, in the language of the window
        let far = field(&w, "pick_max_offset_mm");
        assert!(w.set_field(far, "0.01".into()));
        assert!(w.offset_parts());
        assert_eq!(w.status, Status::OffsetDone { pass: 0, fail: 2 });
        assert!(
            w.offset_lines()[1].0.contains("too far from the nozzle"),
            "{:?}",
            w.offset_lines()
        );
        let turn = field(&w, "pick_max_turn_deg");
        assert!(!w.set_field(turn, "91".into()) && w.status.is_problem());
        // a new picture forgets the table
        assert!(w.open_image(&parts_picture(&s)));
        assert!(w.held.is_empty() && w.offset_lines().is_empty());
    }

    #[test]
    fn a_camera_that_sees_the_parts_mirrored_changes_the_sign_of_their_x_and_of_their_turn() {
        let s = Scratch::new("offsets_mirror");
        let mut w = holding(&s, "");
        assert!(w.offset_parts());
        let direct = w.held.clone();
        w.set_pick_mirror(true);
        assert!(w.held.is_empty());
        assert!(w.offset_parts());
        assert_eq!(w.held.len(), direct.len());
        for (m, d) in w.held.iter().zip(&direct) {
            assert_eq!(m.part.x_mm, -d.part.x_mm);
            assert_eq!(m.part.y_mm, d.part.y_mm);
            assert_eq!(m.x_mm, -d.x_mm);
            assert_eq!(m.part.length_mm, d.part.length_mm);
        }
        w.set_pick_mirror(false);
        assert!(w.offset_parts());
        assert_eq!(w.held, direct);
    }

    #[test]
    fn the_corrections_are_refused_with_the_reason_and_leave_nothing_behind() {
        let s = Scratch::new("offsets_refused");
        // no scale
        let mut w = window_with("");
        assert!(w.open_image(&parts_picture(&s)));
        assert!(!w.offset_parts());
        assert_eq!(w.status, Status::NeedScale);
        // no table, no package, a package that is not in the table
        let mut w = window_with("mm_per_px = 0.05\n");
        assert!(w.open_image(&parts_picture(&s)));
        assert!(!w.offset_parts());
        assert_eq!(w.status, Status::NeedTable);
        let mut w = holding(&s, "");
        let name = field(&w, "inspect_package");
        assert!(w.set_field(name, String::new()));
        assert!(!w.offset_parts());
        assert_eq!(w.status, Status::NeedPackage);
        assert!(w.set_field(name, "nope".into()));
        assert!(!w.offset_parts());
        assert!(matches!(&w.status, Status::Failed(e) if e.starts_with("package_unknown")));
        assert!(w.held.is_empty());
        // a picture with nothing in it
        let mut blank = holding(&s, "");
        let path = s.path("blank.pgm");
        fs::write(&path, pgm(&Gray::blank(100, 100, 220).unwrap())).unwrap();
        assert!(blank.open_image(&path));
        assert!(!blank.offset_parts());
        assert_eq!(blank.status, Status::PartsFound(0));
        // the check of the picture comes first when it is on: a flat picture is refused, a good one is not
        let mut checked = holding(&s, "quality_check = yes\n");
        assert!(checked.offset_parts());
        assert!(checked.open_image(&path));
        assert!(!checked.offset_parts());
        assert!(
            matches!(&checked.status, Status::Failed(e) if e.starts_with("poor_picture: flat")),
            "{:?}",
            checked.status
        );
        assert!(checked.held.is_empty());
        checked.set_quality_check(false);
        assert!(!checked.offset_parts());
        assert_eq!(checked.status, Status::PartsFound(0));
    }

    #[test]
    fn the_picture_is_checked_with_the_values_of_the_panel_and_the_numbers_are_told() {
        let s = Scratch::new("quality");
        let mut w = window_with("");
        assert!(!w.check_quality());
        assert_eq!(w.status, Status::NeedImage);
        assert!(w.quality_line().is_none());
        assert!(w.open_image(&parts_picture(&s)));
        assert!(w.check_quality());
        assert_eq!(w.status, Status::QualityGood);
        assert!(!w.status.is_problem());
        let line = w.quality_line().unwrap();
        assert!(
            line.starts_with("Sharpness ")
                && line.contains(", contrast ")
                && line.contains(", mean level "),
            "{line}"
        );
        // a contrast that this picture does not have, a mean that it is below, and a sharpness that it is below
        let contrast = field(&w, "quality_min_contrast");
        assert!(w.set_field(contrast, "250".into()));
        assert!(w.check_quality());
        assert_eq!(w.status, Status::QualityPoor(vec!["flat"]));
        assert!(w.status.is_problem());
        assert_eq!(w.status.text(Lang::En), "Poor picture: no contrast");
        assert!(w.set_field(contrast, "10".into()));
        let sharp = field(&w, "quality_min_sharpness");
        assert!(w.set_field(sharp, "1000000".into()));
        assert!(w.check_quality());
        assert_eq!(w.status, Status::QualityPoor(vec!["blurry"]));
        let min = field(&w, "quality_min_mean");
        assert!(w.set_field(sharp, "0".into()));
        assert!(w.set_field(min, "230".into()));
        assert!(w.check_quality());
        assert_eq!(w.status, Status::QualityPoor(vec!["dark"]));
        // a mean range that makes no sense is refused and the old one stays
        assert!(!w.set_field(min, "240".into()));
        assert!(w.status.is_problem());
        // a new picture forgets the numbers
        assert!(w.open_image(&parts_picture(&s)));
        assert!(w.quality_line().is_none());
        // the new status sentences are spoken in every language
        for st in [
            Status::OffsetDone { pass: 2, fail: 1 },
            Status::QualityGood,
            Status::QualityPoor(vec!["flat", "dark", "bright", "blurry"]),
        ] {
            let texts: Vec<String> = Lang::ALL.iter().map(|l| st.text(*l)).collect();
            assert!(
                texts.iter().all(|t| !t.is_empty() && !t.contains('{')),
                "{st:?}"
            );
            assert_ne!(texts[0], texts[5], "{st:?}");
        }
    }
}
