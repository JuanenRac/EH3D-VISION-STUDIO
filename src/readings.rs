//! The panels of the window for the histogram and its thresholds, the edge to a fraction of a pixel and the check against a reference through a mask of tolerance: what the commands
//! `histogram`, `edge` and `refcheck` do on the command line, with the values of the fields of the window. Each one is a method of `Window` that sets the status and keeps its result,
//! so that all that the window decides is tested without a screen; `gui.rs` only draws it. The server settings of the window are kept here too, since they are one more thing the
//! window reads from the configuration and hands back to it.

use crate::edge::measure as measure_edge;
use crate::histogram::{histogram, report, Histogram, Report};
use crate::image::from_bytes;
use crate::lang::Msg;
use crate::reference::map as reference_map;
use crate::server::{link_settings, sc};
use crate::window::{file_name, Loaded, Status, Window};
use std::path::Path;

/// The labels of the four numbers that lay the line of an edge across the picture, in the order they are kept.
pub const EDGE_LABELS: [Msg; 4] = [Msg::EdgeFromX, Msg::EdgeFromY, Msg::EdgeToX, Msg::EdgeToY];

/// How many groups of grey levels the window draws the histogram in.
pub const HISTOGRAM_BARS: usize = 32;

impl Window {
    /// Reads the histogram of the open picture and the thresholds that Otsu, isodata and the triangle method pick, as `eh3d-vision histogram` does.
    pub fn read_histogram(&mut self) -> bool {
        let Some(image) = self.image.as_ref() else {
            self.status = Status::NeedImage;
            return false;
        };
        match histogram(&image.gray) {
            Ok(h) => {
                let r = report(&h);
                self.status = Status::Histogram {
                    otsu: r.otsu,
                    isodata: r.isodata,
                    triangle: r.triangle,
                    agree: r.agree,
                };
                self.histogram = Some((h, r));
                true
            }
            Err(e) => {
                self.histogram = None;
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// The report of the last histogram.
    pub fn histogram_report(&self) -> Option<&Report> {
        self.histogram.as_ref().map(|(_, r)| r)
    }

    /// The counts of the last histogram in [`HISTOGRAM_BARS`] groups, for the window to draw.
    pub fn histogram_bars(&self) -> Option<Vec<u64>> {
        let (h, _) = self.histogram.as_ref()?;
        h.grouped(HISTOGRAM_BARS).ok()
    }

    /// The histogram that was read, as it is kept.
    pub fn histogram_of(&self) -> Option<&Histogram> {
        self.histogram.as_ref().map(|(h, _)| h)
    }

    /// Takes the text typed in one of the four numbers of the line across an edge (`index` 0 to 3).
    pub fn set_edge_field(&mut self, index: usize, text: String) {
        if let Some(slot) = self.edge_line.get_mut(index) {
            *slot = text;
            self.edge = None;
        }
    }

    /// Measures the edge across the line of the four numbers, as `eh3d-vision edge` does with the values of `edge_step_px` and `edge_max_uncertainty_px`.
    pub fn measure_edge(&mut self) -> bool {
        let Some(image) = self.image.as_ref() else {
            self.status = Status::NeedImage;
            return false;
        };
        let mut v = [0.0f64; 4];
        for (k, slot) in v.iter_mut().enumerate() {
            match self.edge_line[k].trim().parse::<f64>() {
                Ok(n) if n.is_finite() => *slot = n,
                _ => {
                    self.status = Status::BadValue(EDGE_LABELS[k]);
                    return false;
                }
            }
        }
        match measure_edge(
            &image.gray,
            (v[0], v[1]),
            (v[2], v[3]),
            &self.cfg.edge_settings(),
        ) {
            Ok(e) => {
                self.status = Status::EdgeDone {
                    x: e.x,
                    y: e.y,
                    uncertainty: e.uncertainty_px,
                    sure: e.sure,
                };
                self.edge = Some(e);
                true
            }
            Err(why) => {
                self.edge = None;
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// Opens the picture that is the mask of tolerance of the check against the reference. It is only read.
    pub fn open_mask(&mut self, path: &Path) -> bool {
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
                self.mask = Some(Loaded {
                    name: name.clone(),
                    path: path.to_path_buf(),
                    gray,
                });
                self.reference = None;
                self.status = Status::MaskLoaded(name);
                true
            }
            Err(e) => {
                self.status = Status::Failed(format!("{name}: {e}"));
                false
            }
        }
    }

    /// Forgets the mask of tolerance: every pixel is held to the general tolerance again.
    pub fn clear_mask(&mut self) {
        self.mask = None;
        self.reference = None;
    }

    /// Checks the open picture against the other one (the reference) through the mask of tolerance, if there is one, as `eh3d-vision refcheck` does with the threshold, the least area and
    /// the share of the fields of the comparison of two pictures.
    pub fn check_reference(&mut self) -> bool {
        let Some(picture) = self.image.as_ref() else {
            self.status = Status::NeedImage;
            return false;
        };
        let Some(reference) = self.other.as_ref() else {
            self.status = Status::NeedOther;
            return false;
        };
        match reference_map(
            &reference.gray,
            &picture.gray,
            self.mask.as_ref().map(|m| &m.gray),
            &self.cfg.diff_settings(),
        ) {
            Ok((_, c)) => {
                let percent = c.outside_share * 100.0;
                self.status = if c.same {
                    Status::ReferenceSame { percent }
                } else {
                    Status::ReferenceDifferent {
                        percent,
                        regions: c.regions,
                    }
                };
                self.reference = Some(c);
                true
            }
            Err(e) => {
                self.reference = None;
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// The settings of the link to the server that the configuration holds.
    pub fn link(&self) -> Result<sc::Settings, String> {
        link_settings(&self.cfg)
    }

    /// Takes the settings that the panel of the server applied: they are checked as a whole and, when valid, replace the six keys of the link in the configuration. A server that is
    /// not wanted any more is an empty address. The token is kept as it is typed, and never written anywhere by the window.
    pub fn apply_link(&mut self, s: &sc::Settings) -> Result<(), String> {
        let mut trial = self.cfg.clone();
        trial.server_address = s.server_address.clone();
        trial.server_token = s.server_token.expose().to_string();
        trial.server_machine = s.server_machine.clone();
        trial.server_timeout_s = s.server_timeout_s;
        trial.server_retries = s.server_retries;
        trial.server_mdns = s.server_mdns;
        trial.validate()?;
        self.cfg = trial;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::image::Gray;
    use crate::lang::Lang;
    use crate::sim::to_pgm;
    use std::path::PathBuf;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "eh3d_vision_readings_{}_{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn window() -> Window {
        Window::new(Lang::En, Config::default(), None)
    }

    /// A picture with a vertical edge at x = 20.4 (a half pixel of blur), 40 on the left and 210 on the right.
    fn edge_picture() -> Gray {
        let mut data = Vec::new();
        for _ in 0..24 {
            for x in 0..48 {
                let cover = 0.5 * (1.0 + erf((x as f64 - 20.4) / 0.7));
                data.push((40.0 + 170.0 * cover).round() as u8);
            }
        }
        Gray::new(48, 24, data).unwrap()
    }

    fn erf(x: f64) -> f64 {
        let t = 1.0 / (1.0 + 0.327_591_1 * x.abs());
        let poly = t
            * (0.254_829_592
                + t * (-0.284_496_736
                    + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
        (1.0 - poly * (-x * x).exp()).copysign(x)
    }

    fn open(w: &mut Window, d: &Path, name: &str, g: &Gray) -> bool {
        let p = d.join(name);
        std::fs::write(&p, to_pgm(g)).unwrap();
        w.open_image(&p)
    }

    #[test]
    fn the_histogram_of_a_picture_is_read_with_its_three_thresholds() {
        let d = dir("hist");
        let mut w = window();
        assert!(!w.read_histogram());
        assert_eq!(w.status, Status::NeedImage);
        assert!(open(&mut w, &d, "e.pgm", &edge_picture()));
        assert!(w.read_histogram());
        match &w.status {
            Status::Histogram { otsu, .. } => assert!((40..=210).contains(otsu)),
            other => panic!("{other:?}"),
        }
        let r = w.histogram_report().unwrap();
        assert_eq!(r.total, 48 * 24);
        let bars = w.histogram_bars().unwrap();
        assert_eq!(bars.len(), HISTOGRAM_BARS);
        assert_eq!(bars.iter().sum::<u64>(), 48 * 24);
        assert_eq!(w.histogram_of().unwrap().total, 48 * 24);
        for l in Lang::ALL {
            let t = w.status.text(l);
            assert!(!t.contains('{') && !t.trim().is_empty(), "{t}");
        }
    }

    #[test]
    fn the_edge_is_measured_from_the_four_numbers_and_a_bad_one_names_its_label() {
        let d = dir("edge");
        let mut w = window();
        assert!(!w.measure_edge());
        assert_eq!(w.status, Status::NeedImage);
        assert!(open(&mut w, &d, "e.pgm", &edge_picture()));
        for (k, v) in ["6", "12", "40", "12"].into_iter().enumerate() {
            w.set_edge_field(k, v.to_string());
        }
        assert!(w.measure_edge());
        let e = w.edge.clone().unwrap();
        assert!((e.x - 20.4).abs() < 0.06, "{}", e.x);
        assert!(
            matches!(w.status, Status::EdgeDone { sure: true, .. }),
            "{:?}",
            w.status
        );
        assert!(!w.status.is_problem());
        // a field that is not a number is refused with its own label, and the old reading goes
        w.set_edge_field(2, "far".into());
        assert!(w.edge.is_none());
        assert!(!w.measure_edge());
        assert_eq!(w.status, Status::BadValue(Msg::EdgeToX));
        // a line that does not cross an edge says why
        for (k, v) in ["2", "2", "10", "2"].into_iter().enumerate() {
            w.set_edge_field(k, v.to_string());
        }
        assert!(!w.measure_edge());
        assert!(
            matches!(&w.status, Status::Failed(m) if m.starts_with("edge_flat")),
            "{:?}",
            w.status
        );
        // a tight limit on the uncertainty makes the reading not sure, and that is drawn as a problem
        let cfg = Config {
            edge_max_uncertainty_px: 0.0001,
            ..Config::default()
        };
        let mut tight = Window::new(Lang::En, cfg, None);
        assert!(open(&mut tight, &d, "e2.pgm", &edge_picture()));
        for (k, v) in ["6", "12", "40", "12"].into_iter().enumerate() {
            tight.set_edge_field(k, v.to_string());
        }
        assert!(tight.measure_edge());
        assert!(tight.status.is_problem());
        // an index that is not one of the four is ignored
        tight.set_edge_field(9, "1".into());
        assert_eq!(tight.edge_line.len(), 4);
    }

    #[test]
    fn a_picture_is_checked_against_a_reference_and_a_mask_widens_or_blinds_places() {
        let d = dir("ref");
        let base = Gray::blank(40, 30, 120).unwrap();
        let mut changed = base.clone();
        for y in 6..14 {
            for x in 6..14 {
                changed.data[y * 40 + x] = 230;
            }
        }
        let mut w = window();
        assert!(open(&mut w, &d, "shown.pgm", &changed));
        assert!(!w.check_reference());
        assert_eq!(w.status, Status::NeedOther);
        std::fs::write(d.join("ref.pgm"), to_pgm(&base)).unwrap();
        assert!(w.open_other(&d.join("ref.pgm")));
        assert!(w.check_reference());
        assert!(
            matches!(w.status, Status::ReferenceDifferent { regions: 1, .. }),
            "{:?}",
            w.status
        );
        assert!(w.status.is_problem());
        assert_eq!(w.reference.as_ref().unwrap().outside_px, 64);
        // a mask that leaves the place out: the picture is within the tolerance
        let mut blind = Gray::blank(40, 30, 0).unwrap();
        for y in 6..14 {
            for x in 6..14 {
                blind.data[y * 40 + x] = 255;
            }
        }
        std::fs::write(d.join("mask.pgm"), to_pgm(&blind)).unwrap();
        assert!(w.open_mask(&d.join("mask.pgm")));
        assert_eq!(w.status, Status::MaskLoaded("mask.pgm".into()));
        assert!(w.reference.is_none());
        assert!(w.check_reference());
        assert!(
            matches!(w.status, Status::ReferenceSame { .. }),
            "{:?}",
            w.status
        );
        assert!(!w.status.is_problem());
        // forgetting the mask holds every pixel to the general tolerance again
        w.clear_mask();
        assert!(w.check_reference());
        assert!(matches!(w.status, Status::ReferenceDifferent { .. }));
        // a mask of another size is refused with its code
        std::fs::write(
            d.join("small.pgm"),
            to_pgm(&Gray::blank(20, 30, 0).unwrap()),
        )
        .unwrap();
        assert!(w.open_mask(&d.join("small.pgm")));
        assert!(!w.check_reference());
        assert!(
            matches!(&w.status, Status::Failed(m) if m.contains("size_mismatch")),
            "{:?}",
            w.status
        );
        // a file that is not a picture is not a mask
        std::fs::write(d.join("text.pgm"), b"hello").unwrap();
        assert!(!w.open_mask(&d.join("text.pgm")));
        assert!(!w.open_mask(&d.join("not-there.pgm")));
        for l in Lang::ALL {
            for st in [
                Status::ReferenceSame { percent: 0.5 },
                Status::ReferenceDifferent {
                    percent: 2.0,
                    regions: 3,
                },
                Status::MaskLoaded("m.pgm".into()),
                Status::EdgeDone {
                    x: 1.0,
                    y: 2.0,
                    uncertainty: 0.1,
                    sure: false,
                },
                Status::Histogram {
                    otsu: 1,
                    isodata: 2,
                    triangle: 3,
                    agree: true,
                },
            ] {
                let t = st.text(l);
                assert!(!t.contains('{') && !t.trim().is_empty(), "{st:?}: {t}");
            }
        }
    }

    #[test]
    fn the_settings_of_the_server_that_the_panel_applies_are_checked_and_kept() {
        let mut w = window();
        let mut s = w.link().unwrap();
        assert_eq!(s.server_address, "");
        s.set("server_address", "192.168.0.180:18200").unwrap();
        s.set("server_machine", "bench").unwrap();
        s.set("server_token", "a-token").unwrap();
        s.set("server_timeout_s", "30").unwrap();
        s.set("server_retries", "4").unwrap();
        s.set("server_mdns", "on").unwrap();
        w.apply_link(&s).unwrap();
        assert_eq!(w.cfg.server_address, "192.168.0.180:18200");
        assert_eq!(w.cfg.server_machine, "bench");
        assert_eq!(w.cfg.server_token, "a-token");
        assert_eq!((w.cfg.server_timeout_s, w.cfg.server_retries), (30, 4));
        assert!(w.cfg.server_mdns);
        let back = w.link().unwrap();
        assert_eq!(back.server_address, "192.168.0.180:18200");
        assert!(back.server_mdns);
        // the server can be taken away again
        let mut none = back.clone();
        none.set("server_address", "").unwrap();
        w.apply_link(&none).unwrap();
        assert_eq!(w.cfg.server_address, "");
        // the other values of the configuration are not touched
        assert_eq!(w.cfg.min_diameter, Config::default().min_diameter);
    }
}
