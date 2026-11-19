//! The tools of the side panel that work on the picture of the window: the picture with its uneven light taken out, the squares and the crosses, the parts in millimetres, the report of
//! the folder and the scale from the pitch of a target. Each one is a method of `Window` that sets the status and keeps its result, so that all that the window decides is tested without
//! a screen; `gui.rs` only draws it.

use crate::batch::{list_pictures, Checked};
use crate::flatten::flatten;
use crate::image::Gray;
use crate::job::{run as run_batch, BatchJob, BatchParams, Poll};
use crate::lang::Msg;
use crate::limit::write_text;
use crate::measure::{measure_parts, to_csv, PartSettings};
use crate::report;
use crate::scale::{from_pitch, Scale};
use crate::square::{find_squares, find_turned_crosses, ShapeSettings};
use crate::window::{show, Status, Window};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize};

/// The name of the report of a folder that the window writes in the folder of the picture.
pub const REPORT_NAME: &str = "eh3d-vision-report.html";

/// Which shape a mark found by the window has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    Square,
    Cross,
}

/// A square or a cross that was found, in pixels of the picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeMark {
    pub kind: ShapeKind,
    pub x: f64,
    pub y: f64,
    /// The side of the square, or the side of the box of the cross.
    pub size: f64,
    /// The turn counter-clockwise as the machine sees it, from -45 (not included) to 45 degrees.
    pub angle_deg: f64,
    pub score: f64,
}

impl Window {
    /// The picture that the searches work on: as it was opened, or with the uneven light taken out when `flatten_window` is above 0.
    pub fn working(&self) -> Result<Cow<'_, Gray>, String> {
        let img = self.image.as_ref().ok_or("there is no picture")?;
        match self.cfg.flatten_window {
            0 => Ok(Cow::Borrowed(&img.gray)),
            window => flatten(&img.gray, window).map(Cow::Owned),
        }
    }

    /// The settings of the search for squares or for crosses that the values of the window make: each shape has its own range of size and its own elongation.
    pub fn shape_settings(&self, kind: ShapeKind) -> ShapeSettings {
        let (min_size, max_size, max_elongation) = match kind {
            ShapeKind::Square => (
                self.cfg.square_min_size,
                self.cfg.square_max_size,
                self.cfg.square_max_elongation,
            ),
            ShapeKind::Cross => (
                self.cfg.cross_min_size,
                self.cfg.cross_max_size,
                self.cfg.cross_max_elongation,
            ),
        };
        ShapeSettings {
            polarity: self.cfg.polarity,
            threshold: self.cfg.threshold,
            min_size,
            max_size,
            max_elongation,
            min_fill: self.cfg.square_min_fill,
            min_score: self.cfg.cross_min_score,
            max_spots: self.cfg.max_parts,
        }
    }

    /// Looks for the squares or the crosses of the picture; what was found before of the same shape is replaced.
    pub fn find_shapes(&mut self, kind: ShapeKind) -> bool {
        if self.image.is_none() {
            self.status = Status::NeedImage;
            return false;
        }
        let settings = self.shape_settings(kind);
        let result = self.working().and_then(|gray| match kind {
            ShapeKind::Square => find_squares(&gray, &settings).map(|found| {
                found
                    .into_iter()
                    .map(|q| ShapeMark {
                        kind,
                        x: q.x,
                        y: q.y,
                        size: q.side,
                        angle_deg: q.angle_deg,
                        score: q.score,
                    })
                    .collect::<Vec<_>>()
            }),
            ShapeKind::Cross => find_turned_crosses(&gray, &settings).map(|found| {
                found
                    .into_iter()
                    .map(|c| ShapeMark {
                        kind,
                        x: c.x,
                        y: c.y,
                        size: c.size,
                        angle_deg: c.angle_deg,
                        score: c.score,
                    })
                    .collect::<Vec<_>>()
            }),
        });
        match result {
            Ok(found) => {
                let n = found.len();
                self.shapes.retain(|s| s.kind != kind);
                self.shapes.extend(found);
                self.status = match kind {
                    ShapeKind::Square => Status::SquaresFound(n),
                    ShapeKind::Cross => Status::CrossesFound(n),
                };
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// The lines of the list of squares and crosses.
    pub fn shape_lines(&self) -> Vec<String> {
        self.shapes
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let msg = match s.kind {
                    ShapeKind::Square => Msg::SquareLine,
                    ShapeKind::Cross => Msg::CrossLine,
                };
                msg.with(
                    self.lang,
                    &[
                        (i + 1).to_string(),
                        show(s.x, 1),
                        show(s.y, 1),
                        show(s.size, 1),
                        show(s.angle_deg, 1),
                    ],
                )
            })
            .collect()
    }

    /// The millimetres of a pixel that the parts are measured with: the value of the field when it is above 0, else the scale that was set.
    pub fn measure_scale(&self) -> Option<Scale> {
        if self.cfg.mm_per_px > 0.0 {
            Some(Scale {
                mm_per_px: self.cfg.mm_per_px,
            })
        } else {
            self.scale
        }
    }

    /// Measures the parts of the picture in millimetres.
    pub fn measure(&mut self) -> bool {
        if self.image.is_none() {
            self.status = Status::NeedImage;
            return false;
        }
        let Some(scale) = self.measure_scale() else {
            self.status = Status::NeedScale;
            return false;
        };
        let settings = PartSettings {
            polarity: self.cfg.polarity,
            threshold: self.cfg.threshold,
            min_area_px: self.cfg.part_min_area,
            max_area_px: self.cfg.part_max_area,
            max_parts: self.cfg.max_parts,
        };
        let result = self
            .working()
            .and_then(|gray| measure_parts(&gray, &settings, scale));
        match result {
            Ok(parts) => {
                self.status = Status::PartsFound(parts.len());
                self.parts = parts;
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// The lines of the list of parts.
    pub fn part_lines(&self) -> Vec<String> {
        self.parts
            .iter()
            .enumerate()
            .map(|(i, p)| {
                Msg::PartLine.with(
                    self.lang,
                    &[
                        (i + 1).to_string(),
                        show(p.x_mm, 3),
                        show(p.y_mm, 3),
                        show(p.length_mm, 3),
                        show(p.width_mm, 3),
                        show(p.angle_deg, 1),
                    ],
                )
            })
            .collect()
    }

    /// The folder and the name of the picture without its extension, which name the files that are written beside it.
    fn beside(&self) -> Option<(PathBuf, String)> {
        let img = self.image.as_ref()?;
        let folder = img
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let stem = img
            .path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "picture".to_string());
        Some((folder, stem))
    }

    /// Writes the parts as the CSV file `<picture>-parts.csv` beside the picture. A file that is there is written over only when `replace_output` is on.
    pub fn save_parts(&mut self) -> bool {
        let Some((folder, stem)) = self.beside() else {
            self.status = Status::NeedImage;
            return false;
        };
        if self.parts.is_empty() {
            self.status = Status::PartsFound(0);
            return false;
        }
        let name = format!("{stem}-parts.csv");
        match write_text(
            &folder.join(&name),
            &to_csv(&self.parts),
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

    /// The settings every picture of a folder is read with.
    fn batch_params(&self) -> BatchParams {
        BatchParams {
            max_mib: self.cfg.max_file_mib,
            settings: self.cfg.settings(),
            threshold: self.cfg.threshold,
            flatten_px: self.cfg.flatten_window,
        }
    }

    /// Writes the report of the pictures that were read as the HTML file `eh3d-vision-report.html` in the folder, in the language of the window. A file that is there is written over only
    /// when `replace_output` is on.
    fn write_report(&mut self, folder: &Path, items: &[Checked]) -> bool {
        let page = report::html(items, self.lang, self.cfg.report_low_score);
        match write_text(&folder.join(REPORT_NAME), &page, self.cfg.replace_output) {
            Ok(()) => {
                self.status = Status::BatchDone {
                    pictures: items.len(),
                    name: REPORT_NAME.to_string(),
                };
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Reads every picture of the folder of the open picture and writes the report, all at once (the window uses `start_batch_report`, which does not wait). A folder with more pictures
    /// than `batch_max_files` is refused.
    pub fn batch_report(&mut self) -> bool {
        let Some((folder, _)) = self.beside() else {
            self.status = Status::NeedImage;
            return false;
        };
        let files = match list_pictures(&folder, self.cfg.batch_max_files) {
            Ok(f) => f,
            Err(e) => {
                self.status = Status::Failed(e);
                return false;
            }
        };
        let params = self.batch_params();
        let done = AtomicUsize::new(0);
        let items = run_batch(&files, &params, &done, &AtomicBool::new(false)).unwrap_or_default();
        self.write_report(&folder, &items)
    }

    /// Starts the report of the folder of the open picture in a thread and returns at once: `poll_batch` says how far it is and writes the report when it is read, `cancel_batch` stops
    /// it. A second report cannot start while one is running.
    pub fn start_batch_report(&mut self) -> bool {
        if self.batch_job.is_some() {
            self.status = Status::BatchBusy;
            return false;
        }
        let Some((folder, _)) = self.beside() else {
            self.status = Status::NeedImage;
            return false;
        };
        let files = match list_pictures(&folder, self.cfg.batch_max_files) {
            Ok(f) => f,
            Err(e) => {
                self.status = Status::Failed(e);
                return false;
            }
        };
        let total = files.len();
        match BatchJob::start(files, self.batch_params()) {
            Ok(job) => {
                self.batch_job = Some((job, folder));
                self.status = Status::BatchRunning { done: 0, total };
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Whether a report is being read.
    pub fn batch_running(&self) -> bool {
        self.batch_job.is_some()
    }

    /// The share (0 to 1) of the pictures of the running report that are read; 0 when there is none.
    pub fn batch_fraction(&self) -> f32 {
        match &self.batch_job {
            Some((job, _)) => {
                let (done, total) = job.progress();
                if total == 0 {
                    1.0
                } else {
                    done as f32 / total as f32
                }
            }
            None => 0.0,
        }
    }

    /// Asks the running report to stop before the next picture; nothing is written. The status changes when `poll_batch` sees it end.
    pub fn cancel_batch(&mut self) {
        if let Some((job, _)) = &self.batch_job {
            job.cancel();
        }
    }

    /// Looks at the running report without waiting: it sets the status to how far it is, and when it ended it writes the report (or says that it was cancelled or failed). Returns whether a
    /// report is still running after the look.
    pub fn poll_batch(&mut self) -> bool {
        let Some((job, folder)) = self.batch_job.take() else {
            return false;
        };
        match job.poll() {
            Poll::Running { done, total } => {
                self.status = Status::BatchRunning { done, total };
                self.batch_job = Some((job, folder));
                true
            }
            Poll::Finished(items) => {
                if job.is_cancelled() {
                    // the stop came after the last picture was read; the person asked for none
                    self.status = Status::BatchCancelled;
                } else {
                    self.write_report(&folder, &items);
                }
                false
            }
            Poll::Cancelled => {
                self.status = Status::BatchCancelled;
                false
            }
            Poll::Lost => {
                self.status = Status::Failed(
                    "batch_failed: the reading of the folder ended without a result".to_string(),
                );
                false
            }
        }
    }

    /// Sets the scale from the pitch of a target: the marks of the picture are a grid whose neighbours are `pitch_mm` apart.
    pub fn calibrate_pitch(&mut self) -> bool {
        if self.image.is_none() {
            self.status = Status::NeedImage;
            return false;
        }
        if self.cfg.pitch_mm <= 0.0 {
            self.status = Status::NeedPitch;
            return false;
        }
        let places: Vec<(f64, f64)> = self.marks.iter().map(|m| self.mark_px(m)).collect();
        match from_pitch(&places, self.cfg.pitch_mm, self.cfg.pitch_tolerance) {
            Ok(p) => {
                self.scale = Some(p.scale);
                self.alignment = None;
                self.status = Status::PitchScaleSet {
                    mm_per_px: p.scale.mm_per_px,
                    marks: p.marks,
                };
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
    use crate::lang::Lang;
    use std::fs;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let dir = std::env::temp_dir()
                .join(format!("eh3d-vision-tools-{}-{tag}", std::process::id()));
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

    /// Paints a square (`arm_half` of `None`) or a cross of side `2 * half + 1` turned by `phi` degrees (y up) at the pixel (cx, cy).
    fn paint(g: &mut Gray, c: (usize, usize), half: f64, arm_half: Option<f64>, phi: f64) {
        let (sin, cos) = phi.to_radians().sin_cos();
        let reach = (half * 1.5) as isize + 2;
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let (x, y) = (dx as f64, -dy as f64);
                let (u, v) = ((x * cos + y * sin).abs(), (-x * sin + y * cos).abs());
                let inside = match arm_half {
                    None => u <= half && v <= half,
                    Some(a) => (u <= half && v <= a) || (v <= half && u <= a),
                };
                if inside {
                    let (px, py) = (c.0 as isize + dx, c.1 as isize + dy);
                    g.data[py as usize * g.w + px as usize] = 30;
                }
            }
        }
    }

    /// A picture with a disc, a square turned by 25 degrees and a cross turned by 15 degrees, saved as `shapes.pgm`.
    fn shapes_picture(s: &Scratch) -> PathBuf {
        let mut g = Gray::blank(300, 120, 220).unwrap();
        g.disc(50.0, 60.0, 12.0, 30);
        paint(&mut g, (140, 60), 14.0, None, 25.0);
        paint(&mut g, (240, 60), 16.0, Some(2.0), 15.0);
        let path = s.path("shapes.pgm");
        fs::write(&path, pgm(&g)).unwrap();
        path
    }

    #[test]
    fn squares_and_crosses_are_found_with_their_turn_and_listed() {
        let s = Scratch::new("shapes");
        let mut w = window_with("");
        assert!(!w.find_shapes(ShapeKind::Square));
        assert_eq!(w.status, Status::NeedImage);
        assert!(w.open_image(&shapes_picture(&s)));
        assert!(w.find_shapes(ShapeKind::Square));
        assert_eq!(w.status, Status::SquaresFound(1));
        assert!(w.find_shapes(ShapeKind::Cross));
        assert_eq!(w.status, Status::CrossesFound(1));
        assert_eq!(w.shapes.len(), 2);
        let square = w
            .shapes
            .iter()
            .find(|m| m.kind == ShapeKind::Square)
            .unwrap();
        assert!((square.x - 140.0).abs() < 0.3 && (square.angle_deg - 25.0).abs() < 2.0);
        let cross = w
            .shapes
            .iter()
            .find(|m| m.kind == ShapeKind::Cross)
            .unwrap();
        assert!((cross.x - 240.0).abs() < 0.3 && (cross.angle_deg - 15.0).abs() < 2.0);
        let lines = w.shape_lines();
        assert!(
            lines[0].starts_with("Square 1: (140.0, 60.0) px, side "),
            "{lines:?}"
        );
        assert!(
            lines[1].starts_with("Cross 2: (240.0, 60.0) px, size "),
            "{lines:?}"
        );
        // looking again replaces what was found of the same shape and keeps the other
        assert!(w.find_shapes(ShapeKind::Square));
        assert_eq!(w.shapes.len(), 2);
        // looking for the marks again forgets them
        assert!(w.detect());
        assert!(w.shapes.is_empty());
    }

    #[test]
    fn the_tolerances_of_the_shapes_come_from_the_values() {
        let s = Scratch::new("shapes_cfg");
        let mut w = window_with("square_min_fill = 1\ncross_min_size = 60\n");
        assert!(w.open_image(&shapes_picture(&s)));
        assert!(w.find_shapes(ShapeKind::Square));
        assert_eq!(w.status, Status::SquaresFound(0));
        assert!(w.find_shapes(ShapeKind::Cross));
        assert_eq!(w.status, Status::CrossesFound(0));
        assert!(w.status.text(Lang::En).contains("No crosses"));
        // a setting that makes no sense is the status of the failure
        let i = w
            .fields
            .iter()
            .position(|f| f.key == "cross_min_size")
            .unwrap();
        assert!(!w.set_field(i, "3".into()));
        assert!(w.status.is_problem());
    }

    #[test]
    fn a_square_and_a_cross_each_have_their_own_range_of_size() {
        let s = Scratch::new("shapes_own");
        // the square is 29 pixels wide and the cross 33: a range that leaves out only one of them
        let mut w = window_with(
            "square_min_size = 60
",
        );
        assert!(w.open_image(&shapes_picture(&s)));
        assert!(w.find_shapes(ShapeKind::Square));
        assert_eq!(w.status, Status::SquaresFound(0));
        assert!(w.find_shapes(ShapeKind::Cross));
        assert_eq!(w.status, Status::CrossesFound(1));
        let mut w = window_with(
            "cross_max_size = 20
",
        );
        assert!(w.open_image(&shapes_picture(&s)));
        assert!(w.find_shapes(ShapeKind::Cross));
        assert_eq!(w.status, Status::CrossesFound(0));
        assert!(w.find_shapes(ShapeKind::Square));
        assert_eq!(w.status, Status::SquaresFound(1));
        // the elongation too
        let w = window_with(
            "square_max_elongation = 1.5
cross_max_elongation = 3
",
        );
        assert_eq!(w.shape_settings(ShapeKind::Square).max_elongation, 1.5);
        assert_eq!(w.shape_settings(ShapeKind::Cross).max_elongation, 3.0);
        // a value that makes no sense is the status of the failure
        let mut w = window_with("");
        let i = w
            .fields
            .iter()
            .position(|f| f.key == "square_max_size")
            .unwrap();
        assert!(!w.set_field(i, "5".into()));
        assert!(w.status.is_problem());
    }

    #[test]
    fn the_parts_are_measured_with_the_scale_and_saved_beside_the_picture() {
        let s = Scratch::new("parts");
        let mut g = Gray::blank(240, 160, 220).unwrap();
        for y in 45..=75 {
            for x in 70..=130 {
                g.data[y * 240 + x] = 30;
            }
        }
        let path = s.path("part.pgm");
        fs::write(&path, pgm(&g)).unwrap();
        let mut w = window_with("");
        assert!(!w.measure());
        assert_eq!(w.status, Status::NeedImage);
        assert!(w.open_image(&path));
        // no scale, no field: nothing to measure with
        assert!(!w.measure());
        assert_eq!(w.status, Status::NeedScale);
        let i = w.fields.iter().position(|f| f.key == "mm_per_px").unwrap();
        assert!(w.set_field(i, "0.5".into()));
        assert!(w.measure());
        assert_eq!(w.status, Status::PartsFound(1));
        let lines = w.part_lines();
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0].starts_with("Part 1: (-10.000, 10.000) mm, 30.500 x 15.500 mm"),
            "{lines:?}"
        );
        // the file is written beside the picture, and not over one that is there unless the value says so
        assert!(w.save_parts());
        assert_eq!(w.status, Status::Exported("part-parts.csv".into()));
        let written = fs::read_to_string(s.path("part-parts.csv"))
            .unwrap()
            .replace('\r', "");
        assert!(
            written.starts_with("index,x_mm,y_mm,angle_deg,length_mm,width_mm,area_mm2\n1,"),
            "{written}"
        );
        assert!(!w.save_parts());
        assert!(
            matches!(&w.status, Status::Failed(e) if e.starts_with("file_exists")),
            "{:?}",
            w.status
        );
        w.cfg.replace_output = true;
        assert!(w.save_parts());
        // the scale that was set from a mark is used when the field is 0
        assert!(w.set_field(i, "0".into()));
        assert!(!w.measure());
        w.scale = Some(Scale { mm_per_px: 0.25 });
        assert!(w.measure());
        assert!(
            w.part_lines()[0].contains("15.250 x 7.750 mm"),
            "{:?}",
            w.part_lines()
        );
        // with the parts forgotten there is nothing to save
        w.parts.clear();
        assert!(!w.save_parts());
        assert_eq!(w.status, Status::PartsFound(0));
    }

    #[test]
    fn the_flattened_picture_finds_the_marks_that_uneven_light_hides() {
        let s = Scratch::new("flat");
        // the light fades from 250 on the left to 60 on the right, with a dark disc of radius 10 at each place
        let mut g = Gray::blank(400, 200, 220).unwrap();
        for y in 0..200usize {
            for x in 0..400usize {
                g.data[y * 400 + x] = (250.0 - 190.0 * x as f64 / 399.0) as u8;
            }
        }
        for c in 0..4 {
            let (cx, cy) = (50.0 + c as f64 * 100.0, 100.0);
            for y in 0..200usize {
                for x in 0..400usize {
                    if (x as f64 - cx).powi(2) + (y as f64 - cy).powi(2) <= 100.0 {
                        g.data[y * 400 + x] = (f64::from(g.data[y * 400 + x]) * 0.25) as u8;
                    }
                }
            }
        }
        let path = s.path("faded.pgm");
        fs::write(&path, pgm(&g)).unwrap();
        let mut plain = window_with("");
        assert!(plain.open_image(&path));
        let mut flat = window_with("flatten_window = 61\n");
        assert!(flat.open_image(&path));
        assert_eq!(flat.marks.len(), 4);
        assert_ne!(plain.marks.len(), 4);
        // a window that makes no sense is the failure of the search
        let i = flat
            .fields
            .iter()
            .position(|f| f.key == "flatten_window")
            .unwrap();
        assert!(!flat.set_field(i, "2".into()));
    }

    #[test]
    fn the_centres_are_refined_with_a_confidence_when_asked() {
        let s = Scratch::new("refine");
        let mut g = Gray::blank(200, 100, 220).unwrap();
        g.disc(60.0, 50.0, 12.0, 30);
        g.disc(140.0, 50.0, 12.0, 30);
        let path = s.path("two.pgm");
        fs::write(&path, pgm(&g)).unwrap();
        let mut plain = window_with("");
        assert!(plain.open_image(&path));
        assert!(plain.confidence.is_empty());
        let mut sharp = window_with("refine_subpixel = yes\n");
        assert!(sharp.open_image(&path));
        assert_eq!(sharp.confidence.len(), 2);
        assert!(
            sharp.confidence.iter().all(|c| *c > 0.8 && *c <= 1.0),
            "{:?}",
            sharp.confidence
        );
        assert_eq!(sharp.marks.len(), plain.marks.len());
    }

    #[test]
    fn the_scale_comes_from_the_pitch_of_a_target() {
        let s = Scratch::new("pitch");
        let mut g = Gray::blank(260, 200, 220).unwrap();
        for r in 0..4 {
            for c in 0..5 {
                g.disc(40.0 + c as f64 * 40.0, 40.0 + r as f64 * 40.0, 8.0, 30);
            }
        }
        let path = s.path("target.pgm");
        fs::write(&path, pgm(&g)).unwrap();
        let mut w = window_with("");
        assert!(!w.calibrate_pitch());
        assert_eq!(w.status, Status::NeedImage);
        assert!(w.open_image(&path));
        assert!(!w.calibrate_pitch());
        assert_eq!(w.status, Status::NeedPitch);
        assert!(w.status.is_problem());
        let i = w.fields.iter().position(|f| f.key == "pitch_mm").unwrap();
        assert!(w.set_field(i, "2".into()));
        assert!(w.calibrate_pitch());
        let Status::PitchScaleSet { mm_per_px, marks } = w.status.clone() else {
            panic!("{:?}", w.status);
        };
        assert!(
            (mm_per_px - 0.05).abs() < 1e-4 && marks == 20,
            "{:?}",
            w.status
        );
        assert!((w.scale.unwrap().mm_per_px - 0.05).abs() < 1e-4);
        assert!(w.status.text(Lang::En).contains("0.05"));
        // a target that is not a grid is refused with the reason
        w.marks.truncate(3);
        assert!(!w.calibrate_pitch());
        assert!(
            matches!(&w.status, Status::Failed(e) if e.contains("at least 4")),
            "{:?}",
            w.status
        );
    }

    #[test]
    fn the_report_of_the_folder_is_written_beside_the_picture() {
        let s = Scratch::new("batch");
        let mut g = Gray::blank(200, 100, 220).unwrap();
        g.disc(60.0, 50.0, 12.0, 30);
        fs::write(s.path("one.pgm"), pgm(&g)).unwrap();
        fs::write(s.path("two.pgm"), pgm(&Gray::blank(40, 40, 100).unwrap())).unwrap();
        fs::write(s.path("bad.pgm"), b"not a picture").unwrap();
        let mut w = window_with("language = es\n");
        w.lang = Lang::Es;
        assert!(!w.batch_report());
        assert_eq!(w.status, Status::NeedImage);
        assert!(w.open_image(&s.path("one.pgm")));
        assert!(w.batch_report());
        assert_eq!(
            w.status,
            Status::BatchDone {
                pictures: 3,
                name: REPORT_NAME.into()
            }
        );
        let page = fs::read_to_string(s.path(REPORT_NAME)).unwrap();
        assert!(
            page.contains("<html lang=\"es\">") && page.contains("bad.pgm"),
            "{page}"
        );
        assert_eq!(page.matches("<circle").count(), 1);
        // the page is not written over unless the value says so, and a folder with too many pictures is refused
        assert!(!w.batch_report());
        assert!(
            matches!(&w.status, Status::Failed(e) if e.starts_with("file_exists")),
            "{:?}",
            w.status
        );
        w.cfg.replace_output = true;
        assert!(w.batch_report());
        w.cfg.batch_max_files = 2;
        assert!(!w.batch_report());
        assert!(
            matches!(&w.status, Status::Failed(e) if e.starts_with("too_many_files")),
            "{:?}",
            w.status
        );
    }

    /// Looks at the running report until it ends (it must in a minute).
    fn wait_batch(w: &mut Window) {
        let start = std::time::Instant::now();
        while w.poll_batch() {
            assert!(start.elapsed().as_secs() < 60, "the report never ended");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[test]
    fn the_report_in_a_thread_writes_the_same_page_and_the_window_is_free_meanwhile() {
        let s = Scratch::new("batch_thread");
        let mut g = Gray::blank(200, 100, 220).unwrap();
        g.disc(60.0, 50.0, 12.0, 30);
        fs::write(s.path("one.pgm"), pgm(&g)).unwrap();
        fs::write(s.path("bad.pgm"), b"not a picture").unwrap();
        let mut w = window_with("");
        assert!(!w.start_batch_report());
        assert_eq!(w.status, Status::NeedImage);
        assert!(!w.batch_running() && !w.poll_batch());
        assert!(w.open_image(&s.path("one.pgm")));
        assert!(w.start_batch_report());
        assert!(w.batch_running());
        assert!(matches!(w.status, Status::BatchRunning { total: 2, .. }));
        // a second report cannot start while one is running
        assert!(!w.start_batch_report());
        assert_eq!(w.status, Status::BatchBusy);
        wait_batch(&mut w);
        assert!(!w.batch_running());
        assert_eq!(
            w.status,
            Status::BatchDone {
                pictures: 2,
                name: REPORT_NAME.into()
            }
        );
        let threaded = fs::read_to_string(s.path(REPORT_NAME)).unwrap();
        assert_eq!(threaded.matches("<circle").count(), 1);
        // the same page as the one that is made without waiting
        w.cfg.replace_output = true;
        assert!(w.batch_report());
        assert_eq!(fs::read_to_string(s.path(REPORT_NAME)).unwrap(), threaded);
        assert!((w.batch_fraction() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn a_cancelled_report_writes_nothing() {
        let s = Scratch::new("batch_cancel");
        let mut g = Gray::blank(1200, 1200, 220).unwrap();
        g.disc(60.0, 50.0, 12.0, 30);
        for i in 0..6 {
            fs::write(s.path(&format!("p{i}.pgm")), pgm(&g)).unwrap();
        }
        let mut w = window_with("");
        assert!(w.open_image(&s.path("p0.pgm")));
        assert!(w.start_batch_report());
        w.cancel_batch();
        wait_batch(&mut w);
        // the stop was seen before the last picture, or after it: either way the person asked for none
        assert_eq!(w.status, Status::BatchCancelled);
        assert!(!s.path(REPORT_NAME).exists());
        assert!(!w.batch_running());
        // cancelling when nothing runs is nothing
        w.cancel_batch();
        // and a new report can start
        assert!(w.start_batch_report());
        wait_batch(&mut w);
        assert!(matches!(w.status, Status::BatchDone { pictures: 6, .. }));
    }

    #[test]
    fn the_report_in_a_thread_refuses_a_folder_with_too_many_pictures_or_a_page_that_is_there() {
        let s = Scratch::new("batch_thread_refuse");
        for i in 0..3 {
            fs::write(
                s.path(&format!("p{i}.pgm")),
                pgm(&Gray::blank(30, 30, 100).unwrap()),
            )
            .unwrap();
        }
        let mut w = window_with(
            "batch_max_files = 2
",
        );
        assert!(w.open_image(&s.path("p0.pgm")));
        assert!(!w.start_batch_report());
        assert!(matches!(&w.status, Status::Failed(e) if e.starts_with("too_many_files")));
        assert!(!w.batch_running());
        let mut w = window_with("");
        assert!(w.open_image(&s.path("p0.pgm")));
        fs::write(s.path(REPORT_NAME), "old").unwrap();
        assert!(w.start_batch_report());
        wait_batch(&mut w);
        assert!(
            matches!(&w.status, Status::Failed(e) if e.starts_with("file_exists")),
            "{:?}",
            w.status
        );
        assert_eq!(fs::read_to_string(s.path(REPORT_NAME)).unwrap(), "old");
    }

    #[test]
    fn every_new_sentence_of_the_status_is_spoken_in_each_language() {
        let statuses = [
            Status::BatchRunning { done: 2, total: 5 },
            Status::BatchCancelled,
            Status::BatchBusy,
            Status::SquaresFound(2),
            Status::SquaresFound(0),
            Status::CrossesFound(3),
            Status::CrossesFound(0),
            Status::PartsFound(4),
            Status::PartsFound(0),
            Status::PitchScaleSet {
                mm_per_px: 0.05,
                marks: 20,
            },
            Status::NeedPitch,
            Status::BatchDone {
                pictures: 5,
                name: "x.html".into(),
            },
        ];
        for st in &statuses {
            let texts: Vec<String> = Lang::ALL.iter().map(|l| st.text(*l)).collect();
            for a in &texts {
                assert!(!a.is_empty() && !a.contains('{'), "{st:?}: {a}");
            }
            // the Japanese text is never the English one
            assert_ne!(texts[0], texts[5], "{st:?}");
        }
    }
}
