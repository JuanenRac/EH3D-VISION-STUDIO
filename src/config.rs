//! The configuration file: every value that is tuned (what a mark looks like, the grid of the target, how much a fit may be off) is written in a plain `key = value` file, not in the code.
//! The file is `vision.cfg`; the variable `EH3D_VISION_CONFIG` names another one. The values that are written here are the ones that the program starts with, and the file that comes with
//! the program (`examples/vision.cfg`) holds all of them with their meaning. A file that is not there means the defaults; a file with a mistake is refused with the line and the reason.

use crate::fiducial::{Polarity, Settings};
use crate::offset::OffsetSettings;
use crate::quality::QualitySettings;
use crate::ring::RingSettings;
use std::path::{Path, PathBuf};

/// The name of the file that is looked for when nothing else is said.
pub const FILE_NAME: &str = "vision.cfg";
/// The variable of the system that names another file.
pub const FILE_VARIABLE: &str = "EH3D_VISION_CONFIG";

/// Every key of the file, in the order the file lists them.
pub const KEYS: [&str; 66] = [
    "language",
    "polarity",
    "threshold",
    "min_diameter",
    "max_diameter",
    "min_fill",
    "max_elongation",
    "known_mm",
    "grid_columns",
    "grid_rows",
    "allow_scale",
    "max_rms",
    "max_stretch",
    "max_file_mib",
    "show_residuals",
    "refine_subpixel",
    "full_contrast",
    "flatten_window",
    "mm_per_px",
    "part_min_area",
    "part_max_area",
    "max_parts",
    "cross_min_size",
    "cross_max_size",
    "cross_max_elongation",
    "pitch_tolerance",
    "batch_max_files",
    "square_min_fill",
    "cross_min_score",
    "lens_k1",
    "lens_k2",
    "lens_max_rms",
    "package_table",
    "inspect_tolerance",
    "inspect_max_angle",
    "report_low_score",
    "pitch_mm",
    "replace_output",
    "shots_overlap",
    "shots_margin",
    "square_min_size",
    "square_max_size",
    "square_max_elongation",
    "show_lens_grid",
    "inspect_package",
    "shots_part_width",
    "shots_part_height",
    "ring_min_diameter",
    "ring_max_diameter",
    "ring_min_ratio",
    "ring_max_ratio",
    "ring_min_fill",
    "ring_max_elongation",
    "ring_split",
    "ring_size_gap",
    "nozzle_x_mm",
    "nozzle_y_mm",
    "pick_max_offset_mm",
    "pick_max_turn_deg",
    "nominal_angle_deg",
    "pick_mirror",
    "quality_check",
    "quality_min_contrast",
    "quality_min_mean",
    "quality_max_mean",
    "quality_min_sharpness",
];

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// `auto` (the language of the system) or one of `en es de fr it ja zh pt`.
    pub language: String,
    pub polarity: Polarity,
    /// The threshold between dark and light, 0 to 255; `None` is `auto` (the threshold of Otsu).
    pub threshold: Option<u8>,
    pub min_diameter: f64,
    pub max_diameter: f64,
    pub min_fill: f64,
    pub max_elongation: f64,
    /// The size in millimetres of the mark that sets the scale.
    pub known_mm: f64,
    pub grid_columns: usize,
    pub grid_rows: usize,
    /// Whether the alignment may change the size of the board.
    pub allow_scale: bool,
    /// The largest residual of an alignment that is accepted, in millimetres.
    pub max_rms: f64,
    /// The largest change of size that is accepted (0.01 is one percent).
    pub max_stretch: f64,
    /// The largest picture or file of points that is read, in MiB.
    pub max_file_mib: u64,
    /// Whether the window draws, for each mark of the alignment, a line from the place the design puts it to the mark the camera found, with the distance in millimetres.
    pub show_residuals: bool,
    /// Whether `find` works the centre of each mark out again from the grey levels (to a fraction of a pixel) and prints a confidence.
    pub refine_subpixel: bool,
    /// The contrast (0 to 1) from which the confidence of a mark is the score of its shape alone; a mark that stands out less is trusted less.
    pub full_contrast: f64,
    /// The side in pixels of the window over which the light is averaged to take uneven light out of a picture before the marks are searched for; 0 leaves the picture as it is.
    pub flatten_window: usize,
    /// The millimetres of a pixel of the camera, as `calibrate` works it out; 0 means that it is not known (the command `measure` then needs `--mm-per-px`).
    pub mm_per_px: f64,
    /// The fewest and the most pixels a spot has to count as a part.
    pub part_min_area: u64,
    pub part_max_area: u64,
    /// The most parts that are measured in a picture; a picture with more is refused.
    pub max_parts: usize,
    /// The range of the side of the box of a cross-shaped mark, in pixels, and how much longer than wide that box may be.
    pub cross_min_size: f64,
    pub cross_max_size: f64,
    pub cross_max_elongation: f64,
    /// How much the distances between neighbouring marks of a target may differ (0.05 is five percent) for `calibrate` to give a scale from their pitch.
    pub pitch_tolerance: f64,
    /// The most pictures `batch` reads from a folder; a folder with more is refused.
    pub batch_max_files: usize,
    /// The least share of its box that a square mark fills (0 to 1); a disc fills about 0.785.
    pub square_min_fill: f64,
    /// The least score of a cross mark turned by any angle (0 to 1).
    pub cross_min_score: f64,
    /// The two radial factors of the lens, as `lens` works them out; 0 and 0 mean that the lens is not corrected.
    pub lens_k1: f64,
    pub lens_k2: f64,
    /// The most the marks of a grid may be from their lattice after the correction of the lens, in pixels; a fit above it is refused.
    pub lens_max_rms: f64,
    /// The file with the table of packages that `inspect` compares the parts with; empty means that `--table` has to be given.
    pub package_table: String,
    /// The most a part may differ from the body of its package in length or in width, in millimetres, unless the table gives another for the package.
    pub inspect_tolerance: f64,
    /// The most a part may be turned from the x axis, in degrees (0 is not checked, 90 at most).
    pub inspect_max_angle: f64,
    /// The score below which a picture of the HTML report of `batch` gets a warning (0 to 1).
    pub report_low_score: f64,
    /// The distance in millimetres between neighbouring marks of the target that sets the scale (`calibrate`); 0 means that it is not known.
    pub pitch_mm: f64,
    /// Whether the files that the window writes (the parts, the report of a folder) may be written over a file that is there; the command line does so with `--replace`.
    pub replace_output: bool,
    /// The share of the field (0 to 0.9) by which neighbouring pictures of the plan of `shots` overlap, and the margin around the part that the pictures must also cover, in millimetres.
    pub shots_overlap: f64,
    pub shots_margin: f64,
    /// The range of the side of a square mark, in pixels, and how much longer than wide its box may be (the crosses have their own, `cross_min_size` and the rest).
    pub square_min_size: f64,
    pub square_max_size: f64,
    pub square_max_elongation: f64,
    /// Whether the window draws, once the lens is fitted from a grid, the places where the fitted lens puts the lattice of the grid, to be compared with the marks that were found.
    pub show_lens_grid: bool,
    /// The name of the package of the table that `inspect` and the window judge the parts against when `--package` is not given; empty means that it has to be given.
    pub inspect_package: String,
    /// The size of the part in millimetres that `shots` and the window plan the pictures for; 0 and 0 mean that the body of the package is used.
    pub shots_part_width: f64,
    pub shots_part_height: f64,
    /// The range of the outer diameter of a ring mark in pixels (8 up), the range of its inner diameter over the outer one (above 0, below 1), the least share of its band that it fills, and how
    /// much longer than wide it may be.
    pub ring_min_diameter: f64,
    pub ring_max_diameter: f64,
    pub ring_min_ratio: f64,
    pub ring_max_ratio: f64,
    pub ring_min_fill: f64,
    pub ring_max_elongation: f64,
    /// The outer diameter in pixels that tells small rings from large ones (0 finds it from the diameters), and how much larger the large ones must be than the small ones for the two sizes to be told apart then.
    pub ring_split: f64,
    pub ring_size_gap: f64,
    /// Where the axis of the nozzle is in the picture of a picked part, in millimetres from the centre of the picture (x to the right, y up), the farthest from it that the part may be held
    /// (0 does not check it) and the most that it may be turned from the way it should be held, in degrees (0 does not check it, 90 at most): what `offset` works with.
    pub nozzle_x_mm: f64,
    pub nozzle_y_mm: f64,
    pub pick_max_offset_mm: f64,
    pub pick_max_turn_deg: f64,
    /// The turn of the long side of a picked part when it is held as it should be, in degrees counter-clockwise from the x axis (`offset` and the window turn the part from it).
    pub nominal_angle_deg: f64,
    /// Whether the camera sees the picked part turned over left to right (it looks up from under the nozzle, or there is a mirror): the x and the turn of each part change sign.
    pub pick_mirror: bool,
    /// Whether a picture is refused before a part is measured from it when it is flat, too dark, too bright or (when `quality_min_sharpness` is above 0) blurred, and the values of that check.
    pub quality_check: bool,
    pub quality_min_contrast: f64,
    pub quality_min_mean: f64,
    pub quality_max_mean: f64,
    pub quality_min_sharpness: f64,
}

impl Default for Config {
    fn default() -> Self {
        let s = Settings::default();
        Config {
            language: "auto".into(),
            polarity: s.polarity,
            threshold: None,
            min_diameter: s.min_diameter,
            max_diameter: s.max_diameter,
            min_fill: s.min_fill,
            max_elongation: s.max_elongation,
            known_mm: 1.0,
            grid_columns: 5,
            grid_rows: 4,
            allow_scale: false,
            max_rms: 0.1,
            max_stretch: 0.01,
            max_file_mib: 64,
            show_residuals: true,
            refine_subpixel: false,
            full_contrast: 0.3,
            flatten_window: 0,
            mm_per_px: 0.0,
            part_min_area: 100,
            part_max_area: 50_000_000,
            max_parts: 1000,
            cross_min_size: 12.0,
            cross_max_size: 200.0,
            cross_max_elongation: 1.2,
            pitch_tolerance: 0.05,
            batch_max_files: 500,
            square_min_fill: 0.88,
            cross_min_score: 0.85,
            lens_k1: 0.0,
            lens_k2: 0.0,
            lens_max_rms: 0.5,
            package_table: String::new(),
            inspect_tolerance: 0.2,
            inspect_max_angle: 0.0,
            report_low_score: 0.6,
            pitch_mm: 0.0,
            replace_output: false,
            shots_overlap: 0.1,
            shots_margin: 0.5,
            square_min_size: 12.0,
            square_max_size: 200.0,
            square_max_elongation: 1.2,
            show_lens_grid: true,
            inspect_package: String::new(),
            shots_part_width: 0.0,
            shots_part_height: 0.0,
            ring_min_diameter: 12.0,
            ring_max_diameter: 200.0,
            ring_min_ratio: 0.25,
            ring_max_ratio: 0.85,
            ring_min_fill: 0.85,
            ring_max_elongation: 1.25,
            ring_split: 0.0,
            ring_size_gap: 1.25,
            nozzle_x_mm: 0.0,
            nozzle_y_mm: 0.0,
            pick_max_offset_mm: 0.0,
            pick_max_turn_deg: 0.0,
            nominal_angle_deg: 0.0,
            pick_mirror: false,
            quality_check: false,
            quality_min_contrast: 10.0,
            quality_min_mean: 20.0,
            quality_max_mean: 235.0,
            quality_min_sharpness: 0.0,
        }
    }
}

fn number(text: &str, key: &str) -> Result<f64, String> {
    text.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{key} '{text}' is not a number"))
}

/// `yes`, `true` or `on`, or `no`, `false` or `off`.
fn yes_no(text: &str, key: &str) -> Result<bool, String> {
    match text {
        "yes" | "true" | "on" => Ok(true),
        "no" | "false" | "off" => Ok(false),
        _ => Err(format!("{key} '{text}' is not yes or no")),
    }
}

fn count(text: &str, key: &str) -> Result<usize, String> {
    text.parse::<usize>()
        .map_err(|_| format!("{key} '{text}' is not a whole number"))
}

impl Config {
    /// The settings of the search for marks that these values make.
    pub fn settings(&self) -> Settings {
        Settings {
            polarity: self.polarity,
            min_diameter: self.min_diameter,
            max_diameter: self.max_diameter,
            min_fill: self.min_fill,
            max_elongation: self.max_elongation,
        }
    }

    /// The settings of the offset of a picked part that the values make.
    pub fn offset_settings(&self) -> OffsetSettings {
        OffsetSettings {
            nozzle_x_mm: self.nozzle_x_mm,
            nozzle_y_mm: self.nozzle_y_mm,
            nominal_angle_deg: self.nominal_angle_deg,
            tolerance_mm: self.inspect_tolerance,
            max_offset_mm: self.pick_max_offset_mm,
            max_turn_deg: self.pick_max_turn_deg,
        }
    }

    /// The settings of the check of the picture that the values make.
    pub fn quality_settings(&self) -> QualitySettings {
        QualitySettings {
            min_contrast: self.quality_min_contrast,
            min_mean: self.quality_min_mean,
            max_mean: self.quality_max_mean,
            min_sharpness: self.quality_min_sharpness,
        }
    }

    /// The settings of the search for ring marks that the values make.
    pub fn ring_settings(&self) -> RingSettings {
        RingSettings {
            polarity: self.polarity,
            threshold: self.threshold,
            min_diameter: self.ring_min_diameter,
            max_diameter: self.ring_max_diameter,
            min_ratio: self.ring_min_ratio,
            max_ratio: self.ring_max_ratio,
            min_fill: self.ring_min_fill,
            max_elongation: self.ring_max_elongation,
            max_spots: self.max_parts,
            full_contrast: self.full_contrast,
            split: self.ring_split,
            size_gap: self.ring_size_gap,
        }
    }

    /// Sets one value from its text. The rest of the file is not looked at: see `validate`.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let value = value.trim();
        match key {
            "language" => {
                let ok = value == "auto"
                    || ["en", "es", "de", "fr", "it", "ja", "zh", "pt"].contains(&value);
                if !ok {
                    return Err(format!(
                        "language '{value}' is not auto, en, es, de, fr, it, ja, zh or pt"
                    ));
                }
                self.language = value.to_string();
            }
            "polarity" => {
                self.polarity = match value {
                    "dark" => Polarity::Dark,
                    "light" => Polarity::Light,
                    _ => return Err(format!("polarity '{value}' is not dark or light")),
                }
            }
            "threshold" => {
                self.threshold = if value == "auto" {
                    None
                } else {
                    Some(
                        value
                            .parse::<u8>()
                            .map_err(|_| format!("threshold '{value}' is not auto or 0 to 255"))?,
                    )
                }
            }
            "min_diameter" => self.min_diameter = number(value, key)?,
            "max_diameter" => self.max_diameter = number(value, key)?,
            "min_fill" => self.min_fill = number(value, key)?,
            "max_elongation" => self.max_elongation = number(value, key)?,
            "known_mm" => self.known_mm = number(value, key)?,
            "grid_columns" => self.grid_columns = count(value, key)?,
            "grid_rows" => self.grid_rows = count(value, key)?,
            "allow_scale" => {
                self.allow_scale = match value {
                    "yes" | "true" | "on" => true,
                    "no" | "false" | "off" => false,
                    _ => return Err(format!("allow_scale '{value}' is not yes or no")),
                }
            }
            "max_rms" => self.max_rms = number(value, key)?,
            "max_stretch" => self.max_stretch = number(value, key)?,
            "max_file_mib" => {
                self.max_file_mib = value
                    .parse::<u64>()
                    .map_err(|_| format!("max_file_mib '{value}' is not a whole number"))?
            }
            "show_residuals" => {
                self.show_residuals = match value {
                    "yes" | "true" | "on" => true,
                    "no" | "false" | "off" => false,
                    _ => return Err(format!("show_residuals '{value}' is not yes or no")),
                }
            }
            "show_lens_grid" => {
                self.show_lens_grid = match value {
                    "yes" | "true" | "on" => true,
                    "no" | "false" | "off" => false,
                    _ => return Err(format!("show_lens_grid '{value}' is not yes or no")),
                }
            }
            "refine_subpixel" => {
                self.refine_subpixel = match value {
                    "yes" | "true" | "on" => true,
                    "no" | "false" | "off" => false,
                    _ => return Err(format!("refine_subpixel '{value}' is not yes or no")),
                }
            }
            "full_contrast" => self.full_contrast = number(value, key)?,
            "flatten_window" => self.flatten_window = count(value, key)?,
            "mm_per_px" => self.mm_per_px = number(value, key)?,
            "part_min_area" => self.part_min_area = count(value, key)? as u64,
            "part_max_area" => self.part_max_area = count(value, key)? as u64,
            "max_parts" => self.max_parts = count(value, key)?,
            "cross_min_size" => self.cross_min_size = number(value, key)?,
            "cross_max_size" => self.cross_max_size = number(value, key)?,
            "cross_max_elongation" => self.cross_max_elongation = number(value, key)?,
            "square_min_size" => self.square_min_size = number(value, key)?,
            "square_max_size" => self.square_max_size = number(value, key)?,
            "square_max_elongation" => self.square_max_elongation = number(value, key)?,
            "pitch_tolerance" => self.pitch_tolerance = number(value, key)?,
            "batch_max_files" => self.batch_max_files = count(value, key)?,
            "square_min_fill" => self.square_min_fill = number(value, key)?,
            "cross_min_score" => self.cross_min_score = number(value, key)?,
            "lens_k1" => self.lens_k1 = number(value, key)?,
            "lens_k2" => self.lens_k2 = number(value, key)?,
            "lens_max_rms" => self.lens_max_rms = number(value, key)?,
            "package_table" => self.package_table = value.to_string(),
            "inspect_package" => self.inspect_package = value.to_string(),
            "inspect_tolerance" => self.inspect_tolerance = number(value, key)?,
            "inspect_max_angle" => self.inspect_max_angle = number(value, key)?,
            "report_low_score" => self.report_low_score = number(value, key)?,
            "pitch_mm" => self.pitch_mm = number(value, key)?,
            "shots_overlap" => self.shots_overlap = number(value, key)?,
            "shots_margin" => self.shots_margin = number(value, key)?,
            "shots_part_width" => self.shots_part_width = number(value, key)?,
            "shots_part_height" => self.shots_part_height = number(value, key)?,
            "ring_min_diameter" => self.ring_min_diameter = number(value, key)?,
            "ring_max_diameter" => self.ring_max_diameter = number(value, key)?,
            "ring_min_ratio" => self.ring_min_ratio = number(value, key)?,
            "ring_max_ratio" => self.ring_max_ratio = number(value, key)?,
            "ring_min_fill" => self.ring_min_fill = number(value, key)?,
            "ring_max_elongation" => self.ring_max_elongation = number(value, key)?,
            "ring_split" => self.ring_split = number(value, key)?,
            "ring_size_gap" => self.ring_size_gap = number(value, key)?,
            "nozzle_x_mm" => self.nozzle_x_mm = number(value, key)?,
            "nozzle_y_mm" => self.nozzle_y_mm = number(value, key)?,
            "pick_max_offset_mm" => self.pick_max_offset_mm = number(value, key)?,
            "pick_max_turn_deg" => self.pick_max_turn_deg = number(value, key)?,
            "nominal_angle_deg" => self.nominal_angle_deg = number(value, key)?,
            "pick_mirror" => self.pick_mirror = yes_no(value, key)?,
            "quality_check" => self.quality_check = yes_no(value, key)?,
            "quality_min_contrast" => self.quality_min_contrast = number(value, key)?,
            "quality_min_mean" => self.quality_min_mean = number(value, key)?,
            "quality_max_mean" => self.quality_max_mean = number(value, key)?,
            "quality_min_sharpness" => self.quality_min_sharpness = number(value, key)?,
            "replace_output" => {
                self.replace_output = match value {
                    "yes" | "true" | "on" => true,
                    "no" | "false" | "off" => false,
                    _ => return Err(format!("replace_output '{value}' is not yes or no")),
                }
            }
            other => return Err(format!("{other}: unknown key")),
        }
        Ok(())
    }

    /// Whether the values make sense together.
    pub fn validate(&self) -> Result<(), String> {
        if !(self.min_diameter > 1.0 && self.max_diameter >= self.min_diameter) {
            return Err(
                "the diameters must be from 2 up and the largest not below the smallest".into(),
            );
        }
        if !(self.min_fill > 0.0 && self.min_fill <= 1.0) {
            return Err("min_fill must be more than 0 and at most 1".into());
        }
        if self.max_elongation < 1.0 {
            return Err("max_elongation must be 1 or more".into());
        }
        if self.known_mm <= 0.0 {
            return Err("known_mm must be more than 0".into());
        }
        if !(2..=50).contains(&self.grid_columns) || !(2..=50).contains(&self.grid_rows) {
            return Err("the grid needs from 2 to 50 columns and from 2 to 50 rows".into());
        }
        if self.max_rms < 0.0 || self.max_stretch < 0.0 {
            return Err("max_rms and max_stretch must be 0 or more".into());
        }
        if !(1..=1024).contains(&self.max_file_mib) {
            return Err("max_file_mib must be from 1 to 1024".into());
        }
        if !(self.full_contrast > 0.0 && self.full_contrast <= 1.0) {
            return Err("full_contrast must be more than 0 and at most 1".into());
        }
        if self.flatten_window != 0
            && !(crate::flatten::MIN_WINDOW..=crate::flatten::MAX_WINDOW)
                .contains(&self.flatten_window)
        {
            return Err(format!(
                "flatten_window must be 0 (off) or from {} to {}",
                crate::flatten::MIN_WINDOW,
                crate::flatten::MAX_WINDOW
            ));
        }
        if self.mm_per_px < 0.0 {
            return Err("mm_per_px must be 0 (not known) or more".into());
        }
        if self.part_min_area == 0 || self.part_max_area < self.part_min_area {
            return Err("part_min_area must be 1 or more and part_max_area not below it".into());
        }
        if !(1..=100_000).contains(&self.max_parts) {
            return Err("max_parts must be from 1 to 100000".into());
        }
        if !(self.cross_min_size >= 7.0 && self.cross_max_size >= self.cross_min_size) {
            return Err(
                "the side of a cross must be from 7 pixels up and the largest not below the smallest"
                    .into(),
            );
        }
        if !(self.square_min_size >= 7.0 && self.square_max_size >= self.square_min_size) {
            return Err(
                "the side of a square must be from 7 pixels up and the largest not below the smallest"
                    .into(),
            );
        }
        if self.square_max_elongation < 1.0 {
            return Err("square_max_elongation must be 1 or more".into());
        }
        if self.cross_max_elongation < 1.0 {
            return Err("cross_max_elongation must be 1 or more".into());
        }
        if !(0.0..=1.0).contains(&self.pitch_tolerance) {
            return Err("pitch_tolerance must be from 0 to 1".into());
        }
        if !(1..=100_000).contains(&self.batch_max_files) {
            return Err("batch_max_files must be from 1 to 100000".into());
        }
        if !(self.square_min_fill > 0.0 && self.square_min_fill <= 1.0) {
            return Err("square_min_fill must be more than 0 and at most 1".into());
        }
        if !(self.cross_min_score > 0.0 && self.cross_min_score <= 1.0) {
            return Err("cross_min_score must be more than 0 and at most 1".into());
        }
        let limit = crate::lens::K_LIMIT;
        if self.lens_k1.abs() > limit || self.lens_k2.abs() > limit {
            return Err(format!(
                "lens_k1 and lens_k2 must be between -{limit} and {limit}"
            ));
        }
        if self.lens_max_rms <= 0.0 {
            return Err("lens_max_rms must be more than 0".into());
        }
        if self.package_table.len() > 1024 || self.package_table.contains('\0') {
            return Err("package_table must be a file name of at most 1024 characters".into());
        }
        if self.inspect_package.len() > 256 || self.inspect_package.contains(' ') {
            return Err("inspect_package must be a name of at most 256 characters".into());
        }
        if self.inspect_tolerance <= 0.0 {
            return Err("inspect_tolerance must be more than 0".into());
        }
        if !(0.0..=90.0).contains(&self.inspect_max_angle) {
            return Err("inspect_max_angle must be from 0 (not checked) to 90".into());
        }
        if !(0.0..=1.0).contains(&self.report_low_score) {
            return Err("report_low_score must be from 0 to 1".into());
        }
        if self.pitch_mm < 0.0 {
            return Err("pitch_mm must be 0 (not known) or more".into());
        }
        if !(0.0..=crate::shots::MAX_OVERLAP).contains(&self.shots_overlap) {
            return Err(format!(
                "shots_overlap must be from 0 to {}",
                crate::shots::MAX_OVERLAP
            ));
        }
        if !(self.ring_min_diameter >= 8.0 && self.ring_max_diameter >= self.ring_min_diameter) {
            return Err(
                "the outer diameter of a ring must be from 8 pixels up and the largest not below the smallest"
                    .into(),
            );
        }
        if !(self.ring_min_ratio > 0.0
            && self.ring_max_ratio < 1.0
            && self.ring_max_ratio >= self.ring_min_ratio)
        {
            return Err("the ratio of the inner to the outer diameter of a ring must be above 0 and below 1, the largest not below the smallest".into());
        }
        if !(self.ring_min_fill > 0.0 && self.ring_min_fill <= 1.0) {
            return Err("ring_min_fill must be more than 0 and at most 1".into());
        }
        if self.ring_max_elongation < 1.0 {
            return Err("ring_max_elongation must be 1 or more".into());
        }
        if self.ring_split < 0.0 {
            return Err("ring_split must be 0 (found from the diameters) or more".into());
        }
        if self.pick_max_offset_mm < 0.0 {
            return Err("pick_max_offset_mm must be 0 (not checked) or more".into());
        }
        if !(0.0..=90.0).contains(&self.pick_max_turn_deg) {
            return Err("pick_max_turn_deg must be from 0 (not checked) to 90".into());
        }
        crate::quality::check(&self.quality_settings())?;
        if self.ring_size_gap <= 1.0 {
            return Err("ring_size_gap must be above 1".into());
        }
        if self.shots_part_width < 0.0 || self.shots_part_height < 0.0 {
            return Err(
                "shots_part_width and shots_part_height must be 0 (the package is used) or more"
                    .into(),
            );
        }
        if self.shots_margin < 0.0 {
            return Err("shots_margin must be 0 or more".into());
        }
        Ok(())
    }

    /// The values of a text: lines of `key = value`, blank lines and lines that start with `#` left out. A key that is written twice, an unknown key or a bad value is an error with the line.
    pub fn parse(text: &str) -> Result<Config, String> {
        let mut config = Config::default();
        let mut seen: Vec<&str> = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("line {}: it must be key = value", i + 1))?;
            let key = key.trim();
            if seen.contains(&key) {
                return Err(format!("line {}: {key} is written twice", i + 1));
            }
            config
                .set(key, value)
                .map_err(|e| format!("line {}: {e}", i + 1))?;
            seen.push(key);
        }
        config.validate()?;
        Ok(config)
    }

    /// The values of the file at `path`.
    pub fn read(path: &Path) -> Result<Config, String> {
        let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if meta.len() > 65_536 {
            return Err(format!(
                "{}: a configuration file is not larger than 64 KiB",
                path.display()
            ));
        }
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Config::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The file to read: the one the variable names, else `vision.cfg` in the current folder, else the one beside the program. `None` when there is none.
    pub fn find_file() -> Option<PathBuf> {
        if let Some(v) = std::env::var_os(FILE_VARIABLE).filter(|v| !v.is_empty()) {
            return Some(PathBuf::from(v));
        }
        let beside = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(FILE_NAME)));
        [Some(PathBuf::from(FILE_NAME)), beside]
            .into_iter()
            .flatten()
            .find(|p| p.is_file())
    }

    /// The configuration to start with, and what is wrong with the file when it cannot be used (the defaults are then the values).
    pub fn load() -> (Config, Option<String>) {
        match Config::find_file() {
            None => (Config::default(), None),
            Some(path) => match Config::read(&path) {
                Ok(c) => (c, None),
                Err(e) => (Config::default(), Some(e)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid() {
        assert!(Config::default().validate().is_ok());
        assert_eq!(Config::parse("").unwrap(), Config::default());
        assert_eq!(Config::parse("# nothing\n\n").unwrap(), Config::default());
    }

    #[test]
    fn values_are_read_and_the_rest_keeps_the_default() {
        let c = Config::parse(
            "language = pt\npolarity = light\nthreshold = 120\nmin_diameter = 12 \ngrid_columns = 7\nallow_scale = yes\n",
        )
        .unwrap();
        assert_eq!(c.language, "pt");
        assert_eq!(c.polarity, Polarity::Light);
        assert_eq!(c.threshold, Some(120));
        assert_eq!(c.min_diameter, 12.0);
        assert_eq!(c.grid_columns, 7);
        assert!(c.allow_scale);
        assert_eq!(c.max_rms, Config::default().max_rms);
        assert_eq!(Config::parse("threshold = auto").unwrap().threshold, None);
    }

    #[test]
    fn a_mistake_is_refused_with_its_line() {
        let cases = [
            ("polarity = grey", "line 1: polarity 'grey'"),
            (
                "known_mm = 2\nmin_fill = x",
                "line 2: min_fill 'x' is not a number",
            ),
            ("colour = red", "line 1: colour: unknown key"),
            ("just a line", "line 1: it must be key = value"),
            ("threshold = 300", "line 1: threshold '300'"),
            (
                "max_rms = 1\nmax_rms = 2",
                "line 2: max_rms is written twice",
            ),
            ("language = xx", "line 1: language 'xx'"),
            (
                "show_lens_grid = maybe",
                "line 1: show_lens_grid 'maybe' is not yes or no",
            ),
            (
                "show_residuals = maybe",
                "line 1: show_residuals 'maybe' is not yes or no",
            ),
            (
                "grid_columns = 2.5",
                "line 1: grid_columns '2.5' is not a whole number",
            ),
        ];
        for (text, expected) in cases {
            let e = Config::parse(text).unwrap_err();
            assert!(e.contains(expected), "{text:?} gave {e:?}");
        }
    }

    #[test]
    fn values_that_make_no_sense_together_are_refused() {
        for text in [
            "min_diameter = 30\nmax_diameter = 20",
            "min_diameter = 1",
            "min_fill = 0",
            "min_fill = 1.5",
            "max_elongation = 0.5",
            "known_mm = 0",
            "grid_rows = 1",
            "grid_columns = 51",
            "max_rms = -1",
            "max_file_mib = 0",
            "full_contrast = 0",
            "full_contrast = 1.5",
            "flatten_window = 2",
            "flatten_window = 5000",
            "mm_per_px = -0.1",
            "part_min_area = 0",
            "part_min_area = 500
part_max_area = 100",
            "max_parts = 0",
            "max_parts = 100001",
            "cross_min_size = 5",
            "cross_min_size = 50
cross_max_size = 20",
            "cross_max_elongation = 0.9",
            "square_min_size = 5",
            "square_min_size = 50
square_max_size = 20",
            "square_max_elongation = 0.9",
            "pitch_tolerance = -0.1",
            "pitch_tolerance = 2",
            "batch_max_files = 0",
            "batch_max_files = 100001",
            "square_min_fill = 0",
            "square_min_fill = 1.2",
            "cross_min_score = 0",
            "cross_min_score = 2",
            "lens_k1 = 0.6",
            "lens_k2 = -0.6",
            "lens_max_rms = 0",
            "lens_max_rms = -1",
            "inspect_tolerance = 0",
            "inspect_max_angle = 91",
            "inspect_max_angle = -1",
            "report_low_score = 1.5",
            "report_low_score = -0.1",
            "pitch_mm = -1",
            "replace_output = maybe",
            "shots_overlap = 0.95",
            "shots_overlap = -0.1",
            "shots_margin = -1",
            "shots_part_width = -1",
            "ring_min_diameter = 5",
            "ring_min_diameter = 50\nring_max_diameter = 20",
            "ring_min_ratio = 0",
            "ring_max_ratio = 1",
            "ring_min_ratio = 0.8\nring_max_ratio = 0.5",
            "ring_min_fill = 0",
            "ring_min_fill = 1.5",
            "ring_max_elongation = 0.9",
            "ring_split = -1",
            "ring_size_gap = 1",
            "pick_max_offset_mm = -1",
            "pick_max_turn_deg = 91",
            "pick_max_turn_deg = -1",
            "nominal_angle_deg = north",
            "pick_mirror = maybe",
            "quality_check = 1",
            "quality_min_contrast = -1",
            "quality_min_sharpness = -1",
            "quality_min_mean = 200
quality_max_mean = 100",
            "quality_max_mean = 300",
            "shots_part_height = -0.5",
            "inspect_package = a b",
        ] {
            assert!(Config::parse(text).is_err(), "{text:?} was accepted");
        }
    }

    #[test]
    fn the_lines_of_the_alignment_can_be_switched_off() {
        assert!(Config::default().show_residuals);
        assert!(!Config::parse("show_residuals = no").unwrap().show_residuals);
        assert!(Config::parse("show_residuals = on").unwrap().show_residuals);
    }

    #[test]
    fn a_number_that_is_not_finite_is_refused() {
        assert!(Config::parse("known_mm = nan").is_err());
        assert!(Config::parse("max_diameter = inf").is_err());
    }

    #[test]
    fn the_settings_of_the_search_come_from_the_values() {
        let c = Config::parse("polarity = light\nmin_fill = 0.7").unwrap();
        let s = c.settings();
        assert_eq!(s.polarity, Polarity::Light);
        assert_eq!(s.min_fill, 0.7);
        assert_eq!(s.max_elongation, Config::default().max_elongation);
    }

    #[test]
    fn a_file_that_is_not_there_is_an_error_with_its_name() {
        let e = Config::read(Path::new("/this/file/does/not/exist.cfg")).unwrap_err();
        assert!(e.contains("exist.cfg"));
    }

    #[test]
    fn the_sample_file_lists_every_key_with_the_default_value() {
        let text = include_str!("../examples/vision.cfg");
        let c = Config::parse(text).unwrap();
        assert_eq!(c, Config::default());
        for key in KEYS {
            let written = text
                .lines()
                .any(|l| l.trim_start().starts_with(&format!("{key} =")));
            assert!(written, "{key} is not in examples/vision.cfg");
        }
    }
}
