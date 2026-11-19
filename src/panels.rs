//! The panels of the window for the lens, the parts against their packages and the pictures that a big part needs. Each one is a method of `Window` that sets the status and keeps its result,
//! so that all that the window decides is tested without a screen; `gui.rs` only draws it. The files that are written are named after the open picture and are never written over a file that
//! is there unless `replace_output` says so.

use crate::limit::write_text;
use crate::window::{Status, Window};

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
}
