//! The state of the window without a screen: the picture, the marks found in it, the scale, the distortion of the lens, the design and the alignment. Everything the window does is decided
//! here, in code that is tested; the file `gui.rs` (built with the `gui` feature) only draws this state and passes on what the person does.

use crate::align::{fit, isocam_args, pair_nearest, Transform};
use crate::config::Config;
use crate::fiducial::{find_with, Fiducial, Polarity};
use crate::grid::{fit_distortion, Distortion};
use crate::image::{from_bytes, Gray};
use crate::lang::{Lang, Msg};
use crate::points;
use crate::scale::{undistort, Scale};
use crate::viewport::Viewport;
use std::path::{Path, PathBuf};

/// The extensions the window offers for each kind of file.
pub const IMAGE_EXTENSIONS: [&str; 2] = ["png", "pgm"];
pub const DESIGN_EXTENSIONS: [&str; 2] = ["csv", "txt"];
pub const OUTPUT_EXTENSIONS: [&str; 1] = ["txt"];

/// Where in the side panel a field is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Search,
    Scale,
    Grid,
    Align,
}

/// A value of the configuration that the person can edit in the window.
#[derive(Debug, Clone)]
pub struct FieldState {
    /// The key in the configuration file.
    pub key: &'static str,
    pub label: Msg,
    pub section: Section,
    /// The text being typed (it may not be valid yet).
    pub text: String,
}

/// What the status line says.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Ready,
    Opened { name: String, w: usize, h: usize },
    Found(usize),
    ScaleSet(f64),
    DistortionSet(f64),
    DesignLoaded { name: String, points: usize },
    Aligned { dx: f64, dy: f64, turn: f64 },
    Exported(String),
    Failed(String),
    BadValue(Msg),
    NeedImage,
    NeedMark,
    NeedScale,
    NeedDesign,
    NeedAccepted,
    GridCount { need: usize, found: usize },
    TooLarge { name: String, mib: u64 },
    SameFile,
    ConfigProblem(String),
}

/// A number for the screen with `decimals` decimals.
fn show(value: f64, decimals: usize) -> String {
    format!("{value:.decimals$}")
}

impl Status {
    pub fn text(&self, lang: Lang) -> String {
        match self {
            Status::Ready => Msg::Ready.tr(lang).to_string(),
            Status::Opened { name, w, h } => {
                Msg::Opened.with(lang, &[name, &w.to_string(), &h.to_string()])
            }
            Status::Found(0) => Msg::NoMarks.tr(lang).to_string(),
            Status::Found(n) => Msg::Found.with(lang, &[&n.to_string()]),
            Status::ScaleSet(s) => Msg::ScaleSet.with(lang, &[&show(*s, 5)]),
            Status::DistortionSet(k) => Msg::DistortionSet.with(lang, &[&show(*k, 5)]),
            Status::DesignLoaded { name, points } => {
                Msg::DesignLoaded.with(lang, &[name, &points.to_string()])
            }
            Status::Aligned { dx, dy, turn } => {
                Msg::Aligned.with(lang, &[&show(*dx, 3), &show(*dy, 3), &show(*turn, 3)])
            }
            Status::Exported(name) => Msg::Exported.with(lang, &[name]),
            Status::Failed(e) => Msg::Failed.with(lang, &[e]),
            Status::BadValue(m) => Msg::BadValue.with(lang, &[m.tr(lang)]),
            Status::NeedImage => Msg::NeedImage.tr(lang).to_string(),
            Status::NeedMark => Msg::NeedMark.tr(lang).to_string(),
            Status::NeedScale => Msg::NeedScale.tr(lang).to_string(),
            Status::NeedDesign => Msg::NeedDesign.tr(lang).to_string(),
            Status::NeedAccepted => Msg::NeedAccepted.tr(lang).to_string(),
            Status::GridCount { need, found } => {
                Msg::GridCount.with(lang, &[&need.to_string(), &found.to_string()])
            }
            Status::TooLarge { name, mib } => Msg::TooLarge.with(lang, &[name, &mib.to_string()]),
            Status::SameFile => Msg::SameFile.tr(lang).to_string(),
            Status::ConfigProblem(e) => Msg::ConfigProblem.with(lang, &[e]),
        }
    }

    /// Whether the status is something that did not work (it is drawn in red).
    pub fn is_problem(&self) -> bool {
        matches!(
            self,
            Status::Failed(_)
                | Status::BadValue(_)
                | Status::NeedImage
                | Status::NeedMark
                | Status::NeedScale
                | Status::NeedDesign
                | Status::NeedAccepted
                | Status::GridCount { .. }
                | Status::TooLarge { .. }
                | Status::SameFile
                | Status::ConfigProblem(_)
        )
    }
}

/// The picture that was opened.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub name: String,
    pub path: PathBuf,
    pub gray: Gray,
}

/// What came of the comparison with the design.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Accepted,
    /// The marks do not fit the design well enough: one of them is probably not the one it should be.
    RefusedRms,
    /// The board is bigger or smaller than its design by more than allowed.
    RefusedStretch,
}

/// The correction that takes the board from its design to where the camera found it.
#[derive(Debug, Clone)]
pub struct Alignment {
    pub transform: Transform,
    /// The marks in millimetres, in the order of the design points.
    pub measured: Vec<(f64, f64)>,
    pub verdict: Verdict,
}

/// Whether a transform is accepted (the same rule as `align::accept`, told apart by the reason).
pub fn judge(t: &Transform, max_rms_mm: f64, max_stretch: f64) -> Verdict {
    if t.rms_mm > max_rms_mm {
        Verdict::RefusedRms
    } else if (t.scale - 1.0).abs() > max_stretch {
        Verdict::RefusedStretch
    } else {
        Verdict::Accepted
    }
}

/// A design point of an alignment against the mark the camera found for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Residual {
    /// Where the design puts the point in the picture, in pixels.
    pub design: (f64, f64),
    /// Where the camera found its mark, in pixels.
    pub found: (f64, f64),
    /// The distance between the two, in millimetres.
    pub mm: f64,
}

/// The text of a value of the configuration.
fn field_text(cfg: &Config, key: &str) -> String {
    match key {
        "threshold" => cfg
            .threshold
            .map_or_else(|| "auto".to_string(), |t| t.to_string()),
        "min_diameter" => cfg.min_diameter.to_string(),
        "max_diameter" => cfg.max_diameter.to_string(),
        "min_fill" => cfg.min_fill.to_string(),
        "max_elongation" => cfg.max_elongation.to_string(),
        "known_mm" => cfg.known_mm.to_string(),
        "grid_columns" => cfg.grid_columns.to_string(),
        "grid_rows" => cfg.grid_rows.to_string(),
        "max_rms" => cfg.max_rms.to_string(),
        "max_stretch" => cfg.max_stretch.to_string(),
        _ => String::new(),
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Whether two paths are the same file (by the system when both exist, else by their text).
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

pub struct Window {
    pub lang: Lang,
    pub cfg: Config,
    pub fields: Vec<FieldState>,
    pub image: Option<Loaded>,
    /// Raised each time a picture is opened, so that the window knows to make its texture again.
    pub image_serial: u64,
    pub marks: Vec<Fiducial>,
    pub selected: Option<usize>,
    pub scale: Option<Scale>,
    pub distortion: Option<Distortion>,
    /// Whether the marks are corrected with the distortion before they are used.
    pub use_distortion: bool,
    pub design: Vec<(f64, f64)>,
    pub design_path: Option<PathBuf>,
    pub alignment: Option<Alignment>,
    pub viewport: Viewport,
    pub status: Status,
}

impl Window {
    pub fn new(lang: Lang, cfg: Config, config_problem: Option<String>) -> Window {
        let fields = [
            ("threshold", Msg::Threshold, Section::Search),
            ("min_diameter", Msg::MinDiameter, Section::Search),
            ("max_diameter", Msg::MaxDiameter, Section::Search),
            ("min_fill", Msg::MinFill, Section::Search),
            ("max_elongation", Msg::MaxElongation, Section::Search),
            ("known_mm", Msg::KnownMm, Section::Scale),
            ("grid_columns", Msg::GridColumns, Section::Grid),
            ("grid_rows", Msg::GridRows, Section::Grid),
            ("max_rms", Msg::MaxRms, Section::Align),
            ("max_stretch", Msg::MaxStretch, Section::Align),
        ]
        .into_iter()
        .map(|(key, label, section)| FieldState {
            key,
            label,
            section,
            text: field_text(&cfg, key),
        })
        .collect();
        Window {
            lang,
            cfg,
            fields,
            image: None,
            image_serial: 0,
            marks: Vec::new(),
            selected: None,
            scale: None,
            distortion: None,
            use_distortion: false,
            design: Vec::new(),
            design_path: None,
            alignment: None,
            viewport: Viewport::default(),
            status: config_problem.map_or(Status::Ready, Status::ConfigProblem),
        }
    }

    /// Takes the text typed in a field: it is used when it is a valid value together with the others, else the status says which field is wrong and the value in use does not change.
    pub fn set_field(&mut self, index: usize, text: String) -> bool {
        let Some(field) = self.fields.get_mut(index) else {
            return false;
        };
        field.text.clone_from(&text);
        let (key, label) = (field.key, field.label);
        let mut trial = self.cfg.clone();
        let valid = trial.set(key, &text).and_then(|()| trial.validate());
        match valid {
            Ok(()) => {
                self.cfg = trial;
                true
            }
            Err(_) => {
                self.status = Status::BadValue(label);
                false
            }
        }
    }

    /// Whether the marks are light on a dark board.
    pub fn light_marks(&self) -> bool {
        self.cfg.polarity == Polarity::Light
    }

    pub fn set_light_marks(&mut self, light: bool) {
        self.cfg.polarity = if light {
            Polarity::Light
        } else {
            Polarity::Dark
        };
    }

    pub fn set_allow_scale(&mut self, allow: bool) {
        self.cfg.allow_scale = allow;
        self.alignment = None;
    }

    /// The centre of the picture in pixels and the distance from it to a corner, which the correction of the lens is worked from.
    fn picture(&self) -> Option<((f64, f64), f64)> {
        let img = self.image.as_ref()?;
        let centre = (img.gray.w as f64 / 2.0, img.gray.h as f64 / 2.0);
        Some((centre, centre.0.hypot(centre.1)))
    }

    fn read_limited(&self, path: &Path) -> Result<Vec<u8>, Status> {
        let name = file_name(path);
        let limit = self.cfg.max_file_mib;
        let size = std::fs::metadata(path)
            .map_err(|e| Status::Failed(format!("{name}: {e}")))?
            .len();
        if size > limit.saturating_mul(1_048_576) {
            return Err(Status::TooLarge { name, mib: limit });
        }
        std::fs::read(path).map_err(|e| Status::Failed(format!("{name}: {e}")))
    }

    /// Opens a PNG or PGM picture and looks for the marks in it. The scale, the distortion and the alignment of the earlier picture are forgotten.
    pub fn open_image(&mut self, path: &Path) -> bool {
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
                self.image = Some(Loaded {
                    name: name.clone(),
                    path: path.to_path_buf(),
                    gray,
                });
                self.image_serial += 1;
                self.scale = None;
                self.detect();
                self.status = Status::Opened { name, w, h };
                true
            }
            Err(e) => {
                self.status = Status::Failed(format!("{name}: {e}"));
                false
            }
        }
    }

    /// Looks for the marks again with the values of the fields. What was worked out from the old marks (the distortion and the alignment) is forgotten; the scale stays.
    pub fn detect(&mut self) -> bool {
        let result = match &self.image {
            Some(img) => find_with(&img.gray, &self.cfg.settings(), self.cfg.threshold),
            None => {
                self.status = Status::NeedImage;
                return false;
            }
        };
        match result {
            Ok(marks) => {
                self.status = Status::Found(marks.len());
                self.marks = marks;
                self.selected = None;
                self.distortion = None;
                self.use_distortion = false;
                self.alignment = None;
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// The place of a mark in pixels, corrected for the lens when that is on.
    pub fn mark_px(&self, mark: &Fiducial) -> (f64, f64) {
        match (self.use_distortion, self.distortion, self.picture()) {
            (true, Some(d), Some((centre, norm))) => {
                undistort((mark.x, mark.y), centre, d.k1, norm)
            }
            _ => (mark.x, mark.y),
        }
    }

    /// The marks in millimetres from the centre of the picture with y going up; none without a scale.
    pub fn marks_mm(&self) -> Option<Vec<(f64, f64)>> {
        let scale = self.scale?;
        let (centre, _) = self.picture()?;
        Some(
            self.marks
                .iter()
                .map(|m| {
                    let p = self.mark_px(m);
                    scale.point(p.0, p.1, centre)
                })
                .collect(),
        )
    }

    /// Sets the scale from the selected mark, whose size is the one in the field `known_mm`.
    pub fn calibrate_selected(&mut self) -> bool {
        if self.image.is_none() {
            self.status = Status::NeedImage;
            return false;
        }
        let Some(mark) = self.selected.and_then(|i| self.marks.get(i)) else {
            self.status = Status::NeedMark;
            return false;
        };
        match Scale::from_known_diameter(mark.diameter, self.cfg.known_mm) {
            Ok(s) => {
                self.scale = Some(s);
                self.alignment = None;
                self.status = Status::ScaleSet(s.mm_per_px);
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Works out the distortion of the lens from the marks, which must be the grid of the target (columns x rows marks).
    pub fn fit_grid(&mut self) -> bool {
        let Some((centre, norm)) = self.picture() else {
            self.status = Status::NeedImage;
            return false;
        };
        let (columns, rows) = (self.cfg.grid_columns, self.cfg.grid_rows);
        if self.marks.len() != columns * rows {
            self.status = Status::GridCount {
                need: columns * rows,
                found: self.marks.len(),
            };
            return false;
        }
        let raw: Vec<(f64, f64)> = self.marks.iter().map(|m| (m.x, m.y)).collect();
        match fit_distortion(&raw, columns, rows, centre, norm) {
            Ok(d) => {
                self.distortion = Some(d);
                self.alignment = None;
                self.status = Status::DistortionSet(d.k1);
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Corrects the marks with the distortion (or stops doing it).
    pub fn set_use_distortion(&mut self, on: bool) {
        self.use_distortion = on && self.distortion.is_some();
        self.alignment = None;
    }

    /// Opens a file of the points of the design (`x,y` in millimetres, one per line).
    pub fn load_design(&mut self, path: &Path) -> bool {
        let name = file_name(path);
        let bytes = match self.read_limited(path) {
            Ok(b) => b,
            Err(s) => {
                self.status = s;
                return false;
            }
        };
        let text = String::from_utf8_lossy(&bytes);
        match points::parse(&text, &name) {
            Ok(p) => {
                self.status = Status::DesignLoaded {
                    name,
                    points: p.len(),
                };
                self.design = p;
                self.design_path = Some(path.to_path_buf());
                self.alignment = None;
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Works out the turn, the shift and (if allowed) the size that take the design to the marks. The marks are paired with the design points by their places, so their order does not matter.
    pub fn align(&mut self) -> bool {
        if self.image.is_none() {
            self.status = Status::NeedImage;
            return false;
        }
        let Some(measured) = self.marks_mm() else {
            self.status = Status::NeedScale;
            return false;
        };
        if self.design.is_empty() {
            self.status = Status::NeedDesign;
            return false;
        }
        let result = pair_nearest(&self.design, &measured).and_then(|paired| {
            let t = fit(&self.design, &paired, self.cfg.allow_scale)?;
            Ok((t, paired))
        });
        match result {
            Ok((t, paired)) => {
                let verdict = judge(&t, self.cfg.max_rms, self.cfg.max_stretch);
                self.status = Status::Aligned {
                    dx: t.dx,
                    dy: t.dy,
                    turn: t.theta_deg,
                };
                self.alignment = Some(Alignment {
                    transform: t,
                    measured: paired,
                    verdict,
                });
                true
            }
            Err(e) => {
                self.alignment = None;
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// The line about the scale.
    pub fn scale_line(&self) -> String {
        match self.scale {
            Some(s) => Msg::ScaleLine.with(self.lang, &[&show(s.mm_per_px, 5)]),
            None => Msg::NoScale.tr(self.lang).to_string(),
        }
    }

    /// The line about the distortion, once it is worked out.
    pub fn distortion_line(&self) -> Option<String> {
        self.distortion.map(|d| {
            Msg::DistortionLine.with(
                self.lang,
                &[
                    &show(d.k1, 5),
                    &show(d.rms_before, 2),
                    &show(d.rms_after, 2),
                ],
            )
        })
    }

    /// The line about the design.
    pub fn design_line(&self) -> String {
        match &self.design_path {
            Some(p) if !self.design.is_empty() => {
                Msg::DesignLine.with(self.lang, &[&file_name(p), &self.design.len().to_string()])
            }
            _ => Msg::NoDesign.tr(self.lang).to_string(),
        }
    }

    /// The lines that tell what the alignment came to, each with whether it is a problem.
    pub fn alignment_lines(&self) -> Vec<(String, bool)> {
        let Some(a) = &self.alignment else {
            return Vec::new();
        };
        let (t, lang) = (&a.transform, self.lang);
        let mut lines = vec![
            (
                Msg::ShiftLine.with(lang, &[&show(t.dx, 4), &show(t.dy, 4)]),
                false,
            ),
            (Msg::TurnLine.with(lang, &[&show(t.theta_deg, 4)]), false),
            (Msg::SizeLine.with(lang, &[&show(t.scale, 5)]), false),
            (Msg::ResidualLine.with(lang, &[&show(t.rms_mm, 4)]), false),
        ];
        let verdict = match a.verdict {
            Verdict::Accepted => (Msg::Accepted.tr(lang).to_string(), false),
            Verdict::RefusedRms => (
                Msg::RefusedRms.with(lang, &[&show(t.rms_mm, 3), &self.cfg.max_rms.to_string()]),
                true,
            ),
            Verdict::RefusedStretch => (
                Msg::RefusedStretch.with(
                    lang,
                    &[
                        &show((t.scale - 1.0).abs() * 100.0, 2),
                        &show(self.cfg.max_stretch * 100.0, 2),
                    ],
                ),
                true,
            ),
        };
        lines.push(verdict);
        lines
    }

    /// The options of `eh3d-isocam` for the marks, once the alignment is accepted.
    pub fn isocam_lines(&self) -> Option<Vec<String>> {
        let a = self.alignment.as_ref()?;
        if a.verdict != Verdict::Accepted {
            return None;
        }
        Some(isocam_args(&self.design, &a.measured))
    }

    /// The name offered for the file of the options of ISOCAM.
    pub fn suggested_output(&self) -> String {
        match &self.image {
            Some(img) => {
                let stem = Path::new(&img.name)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                format!("{stem}-isocam.txt")
            }
            None => "isocam-options.txt".to_string(),
        }
    }

    /// Writes the options of ISOCAM to a file. The picture and the design that were opened are never overwritten.
    pub fn export_isocam(&mut self, path: &Path) -> bool {
        let Some(lines) = self.isocam_lines() else {
            self.status = Status::NeedAccepted;
            return false;
        };
        let opened_here = self
            .image
            .iter()
            .map(|i| i.path.as_path())
            .chain(self.design_path.as_deref())
            .any(|p| same_file(p, path));
        if opened_here {
            self.status = Status::SameFile;
            return false;
        }
        let mut text = lines.join("\n");
        text.push('\n');
        match std::fs::write(path, text) {
            Ok(()) => {
                self.status = Status::Exported(file_name(path));
                true
            }
            Err(e) => {
                self.status = Status::Failed(format!("{}: {e}", file_name(path)));
                false
            }
        }
    }

    /// The design points where they should be in the picture, in pixels, once the alignment and the scale are known.
    pub fn design_px(&self) -> Vec<(f64, f64)> {
        let (Some(a), Some(s), Some((centre, _))) = (&self.alignment, self.scale, self.picture())
        else {
            return Vec::new();
        };
        self.design
            .iter()
            .map(|d| {
                let m = a.transform.apply(*d);
                (centre.0 + m.0 / s.mm_per_px, centre.1 - m.1 / s.mm_per_px)
            })
            .collect()
    }

    /// For each design point of an alignment, where the design puts it and where the camera found its mark, both in pixels of the picture, and how far apart they are in
    /// millimetres: what the window draws as the lines of the alignment. None without a picture, a scale and an alignment.
    pub fn residuals(&self) -> Vec<Residual> {
        let (Some(a), Some(s), Some((centre, _))) = (&self.alignment, self.scale, self.picture())
        else {
            return Vec::new();
        };
        let to_px = |m: (f64, f64)| (centre.0 + m.0 / s.mm_per_px, centre.1 - m.1 / s.mm_per_px);
        self.design
            .iter()
            .zip(&a.measured)
            .map(|(d, found)| {
                let placed = a.transform.apply(*d);
                Residual {
                    design: to_px(placed),
                    found: to_px(*found),
                    mm: (placed.0 - found.0).hypot(placed.1 - found.1),
                }
            })
            .collect()
    }

    /// What a picture of the window asks for besides opening the files: the mark number `mark` (from 1) sets the scale, and `align` makes the alignment with the design.
    /// The answer for what cannot be done is the reason, so that a picture is never taken of a state that was not asked for.
    pub fn prepare_shot(&mut self, mark: Option<usize>, align: bool) -> Result<(), String> {
        if let Some(n) = mark {
            if n == 0 || n > self.marks.len() {
                return Err(format!(
                    "--mark {n}: the picture has {} marks, numbered from 1",
                    self.marks.len()
                ));
            }
            self.selected = Some(n - 1);
            if !self.calibrate_selected() {
                return Err(format!("--mark {n}: the scale could not be set from it"));
            }
        }
        if align && !self.align() {
            return Err(
                "--align needs a picture, a scale (--mark N) and a design file with as many points as marks"
                    .to_owned(),
            );
        }
        Ok(())
    }

    /// Fits the whole picture in a view of `w` x `h` pixels.
    pub fn fit_view(&mut self, w: f64, h: f64) {
        if let Some(img) = &self.image {
            self.viewport = Viewport::fit(img.gray.w as f64, img.gray.h as f64, w, h, 12.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A folder of its own for each test, removed at the end.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let dir =
                std::env::temp_dir().join(format!("eh3d-vision-test-{}-{tag}", std::process::id()));
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
    fn board(w: usize, h: usize, places: &[(f64, f64)]) -> Gray {
        let mut g = Gray::blank(w, h, 220).unwrap();
        for p in places {
            g.disc(p.0, p.1, 10.0, 20);
        }
        g
    }

    const FOUR: [(f64, f64); 4] = [(60.0, 60.0), (240.0, 60.0), (240.0, 140.0), (60.0, 140.0)];

    fn window() -> Window {
        Window::new(Lang::En, Config::default(), None)
    }

    /// A window with the picture of four marks open, with the scale set from the first mark taken as 1 mm.
    fn calibrated(s: &Scratch) -> Window {
        let path = s.path("four.pgm");
        fs::write(&path, pgm(&board(300, 200, &FOUR))).unwrap();
        let mut w = window();
        assert!(w.open_image(&path));
        assert_eq!(w.marks.len(), 4);
        w.selected = Some(0);
        assert!(w.calibrate_selected());
        w
    }

    #[test]
    fn a_new_window_waits_for_a_picture() {
        let w = window();
        assert_eq!(w.status, Status::Ready);
        assert_eq!(w.fields.len(), 10);
        assert_eq!(w.fields[0].text, "auto");
        assert_eq!(w.fields[1].text, "8");
        assert!(!w.light_marks());
    }

    #[test]
    fn a_configuration_that_could_not_be_used_is_said() {
        let w = Window::new(Lang::En, Config::default(), Some("line 3".into()));
        assert!(w.status.is_problem());
        assert!(w.status.text(Lang::En).contains("line 3"));
    }

    #[test]
    fn opening_a_picture_finds_its_marks() {
        let s = Scratch::new("open");
        let path = s.path("four.pgm");
        fs::write(&path, pgm(&board(300, 200, &FOUR))).unwrap();
        let mut w = window();
        assert!(w.open_image(&path));
        assert_eq!(w.marks.len(), 4);
        assert_eq!(w.image_serial, 1);
        assert_eq!(
            w.status,
            Status::Opened {
                name: "four.pgm".into(),
                w: 300,
                h: 200
            }
        );
        for m in &w.marks {
            assert!((m.diameter - 20.0).abs() < 1.5, "diameter {}", m.diameter);
        }
    }

    #[test]
    fn a_file_that_is_not_a_picture_is_an_error_and_keeps_the_old_picture() {
        let s = Scratch::new("bad");
        let good = s.path("good.pgm");
        fs::write(&good, pgm(&board(300, 200, &FOUR))).unwrap();
        let junk = s.path("junk.png");
        fs::write(&junk, b"this is not a picture").unwrap();
        let mut w = window();
        assert!(w.open_image(&good));
        assert!(!w.open_image(&junk));
        assert!(w.status.is_problem());
        assert_eq!(w.marks.len(), 4);
        assert!(!w.open_image(&s.path("missing.png")));
    }

    #[test]
    fn a_file_over_the_limit_is_refused_before_it_is_read() {
        let s = Scratch::new("big");
        let path = s.path("big.pgm");
        fs::write(&path, pgm(&board(300, 200, &FOUR))).unwrap();
        let mut w = window();
        w.cfg.max_file_mib = 0;
        assert!(!w.open_image(&path));
        assert!(matches!(w.status, Status::TooLarge { .. }));
    }

    #[test]
    fn the_fields_change_what_is_found() {
        let s = Scratch::new("fields");
        let path = s.path("four.pgm");
        fs::write(&path, pgm(&board(300, 200, &FOUR))).unwrap();
        let mut w = window();
        w.open_image(&path);
        // marks of 20 pixels are no longer in the range
        assert!(w.set_field(1, "30".into()));
        assert!(w.set_field(2, "60".into()));
        assert!(w.detect());
        assert!(w.marks.is_empty());
        assert_eq!(w.status.text(Lang::En), "No marks found");
        // a fixed threshold of 0 leaves nothing dark enough to be a mark
        assert!(w.set_field(1, "8".into()));
        assert!(w.set_field(0, "0".into()));
        assert!(w.detect());
        assert!(w.marks.is_empty());
        assert!(w.set_field(0, "auto".into()));
        assert!(w.detect());
        assert_eq!(w.marks.len(), 4);
    }

    #[test]
    fn light_marks_on_a_dark_board_are_found_with_the_switch() {
        let s = Scratch::new("light");
        let mut g = Gray::blank(300, 200, 30).unwrap();
        g.disc(100.0, 100.0, 12.0, 230);
        let path = s.path("light.pgm");
        fs::write(&path, pgm(&g)).unwrap();
        let mut w = window();
        w.open_image(&path);
        assert!(w.marks.is_empty());
        w.set_light_marks(true);
        assert!(w.detect());
        assert_eq!(w.marks.len(), 1);
    }

    #[test]
    fn a_value_that_makes_no_sense_is_refused_and_the_old_one_stays() {
        let mut w = window();
        assert!(!w.set_field(1, "abc".into()));
        assert_eq!(w.status, Status::BadValue(Msg::MinDiameter));
        assert_eq!(w.cfg.min_diameter, 8.0);
        // the smallest diameter above the largest one is not valid together with it
        assert!(!w.set_field(1, "500".into()));
        assert_eq!(w.cfg.min_diameter, 8.0);
        assert!(!w.set_field(99, "1".into()));
        assert!(w.set_field(3, "0.7".into()));
        assert_eq!(w.cfg.min_fill, 0.7);
    }

    #[test]
    fn the_scale_comes_from_the_selected_mark() {
        let s = Scratch::new("scale");
        let mut w = window();
        assert!(!w.calibrate_selected());
        assert_eq!(w.status, Status::NeedImage);
        let path = s.path("four.pgm");
        fs::write(&path, pgm(&board(300, 200, &FOUR))).unwrap();
        w.open_image(&path);
        assert!(!w.calibrate_selected());
        assert_eq!(w.status, Status::NeedMark);
        assert!(w.set_field(5, "2".into()));
        w.selected = Some(0);
        assert!(w.calibrate_selected());
        let mm = w.scale.unwrap().mm_per_px;
        assert!((mm - 0.1).abs() < 0.01, "scale {mm}");
        let marks = w.marks_mm().unwrap();
        assert_eq!(marks.len(), 4);
        // the picture is 300 x 200 pixels, so its centre is (150, 100): the mark at (60, 60) is left of it and above it
        let first = w
            .marks
            .iter()
            .position(|m| (m.x - 60.0).abs() < 1.0 && (m.y - 60.0).abs() < 1.0)
            .unwrap();
        assert!((-10.0..-8.0).contains(&marks[first].0));
        assert!((3.0..5.0).contains(&marks[first].1));
    }

    #[test]
    fn a_picture_without_a_scale_has_no_millimetres() {
        let s = Scratch::new("noscale");
        let path = s.path("four.pgm");
        fs::write(&path, pgm(&board(300, 200, &FOUR))).unwrap();
        let mut w = window();
        w.open_image(&path);
        assert!(w.marks_mm().is_none());
        assert_eq!(w.scale_line(), "No scale yet");
        assert!(!w.align());
        assert_eq!(w.status, Status::NeedScale);
    }

    #[test]
    fn the_design_is_aligned_with_the_marks_in_any_order() {
        let s = Scratch::new("align");
        let mut w = calibrated(&s);
        let measured = w.marks_mm().unwrap();
        let known = Transform {
            dx: 1.0,
            dy: -0.5,
            theta_deg: 1.5,
            scale: 1.0,
            rms_mm: 0.0,
        };
        // the design is the marks taken back by the known correction, written in the opposite order
        let mut text = String::from("# the design\n");
        for m in measured.iter().rev() {
            let d = known.inverse(*m);
            text.push_str(&format!("{},{}\n", d.0, d.1));
        }
        let file = s.path("design.csv");
        fs::write(&file, text).unwrap();
        assert!(w.load_design(&file));
        assert_eq!(w.design.len(), 4);
        assert!(w.align());
        let a = w.alignment.as_ref().unwrap();
        assert_eq!(a.verdict, Verdict::Accepted);
        assert!((a.transform.theta_deg - 1.5).abs() < 1e-6);
        assert!((a.transform.dx - 1.0).abs() < 1e-6);
        assert!((a.transform.dy + 0.5).abs() < 1e-6);
        assert!(a.transform.rms_mm < 1e-6);
        let lines = w.alignment_lines();
        assert_eq!(lines.len(), 5);
        assert!(lines.iter().all(|(_, problem)| !problem));
        assert!(lines[1].0.starts_with("Turn: 1.5"));
        assert_eq!(w.design_px().len(), 4);
    }

    /// A window with a picture of 200 x 100 pixels at half a millimetre a pixel, and an alignment worked out by hand.
    fn hand_made_alignment() -> Window {
        let mut w = window();
        w.image = Some(Loaded {
            name: "a.pgm".into(),
            path: PathBuf::from("a.pgm"),
            gray: Gray::blank(200, 100, 220).unwrap(),
        });
        w.scale = Some(Scale { mm_per_px: 0.5 });
        w.design = vec![(0.0, 0.0), (10.0, 0.0)];
        w.alignment = Some(Alignment {
            transform: Transform {
                dx: 1.0,
                dy: 0.0,
                theta_deg: 0.0,
                scale: 1.0,
                rms_mm: 0.0,
            },
            measured: vec![(1.0, 0.0), (11.0, 0.5)],
            verdict: Verdict::Accepted,
        });
        w
    }

    #[test]
    fn the_lines_of_the_alignment_join_the_design_to_the_marks_found() {
        let w = hand_made_alignment();
        // the centre of the picture is (100, 50); the design puts the points at 1 mm and 11 mm, which are 2 and 22 pixels to the right of it
        let r = w.residuals();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].design, (102.0, 50.0));
        assert_eq!(r[0].found, (102.0, 50.0));
        assert_eq!(r[0].mm, 0.0);
        assert_eq!(r[1].design, (122.0, 50.0));
        // the second mark was found half a millimetre up, which is one pixel up in the picture
        assert_eq!(r[1].found, (122.0, 49.0));
        assert_eq!(r[1].mm, 0.5);
    }

    #[test]
    fn there_are_no_lines_without_a_scale_or_an_alignment() {
        let mut w = hand_made_alignment();
        w.scale = None;
        assert!(w.residuals().is_empty());
        let mut w = hand_made_alignment();
        w.alignment = None;
        assert!(w.residuals().is_empty());
        assert!(window().residuals().is_empty());
    }

    #[test]
    fn a_picture_can_ask_for_the_scale_and_the_alignment_and_is_told_what_cannot_be_done() {
        let s = Scratch::new("shot-steps");
        let mut w = calibrated(&s);
        // nothing asked, nothing done
        assert!(w.prepare_shot(None, false).is_ok());
        let e = w.prepare_shot(Some(0), false).unwrap_err();
        assert!(e.contains("numbered from 1"), "{e}");
        let e = w.prepare_shot(Some(5), false).unwrap_err();
        assert!(e.contains("the picture has 4 marks"), "{e}");
        // there is no design yet
        let e = w.prepare_shot(Some(2), true).unwrap_err();
        assert!(e.contains("--align needs"), "{e}");
        assert_eq!(w.selected, Some(1));
        let file = s.path("design.csv");
        let text: String = w
            .marks_mm()
            .unwrap()
            .iter()
            .map(|m| format!("{},{}\n", m.0, m.1))
            .collect();
        fs::write(&file, text).unwrap();
        assert!(w.load_design(&file));
        assert!(w.prepare_shot(Some(1), true).is_ok());
        assert!(w.alignment.is_some());
        assert_eq!(w.residuals().len(), 4);
    }

    #[test]
    fn the_options_for_isocam_are_written_only_when_the_alignment_is_accepted() {
        let s = Scratch::new("export");
        let mut w = calibrated(&s);
        let measured = w.marks_mm().unwrap();
        let known = Transform {
            dx: 0.5,
            dy: 0.25,
            theta_deg: -1.0,
            scale: 1.0,
            rms_mm: 0.0,
        };
        let mut text = String::new();
        for m in &measured {
            let d = known.inverse(*m);
            text.push_str(&format!("{},{}\n", d.0, d.1));
        }
        let file = s.path("design.csv");
        fs::write(&file, text).unwrap();
        w.load_design(&file);
        // nothing to export before the alignment
        let out = s.path("options.txt");
        assert!(!w.export_isocam(&out));
        assert_eq!(w.status, Status::NeedAccepted);
        assert!(w.align());
        // the design and the picture are never overwritten
        assert!(!w.export_isocam(&file));
        assert_eq!(w.status, Status::SameFile);
        assert!(!w.export_isocam(&s.path("four.pgm")));
        assert!(w.export_isocam(&out));
        let written = fs::read_to_string(&out).unwrap();
        assert_eq!(written.lines().count(), 4);
        assert!(written.lines().all(|l| l.starts_with("--fiducial ")));
        assert_eq!(w.suggested_output(), "four-isocam.txt");
    }

    #[test]
    fn a_design_that_does_not_fit_is_refused_with_its_reason_and_not_exported() {
        let s = Scratch::new("refused");
        let mut w = calibrated(&s);
        let measured = w.marks_mm().unwrap();
        let mut text = String::new();
        for (k, m) in measured.iter().enumerate() {
            // one of the points is 6 mm away from where it should be
            let x = if k == 0 { m.0 + 6.0 } else { m.0 };
            text.push_str(&format!("{},{}\n", x, m.1));
        }
        let file = s.path("design.csv");
        fs::write(&file, text).unwrap();
        w.load_design(&file);
        assert!(w.align());
        assert_eq!(w.alignment.as_ref().unwrap().verdict, Verdict::RefusedRms);
        let lines = w.alignment_lines();
        assert!(lines.last().unwrap().1);
        assert!(lines.last().unwrap().0.starts_with("Refused"));
        assert!(w.isocam_lines().is_none());
        assert!(!w.export_isocam(&s.path("options.txt")));
        assert_eq!(w.status, Status::NeedAccepted);
    }

    #[test]
    fn the_verdict_is_the_one_of_the_command_line() {
        use crate::align::accept;
        let cases = [(0.05, 1.0), (0.5, 1.0), (0.05, 1.05), (0.05, 0.9)];
        for (rms, scale) in cases {
            let t = Transform {
                dx: 0.0,
                dy: 0.0,
                theta_deg: 0.0,
                scale,
                rms_mm: rms,
            };
            let accepted = judge(&t, 0.1, 0.01) == Verdict::Accepted;
            assert_eq!(accepted, accept(&t, 0.1, 0.01).is_ok(), "{rms} {scale}");
        }
        let big = Transform {
            dx: 0.0,
            dy: 0.0,
            theta_deg: 0.0,
            scale: 1.05,
            rms_mm: 0.0,
        };
        assert_eq!(judge(&big, 0.1, 0.01), Verdict::RefusedStretch);
    }

    #[test]
    fn a_design_with_a_bad_line_or_no_points_is_not_loaded() {
        let s = Scratch::new("design");
        let mut w = window();
        let bad = s.path("bad.csv");
        fs::write(&bad, "1,2\n3;4\n").unwrap();
        assert!(!w.load_design(&bad));
        assert!(w.status.text(Lang::En).contains("line 2"));
        assert_eq!(w.design_line(), "No design loaded");
        let good = s.path("good.csv");
        fs::write(&good, "1,2\n3,4\n").unwrap();
        assert!(w.load_design(&good));
        assert_eq!(w.design_line(), "Design: good.csv, 2 points");
        assert!(!w.align());
        assert_eq!(w.status, Status::NeedImage);
    }

    #[test]
    fn the_distortion_of_a_grid_is_found_in_the_picture() {
        let s = Scratch::new("grid");
        let (width, height) = (640usize, 480usize);
        let centre = (width as f64 / 2.0, height as f64 / 2.0);
        let norm = centre.0.hypot(centre.1);
        let k1 = -0.05;
        let mut places = Vec::new();
        for r in 0..4 {
            for c in 0..5 {
                let ideal = (
                    centre.0 + (c as f64 - 2.0) * 100.0,
                    centre.1 + (r as f64 - 1.5) * 100.0,
                );
                // where a lens with k1 puts the point
                let mut m = ideal;
                for _ in 0..60 {
                    let r2 = ((m.0 - centre.0).powi(2) + (m.1 - centre.1).powi(2)) / (norm * norm);
                    m = (
                        centre.0 + (ideal.0 - centre.0) / (1.0 + k1 * r2),
                        centre.1 + (ideal.1 - centre.1) / (1.0 + k1 * r2),
                    );
                }
                places.push(m);
            }
        }
        let path = s.path("grid.pgm");
        fs::write(&path, pgm(&board(width, height, &places))).unwrap();
        let mut w = window();
        assert!(!w.fit_grid());
        assert_eq!(w.status, Status::NeedImage);
        w.open_image(&path);
        assert_eq!(w.marks.len(), 20);
        assert!(w.fit_grid());
        let d = w.distortion.unwrap();
        assert!((d.k1 - k1).abs() < 0.01, "k1 {}", d.k1);
        assert!(d.rms_after < d.rms_before);
        assert!(w.distortion_line().unwrap().starts_with("k1 = "));
        // with the correction on, the marks move toward a straight grid
        let raw = (w.marks[0].x, w.marks[0].y);
        w.set_use_distortion(true);
        assert!(w.use_distortion);
        assert_ne!(w.mark_px(&w.marks[0]), raw);
        // a different number of columns does not match the marks
        assert!(w.set_field(6, "6".into()));
        assert!(!w.fit_grid());
        assert_eq!(
            w.status,
            Status::GridCount {
                need: 24,
                found: 20
            }
        );
    }

    #[test]
    fn looking_for_the_marks_again_forgets_what_was_worked_out_from_the_old_ones() {
        let s = Scratch::new("forget");
        let mut w = calibrated(&s);
        w.distortion = Some(Distortion {
            k1: 0.01,
            rms_before: 1.0,
            rms_after: 0.5,
        });
        w.use_distortion = true;
        assert!(w.detect());
        assert!(w.distortion.is_none());
        assert!(!w.use_distortion);
        assert!(w.alignment.is_none());
        // the scale stays: it does not depend on which marks are found
        assert!(w.scale.is_some());
    }

    #[test]
    fn fitting_the_view_puts_the_whole_picture_in_it() {
        let s = Scratch::new("view");
        let mut w = calibrated(&s);
        w.fit_view(900.0, 700.0);
        assert_eq!((w.viewport.centre_x, w.viewport.centre_y), (150.0, 100.0));
        assert_eq!(w.viewport.zoom, 876.0 / 300.0);
    }

    #[test]
    fn every_status_has_a_text_in_each_language() {
        let all = [
            Status::Ready,
            Status::Opened {
                name: "a.png".into(),
                w: 1,
                h: 2,
            },
            Status::Found(0),
            Status::Found(3),
            Status::ScaleSet(0.1),
            Status::DistortionSet(0.01),
            Status::DesignLoaded {
                name: "d.csv".into(),
                points: 4,
            },
            Status::Aligned {
                dx: 1.0,
                dy: 2.0,
                turn: 3.0,
            },
            Status::Exported("o.txt".into()),
            Status::Failed("x".into()),
            Status::BadValue(Msg::MinFill),
            Status::NeedImage,
            Status::NeedMark,
            Status::NeedScale,
            Status::NeedDesign,
            Status::NeedAccepted,
            Status::GridCount { need: 20, found: 3 },
            Status::TooLarge {
                name: "b.png".into(),
                mib: 64,
            },
            Status::SameFile,
            Status::ConfigProblem("y".into()),
        ];
        for st in &all {
            for l in Lang::ALL {
                let t = st.text(l);
                assert!(!t.trim().is_empty(), "{st:?} in {}", l.code());
                assert!(
                    !t.contains('{'),
                    "{st:?} in {} has a place left: {t}",
                    l.code()
                );
            }
        }
        assert_eq!(Status::Found(3).text(Lang::Pt), "3 marcas encontradas");
    }
}
