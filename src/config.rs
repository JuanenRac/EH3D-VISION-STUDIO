//! The configuration file: every value that is tuned (what a mark looks like, the grid of the target, how much a fit may be off) is written in a plain `key = value` file, not in the code.
//! The file is `vision.cfg`; the variable `EH3D_VISION_CONFIG` names another one. The values that are written here are the ones that the program starts with, and the file that comes with
//! the program (`examples/vision.cfg`) holds all of them with their meaning. A file that is not there means the defaults; a file with a mistake is refused with the line and the reason.

use crate::fiducial::{Polarity, Settings};
use std::path::{Path, PathBuf};

/// The name of the file that is looked for when nothing else is said.
pub const FILE_NAME: &str = "vision.cfg";
/// The variable of the system that names another file.
pub const FILE_VARIABLE: &str = "EH3D_VISION_CONFIG";

/// Every key of the file, in the order the file lists them.
pub const KEYS: [&str; 15] = [
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
        }
    }
}

fn number(text: &str, key: &str) -> Result<f64, String> {
    text.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{key} '{text}' is not a number"))
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
