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
pub const KEYS: [&str; 168] = [
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
    "focus_reference",
    "edge_step_px",
    "edge_max_uncertainty_px",
    "server_address",
    "server_token",
    "server_machine",
    "server_timeout_s",
    "server_retries",
    "server_mdns",
    "correlate_diameter",
    "correlate_min_score",
    "train_min_side",
    "train_max_side",
    "train_min_contrast",
    "train_agree_score",
    "train_max_ambiguity",
    "train_min_score",
    "train_turn_step_deg",
    "train_turn_max_deg",
    "pair_tolerance_mm",
    "pair_min_marks",
    "pair_max_turn_deg",
    "find_server_max",
    "find_server_private_only",
    "find_server_tries",
    "mjpeg_max_bytes",
    "mjpeg_max_pixels",
    "card_columns",
    "card_rows",
    "card_pitch_mm",
    "card_dot_mm",
    "card_margin_mm",
    "card_paper_w_mm",
    "card_paper_h_mm",
    "light_cells",
    "light_saturation_level",
    "light_max_saturated",
    "light_min_mean",
    "light_max_mean",
    "light_max_uneven",
    "light_max_tilt",
    "light_max_pixels",
    "tape_hole_mm",
    "tape_pitch_mm",
    "tape_tolerance_mm",
    "tape_line_tolerance_mm",
    "tape_size_tolerance",
    "tape_min_holes",
    "tape_max_holes",
    "tape_pocket_pitch_mm",
    "tape_pocket_along_mm",
    "tape_pocket_across_mm",
    "tape_max_pixels",
    "rectify_max_side",
    "pads_tolerance",
    "diff_threshold",
    "diff_min_area",
    "diff_max_changed",
    "orientation_body_contrast",
    "orientation_mark_contrast",
    "orientation_min_mark_px",
    "orientation_tolerance_deg",
    "footprint_tolerance_mm",
    "sharpmap_cells",
    "sharpmap_min_share",
    "sharpmap_min_contrast",
    "board_square_mm",
    "board_max_aspect",
    "board_max_spread",
    "capture_backend",
    "capture_camera",
    "capture_format",
    "capture_profile",
    "capture_log_level",
    "capture_usb_mbps",
    "capture_mjpeg_bits_px",
    "capture_black_mean",
    "capture_frozen_frames",
    "capture_flicker_max",
    "capture_powerline_hz",
    "capture_settle_frames",
    "capture_grab_tries",
    "calib_squares_x",
    "calib_squares_y",
    "calib_square_mm",
    "calib_marker_seed",
    "calib_min_corners",
    "calib_max_residual_px",
    "calib_max_method_gap",
    "calib_check_mm",
    "calib_check_tol_mm",
    "calib_allow_unchecked",
    "camera_offset_x_mm",
    "camera_offset_y_mm",
    "camera_offset_deg",
    "align_tolerance_mm",
    "align_tolerance_deg",
    "align_max_step_mm",
    "align_max_step_deg",
    "align_max_total_mm",
    "align_max_steps",
    "align_gain",
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
    /// `server send focus`: the sharpness (variance of the Laplacian) that counts as 50 points of 100 in the result of a focus check; a picture is sharp from there on.
    pub focus_reference: f64,
    /// `edge`: the distance in pixels between two samples along the line that is looked across, and the largest uncertainty of the position of the edge (one standard deviation, in pixels)
    /// that the verdict accepts.
    pub edge_step_px: f64,
    pub edge_max_uncertainty_px: f64,
    /// The server of the machine: `host:port` (empty means that there is none), the token of this program, the id of the machine, how long one try may take in seconds and how many more
    /// tries follow a failure on the network.
    pub server_address: String,
    pub server_token: String,
    pub server_machine: String,
    pub server_timeout_s: u64,
    pub server_retries: u32,
    /// Whether the server is looked for on the local network (off by default; the program asks nothing of the network unless this is on and a resolver is linked in).
    pub server_mdns: bool,
    /// `correlate`: the diameter in pixels of the disc that is looked for (the search by correlation needs it) and the least correlation that counts as a mark.
    pub correlate_diameter: f64,
    pub correlate_min_score: f64,
    /// `train`: the sides of the examples that train a mark in pixels, the least contrast of an example, how well several examples must agree, the most that a mark may look like itself
    /// moved a few pixels, the least score of a search with the trained mark, the step of the turn in degrees and how far the search turns it (0: not at all, 180: every angle).
    pub train_min_side: usize,
    pub train_max_side: usize,
    pub train_min_contrast: f64,
    pub train_agree_score: f64,
    pub train_max_ambiguity: f64,
    pub train_min_score: f64,
    pub train_turn_step_deg: f64,
    pub train_turn_max_deg: f64,
    /// `pair`: how far a mark may lie from where a pairing puts it (millimetres), the least marks that must be paired and the most that the board may be turned (degrees) when the marks are paired with the design whatever the turn.
    pub pair_tolerance_mm: f64,
    pub pair_min_marks: usize,
    pub pair_max_turn_deg: f64,
    /// `find_server`: the most candidates of the search on the local network, whether only the addresses of the local network are kept, and how many times it asks when nothing useful answers.
    pub find_server_max: usize,
    pub find_server_private_only: bool,
    pub find_server_tries: usize,
    /// `mjpeg`: the most bytes and pixels of a frame of motion JPEG that is read.
    /// `card`: the card of calibration to print, in millimetres: the dots along each side, the pitch, the dot, the margin and the page.
    /// `light`: the check of the light of a picture: the zones along a side, the grey level of a burnt-out pixel, the most burnt-out share, the range of the mean, the most unevenness and tilt, the most pixels.
    /// `tape`: the search for the holes of a tape: the hole and the pitch, the tolerances, the holes it needs and accepts, and the pockets (pitch, along and across from a hole).
    pub mjpeg_max_bytes: usize,
    pub mjpeg_max_pixels: usize,
    pub card_columns: usize,
    pub card_rows: usize,
    pub card_pitch_mm: f64,
    pub card_dot_mm: f64,
    pub card_margin_mm: f64,
    pub card_paper_w_mm: f64,
    pub card_paper_h_mm: f64,
    pub light_cells: usize,
    pub light_saturation_level: usize,
    pub light_max_saturated: f64,
    pub light_min_mean: f64,
    pub light_max_mean: f64,
    pub light_max_uneven: f64,
    pub light_max_tilt: f64,
    pub light_max_pixels: usize,
    pub tape_hole_mm: f64,
    pub tape_pitch_mm: f64,
    pub tape_tolerance_mm: f64,
    pub tape_line_tolerance_mm: f64,
    pub tape_size_tolerance: f64,
    pub tape_min_holes: usize,
    pub tape_max_holes: usize,
    pub tape_pocket_pitch_mm: f64,
    pub tape_pocket_along_mm: f64,
    pub tape_pocket_across_mm: f64,
    pub tape_max_pixels: usize,
    /// `rectify`: the largest side in pixels of the picture that is made.
    pub rectify_max_side: usize,
    /// `pads`: how much the pitch of a row of pads may vary as a share of the median pitch.
    pub pads_tolerance: f64,
    /// `diff`: how much a grey level has to differ for a pixel to count as changed, the fewest pixels of a region of changes and the largest share of changed pixels for two pictures to count as the same.
    pub diff_threshold: u8,
    pub diff_min_area: u64,
    pub diff_max_changed: f64,
    /// `orientation`: how much a pixel must differ from the board to be the part and from the body to be the mark (grey levels), the fewest pixels of a mark and how far the mark may point from
    /// the wanted direction, in degrees.
    pub orientation_body_contrast: f64,
    pub orientation_mark_contrast: f64,
    pub orientation_min_mark_px: u64,
    pub orientation_tolerance_deg: f64,
    /// `footprint`: how far the sides of a pad may be from the design, in millimetres.
    pub footprint_tolerance_mm: f64,
    /// `sharpmap`: the cells on each side of the map, the share of the median sharpness below which a cell is weak, and the contrast below which a cell is empty.
    pub sharpmap_cells: usize,
    pub sharpmap_min_share: f64,
    pub sharpmap_min_contrast: f64,
    /// `board`: the side of a square of the board in millimetres (0: not known; the pitch is `pitch_mm`), and how far the scales along x and y and the distances between neighbours may differ.
    pub board_square_mm: f64,
    pub board_max_aspect: f64,
    pub board_max_spread: f64,
    /// `capture`: the backend of the live camera (`auto`, `sim`, `v4l2`, `mfoundation`, `libcamera`), the camera by its identifier (empty: the first), the format as `640x480@30:yuyv` (empty: chosen),
    /// the settings of the camera as `exposure=120 gain=10 focus=auto`, and the level of the log.
    pub capture_backend: String,
    pub capture_camera: String,
    pub capture_format: String,
    pub capture_profile: String,
    pub capture_log_level: String,
    /// `capture`: what a USB bus carries (megabits a second) and the bits for each pixel of a motion JPEG stream.
    pub capture_usb_mbps: f64,
    pub capture_mjpeg_bits_px: f64,
    /// `capture`: the mean brightness below which a frame is black, the frames in a row that are the same for a stream to be frozen, the depth of the flicker that is allowed and the frequency of the mains.
    pub capture_black_mean: f64,
    pub capture_frozen_frames: usize,
    pub capture_flicker_max: f64,
    pub capture_powerline_hz: u32,
    /// `capture`: the frames thrown away after a change and the most frames asked while waiting.
    pub capture_settle_frames: usize,
    pub capture_grab_tries: usize,
    /// `calib`: the board of calibration (squares each way, the side of a square in millimetres, the seed of the markers) and what is asked of the calibration.
    pub calib_squares_x: usize,
    pub calib_squares_y: usize,
    pub calib_square_mm: f64,
    pub calib_marker_seed: u64,
    pub calib_min_corners: usize,
    pub calib_max_residual_px: f64,
    pub calib_max_method_gap: f64,
    pub calib_check_mm: f64,
    pub calib_check_tol_mm: f64,
    pub calib_allow_unchecked: bool,
    /// `camera`: where the middle of the picture is in the frame of the machine, in millimetres, and how the camera is turned, in degrees. This is the only offset there is.
    pub camera_offset_x_mm: f64,
    pub camera_offset_y_mm: f64,
    pub camera_offset_deg: f64,
    /// `align`: the tolerance, the limit of a step and of the whole move, the most moves and the share of the correction asked of each move.
    pub align_tolerance_mm: f64,
    pub align_tolerance_deg: f64,
    pub align_max_step_mm: f64,
    pub align_max_step_deg: f64,
    pub align_max_total_mm: f64,
    pub align_max_steps: usize,
    pub align_gain: f64,
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
            focus_reference: 100.0,
            edge_step_px: 0.25,
            edge_max_uncertainty_px: 0.25,
            server_address: String::new(),
            server_token: String::new(),
            server_machine: String::new(),
            server_timeout_s: 15,
            server_retries: 2,
            server_mdns: false,
            correlate_diameter: 20.0,
            correlate_min_score: 0.8,
            train_min_side: 8,
            train_max_side: 96,
            train_min_contrast: 20.0,
            train_agree_score: 0.8,
            train_max_ambiguity: 0.9,
            train_min_score: 0.8,
            train_turn_step_deg: 5.0,
            train_turn_max_deg: 0.0,
            pair_tolerance_mm: 0.5,
            pair_min_marks: 3,
            pair_max_turn_deg: 180.0,
            find_server_max: 8,
            find_server_private_only: true,
            find_server_tries: 2,
            mjpeg_max_bytes: 16_777_216,
            mjpeg_max_pixels: 25_000_000,
            card_columns: 9,
            card_rows: 7,
            card_pitch_mm: 10.0,
            card_dot_mm: 4.0,
            card_margin_mm: 10.0,
            card_paper_w_mm: 210.0,
            card_paper_h_mm: 297.0,
            light_cells: 4,
            light_saturation_level: 250,
            light_max_saturated: 0.02,
            light_min_mean: 40.0,
            light_max_mean: 215.0,
            light_max_uneven: 0.35,
            light_max_tilt: 0.25,
            light_max_pixels: 25_000_000,
            tape_hole_mm: 1.5,
            tape_pitch_mm: 4.0,
            tape_tolerance_mm: 0.1,
            tape_line_tolerance_mm: 0.15,
            tape_size_tolerance: 0.25,
            tape_min_holes: 4,
            tape_max_holes: 400,
            tape_pocket_pitch_mm: 4.0,
            tape_pocket_along_mm: 2.0,
            tape_pocket_across_mm: 3.5,
            tape_max_pixels: 25_000_000,
            rectify_max_side: 4096,
            pads_tolerance: 0.05,
            diff_threshold: 25,
            diff_min_area: 4,
            diff_max_changed: 0.01,
            orientation_body_contrast: 30.0,
            orientation_mark_contrast: 40.0,
            orientation_min_mark_px: 4,
            orientation_tolerance_deg: 10.0,
            footprint_tolerance_mm: 0.05,
            sharpmap_cells: 8,
            sharpmap_min_share: 0.5,
            sharpmap_min_contrast: 3.0,
            board_square_mm: 0.0,
            board_max_aspect: 0.02,
            board_max_spread: 0.05,
            capture_backend: "auto".into(),
            capture_camera: String::new(),
            capture_format: String::new(),
            capture_profile: String::new(),
            capture_log_level: "warn".into(),
            capture_usb_mbps: 320.0,
            capture_mjpeg_bits_px: 2.0,
            capture_black_mean: 4.0,
            capture_frozen_frames: 5,
            capture_flicker_max: 0.02,
            capture_powerline_hz: 50,
            capture_settle_frames: 3,
            capture_grab_tries: 30,
            calib_squares_x: 8,
            calib_squares_y: 6,
            calib_square_mm: 5.0,
            calib_marker_seed: 1,
            calib_min_corners: 12,
            calib_max_residual_px: 0.5,
            calib_max_method_gap: 0.02,
            calib_check_mm: 0.0,
            calib_check_tol_mm: 0.05,
            calib_allow_unchecked: false,
            camera_offset_x_mm: 0.0,
            camera_offset_y_mm: 0.0,
            camera_offset_deg: 0.0,
            align_tolerance_mm: 0.05,
            align_tolerance_deg: 0.1,
            align_max_step_mm: 2.0,
            align_max_step_deg: 2.0,
            align_max_total_mm: 10.0,
            align_max_steps: 6,
            align_gain: 1.0,
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

    /// The settings of the reading of an edge that the values make.
    pub fn edge_settings(&self) -> crate::edge::EdgeSettings {
        crate::edge::EdgeSettings {
            step_px: self.edge_step_px,
            max_uncertainty_px: self.edge_max_uncertainty_px,
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

    /// The settings of the search by correlation that the values make (the polarity is the one of the other searches).
    pub fn correlate_settings(&self) -> crate::correlate::CorrelateSettings {
        crate::correlate::CorrelateSettings {
            polarity: self.polarity,
            diameter: self.correlate_diameter,
            min_score: self.correlate_min_score,
            max_matches: self.max_parts,
        }
    }

    /// The settings of the training of a mark and of the search with it that the values make.
    pub fn train_settings(&self) -> crate::train::TrainSettings {
        crate::train::TrainSettings {
            min_side: self.train_min_side,
            max_side: self.train_max_side,
            min_contrast: self.train_min_contrast,
            agree_score: self.train_agree_score,
            max_ambiguity: self.train_max_ambiguity,
            min_score: self.train_min_score,
            turn_step_deg: self.train_turn_step_deg,
            turn_max_deg: self.train_turn_max_deg,
            max_matches: self.max_parts,
        }
    }

    /// The settings of the pairing of the marks with the design whatever the turn that the values make (the stretch is the one of the alignment).
    pub fn pair_settings(&self) -> crate::turnpair::PairSettings {
        crate::turnpair::PairSettings {
            tolerance_mm: self.pair_tolerance_mm,
            min_marks: self.pair_min_marks,
            max_turn_deg: self.pair_max_turn_deg,
            max_stretch: self.max_stretch,
        }
    }

    /// The settings of the list of candidates of the search for the server on the network that the values make.
    pub fn find_settings(&self) -> crate::discovery::FindSettings {
        crate::discovery::FindSettings {
            max_candidates: self.find_server_max,
            private_only: self.find_server_private_only,
            tries: self.find_server_tries,
        }
    }

    /// The limits of the reader of motion JPEG frames that the values make.
    pub fn mjpeg_settings(&self) -> crate::mjpeg::JpegSettings {
        crate::mjpeg::JpegSettings {
            max_bytes: self.mjpeg_max_bytes,
            max_pixels: self.mjpeg_max_pixels,
        }
    }

    /// The settings of the card of calibration to print that the values make.
    pub fn card_settings(&self) -> crate::dotcard::CardSettings {
        crate::dotcard::CardSettings {
            columns: self.card_columns,
            rows: self.card_rows,
            pitch_mm: self.card_pitch_mm,
            dot_mm: self.card_dot_mm,
            margin_mm: self.card_margin_mm,
            paper_w_mm: self.card_paper_w_mm,
            paper_h_mm: self.card_paper_h_mm,
        }
    }

    /// The settings of the check of the light of a picture that the values make.
    pub fn light_settings(&self) -> crate::light::LightSettings {
        crate::light::LightSettings {
            cells: self.light_cells,
            saturation_level: self.light_saturation_level.clamp(1, 255) as u8,
            max_saturated: self.light_max_saturated,
            min_mean: self.light_min_mean,
            max_mean: self.light_max_mean,
            max_uneven: self.light_max_uneven,
            max_tilt: self.light_max_tilt,
            max_pixels: self.light_max_pixels,
        }
    }

    /// The settings of the search for the holes of a tape that the values make (the holes are as dark or as light as the marks are: `polarity`).
    pub fn tape_settings(&self) -> crate::tape::TapeSettings {
        crate::tape::TapeSettings {
            hole_mm: self.tape_hole_mm,
            pitch_mm: self.tape_pitch_mm,
            tolerance_mm: self.tape_tolerance_mm,
            line_tolerance_mm: self.tape_line_tolerance_mm,
            size_tolerance: self.tape_size_tolerance,
            min_holes: self.tape_min_holes,
            max_holes: self.tape_max_holes,
            pocket_pitch_mm: self.tape_pocket_pitch_mm,
            pocket_along_mm: self.tape_pocket_along_mm,
            pocket_across_mm: self.tape_pocket_across_mm,
            polarity: self.polarity,
            max_pixels: self.tape_max_pixels,
        }
    }

    /// The settings of the check of a row of pads that the values make (no pitch is asked for: that is an option of the command).
    pub fn pad_settings(&self) -> crate::pads::PadSettings {
        crate::pads::PadSettings {
            tolerance: self.pads_tolerance,
            expected_pitch_mm: None,
        }
    }

    /// The settings of the comparison of two pictures that the values make.
    pub fn diff_settings(&self) -> crate::diff::DiffSettings {
        crate::diff::DiffSettings {
            threshold: self.diff_threshold,
            min_area: self.diff_min_area,
            max_changed: self.diff_max_changed,
        }
    }

    /// The settings of the reading of the way a part is held that the values make (the direction asked for is not in the file).
    pub fn orientation_settings(&self) -> crate::orientation::OrientationSettings {
        crate::orientation::OrientationSettings {
            body_contrast: self.orientation_body_contrast,
            mark_contrast: self.orientation_mark_contrast,
            min_mark_px: self.orientation_min_mark_px,
            tolerance_deg: self.orientation_tolerance_deg,
            expected_deg: None,
        }
    }

    /// The settings of the map of sharpness that the values make.
    pub fn sharp_settings(&self) -> crate::sharpmap::SharpSettings {
        crate::sharpmap::SharpSettings {
            cells: self.sharpmap_cells,
            min_share: self.sharpmap_min_share,
            min_contrast: self.sharpmap_min_contrast,
        }
    }

    /// The settings of the scale from a board of squares that the values make (`pitch_mm` is the distance between the centres).
    pub fn board_settings(&self) -> crate::board::BoardSettings {
        crate::board::BoardSettings {
            square_mm: self.board_square_mm,
            pitch_mm: self.pitch_mm,
            max_aspect: self.board_max_aspect,
            max_spread: self.board_max_spread,
        }
    }

    /// The budget of a USB bus that the values make.
    pub fn budget_settings(&self) -> crate::capture::budget::BudgetSettings {
        crate::capture::budget::BudgetSettings {
            usb_mbps: self.capture_usb_mbps,
            mjpeg_bits_px: self.capture_mjpeg_bits_px,
        }
    }

    /// The judging of a stream that the values make.
    pub fn health_settings(&self) -> crate::capture::health::HealthSettings {
        crate::capture::health::HealthSettings {
            black_mean: self.capture_black_mean,
            frozen_frames: self.capture_frozen_frames,
            flicker_max: self.capture_flicker_max,
            powerline_hz: self.capture_powerline_hz,
        }
    }

    /// The wait for new frames that the values make.
    pub fn settle_settings(&self) -> crate::capture::settle::SettleSettings {
        crate::capture::settle::SettleSettings {
            frames: self.capture_settle_frames,
            tries: self.capture_grab_tries,
        }
    }

    /// The board of calibration that the values make.
    pub fn board_spec(&self) -> crate::charuco::BoardSpec {
        crate::charuco::BoardSpec {
            cols: self.calib_squares_x,
            rows: self.calib_squares_y,
            square_mm: self.calib_square_mm,
            seed: self.calib_marker_seed,
        }
    }

    /// What is asked of a calibration that the values make.
    pub fn calib_settings(&self) -> crate::charuco::solve::Settings {
        crate::charuco::solve::Settings {
            min_corners: self.calib_min_corners,
            max_residual_px: self.calib_max_residual_px,
            max_method_gap: self.calib_max_method_gap,
            check_mm: self.calib_check_mm,
            check_tol_mm: self.calib_check_tol_mm,
            allow_unchecked: self.calib_allow_unchecked,
        }
    }

    /// The offset of the camera that the values make.
    pub fn camera_offset(&self) -> crate::charuco::solve::CameraOffset {
        crate::charuco::solve::CameraOffset {
            x_mm: self.camera_offset_x_mm,
            y_mm: self.camera_offset_y_mm,
            deg: self.camera_offset_deg,
        }
    }

    /// The alignment of a board that the values make.
    pub fn aim_settings(&self) -> crate::boardaim::AimSettings {
        crate::boardaim::AimSettings {
            tolerance_mm: self.align_tolerance_mm,
            tolerance_deg: self.align_tolerance_deg,
            max_step_mm: self.align_max_step_mm,
            max_step_deg: self.align_max_step_deg,
            max_total_mm: self.align_max_total_mm,
            max_steps: self.align_max_steps,
            gain: self.align_gain,
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
            "focus_reference" => self.focus_reference = number(value, key)?,
            "edge_step_px" => self.edge_step_px = number(value, key)?,
            "edge_max_uncertainty_px" => self.edge_max_uncertainty_px = number(value, key)?,
            "server_address" => self.server_address = value.to_string(),
            "server_token" => self.server_token = value.to_string(),
            "server_machine" => self.server_machine = value.to_string(),
            "server_timeout_s" => self.server_timeout_s = count(value, key)? as u64,
            "server_retries" => {
                self.server_retries = u32::try_from(count(value, key)?)
                    .map_err(|_| format!("{key} '{value}' is too large"))?
            }
            "server_mdns" => self.server_mdns = yes_no(value, key)?,
            "correlate_diameter" => self.correlate_diameter = number(value, key)?,
            "correlate_min_score" => self.correlate_min_score = number(value, key)?,
            "train_min_side" => self.train_min_side = count(value, key)?,
            "train_max_side" => self.train_max_side = count(value, key)?,
            "train_min_contrast" => self.train_min_contrast = number(value, key)?,
            "train_agree_score" => self.train_agree_score = number(value, key)?,
            "train_max_ambiguity" => self.train_max_ambiguity = number(value, key)?,
            "train_min_score" => self.train_min_score = number(value, key)?,
            "train_turn_step_deg" => self.train_turn_step_deg = number(value, key)?,
            "train_turn_max_deg" => self.train_turn_max_deg = number(value, key)?,
            "pair_tolerance_mm" => self.pair_tolerance_mm = number(value, key)?,
            "pair_min_marks" => self.pair_min_marks = count(value, key)?,
            "pair_max_turn_deg" => self.pair_max_turn_deg = number(value, key)?,
            "find_server_max" => self.find_server_max = count(value, key)?,
            "find_server_private_only" => self.find_server_private_only = yes_no(value, key)?,
            "find_server_tries" => self.find_server_tries = count(value, key)?,
            "mjpeg_max_bytes" => self.mjpeg_max_bytes = count(value, key)?,
            "mjpeg_max_pixels" => self.mjpeg_max_pixels = count(value, key)?,
            "card_columns" => self.card_columns = count(value, key)?,
            "card_rows" => self.card_rows = count(value, key)?,
            "card_pitch_mm" => self.card_pitch_mm = number(value, key)?,
            "card_dot_mm" => self.card_dot_mm = number(value, key)?,
            "card_margin_mm" => self.card_margin_mm = number(value, key)?,
            "card_paper_w_mm" => self.card_paper_w_mm = number(value, key)?,
            "card_paper_h_mm" => self.card_paper_h_mm = number(value, key)?,
            "light_cells" => self.light_cells = count(value, key)?,
            "light_saturation_level" => self.light_saturation_level = count(value, key)?,
            "light_max_saturated" => self.light_max_saturated = number(value, key)?,
            "light_min_mean" => self.light_min_mean = number(value, key)?,
            "light_max_mean" => self.light_max_mean = number(value, key)?,
            "light_max_uneven" => self.light_max_uneven = number(value, key)?,
            "light_max_tilt" => self.light_max_tilt = number(value, key)?,
            "light_max_pixels" => self.light_max_pixels = count(value, key)?,
            "tape_hole_mm" => self.tape_hole_mm = number(value, key)?,
            "tape_pitch_mm" => self.tape_pitch_mm = number(value, key)?,
            "tape_tolerance_mm" => self.tape_tolerance_mm = number(value, key)?,
            "tape_line_tolerance_mm" => self.tape_line_tolerance_mm = number(value, key)?,
            "tape_size_tolerance" => self.tape_size_tolerance = number(value, key)?,
            "tape_min_holes" => self.tape_min_holes = count(value, key)?,
            "tape_max_holes" => self.tape_max_holes = count(value, key)?,
            "tape_pocket_pitch_mm" => self.tape_pocket_pitch_mm = number(value, key)?,
            "tape_pocket_along_mm" => self.tape_pocket_along_mm = number(value, key)?,
            "tape_pocket_across_mm" => self.tape_pocket_across_mm = number(value, key)?,
            "tape_max_pixels" => self.tape_max_pixels = count(value, key)?,
            "rectify_max_side" => self.rectify_max_side = count(value, key)?,
            "pads_tolerance" => self.pads_tolerance = number(value, key)?,
            "diff_threshold" => {
                self.diff_threshold = value
                    .parse::<u8>()
                    .map_err(|_| format!("{key} '{value}' is not a whole number from 0 to 255"))?
            }
            "diff_min_area" => self.diff_min_area = count(value, key)? as u64,
            "diff_max_changed" => self.diff_max_changed = number(value, key)?,
            "orientation_body_contrast" => self.orientation_body_contrast = number(value, key)?,
            "orientation_mark_contrast" => self.orientation_mark_contrast = number(value, key)?,
            "orientation_min_mark_px" => self.orientation_min_mark_px = count(value, key)? as u64,
            "orientation_tolerance_deg" => self.orientation_tolerance_deg = number(value, key)?,
            "footprint_tolerance_mm" => self.footprint_tolerance_mm = number(value, key)?,
            "sharpmap_cells" => self.sharpmap_cells = count(value, key)?,
            "sharpmap_min_share" => self.sharpmap_min_share = number(value, key)?,
            "sharpmap_min_contrast" => self.sharpmap_min_contrast = number(value, key)?,
            "board_square_mm" => self.board_square_mm = number(value, key)?,
            "board_max_aspect" => self.board_max_aspect = number(value, key)?,
            "board_max_spread" => self.board_max_spread = number(value, key)?,
            "capture_backend" => self.capture_backend = value.to_string(),
            "capture_camera" => self.capture_camera = value.to_string(),
            "capture_format" => self.capture_format = value.to_string(),
            "capture_profile" => self.capture_profile = value.to_string(),
            "capture_log_level" => self.capture_log_level = value.to_string(),
            "capture_usb_mbps" => self.capture_usb_mbps = number(value, key)?,
            "capture_mjpeg_bits_px" => self.capture_mjpeg_bits_px = number(value, key)?,
            "capture_black_mean" => self.capture_black_mean = number(value, key)?,
            "capture_frozen_frames" => self.capture_frozen_frames = count(value, key)?,
            "capture_flicker_max" => self.capture_flicker_max = number(value, key)?,
            "capture_powerline_hz" => {
                self.capture_powerline_hz = u32::try_from(count(value, key)?)
                    .map_err(|_| format!("{key} '{value}' is too large"))?
            }
            "capture_settle_frames" => self.capture_settle_frames = count(value, key)?,
            "capture_grab_tries" => self.capture_grab_tries = count(value, key)?,
            "calib_squares_x" => self.calib_squares_x = count(value, key)?,
            "calib_squares_y" => self.calib_squares_y = count(value, key)?,
            "calib_square_mm" => self.calib_square_mm = number(value, key)?,
            "calib_marker_seed" => self.calib_marker_seed = count(value, key)? as u64,
            "calib_min_corners" => self.calib_min_corners = count(value, key)?,
            "calib_max_residual_px" => self.calib_max_residual_px = number(value, key)?,
            "calib_max_method_gap" => self.calib_max_method_gap = number(value, key)?,
            "calib_check_mm" => self.calib_check_mm = number(value, key)?,
            "calib_check_tol_mm" => self.calib_check_tol_mm = number(value, key)?,
            "calib_allow_unchecked" => self.calib_allow_unchecked = yes_no(value, key)?,
            "camera_offset_x_mm" => self.camera_offset_x_mm = number(value, key)?,
            "camera_offset_y_mm" => self.camera_offset_y_mm = number(value, key)?,
            "camera_offset_deg" => self.camera_offset_deg = number(value, key)?,
            "align_tolerance_mm" => self.align_tolerance_mm = number(value, key)?,
            "align_tolerance_deg" => self.align_tolerance_deg = number(value, key)?,
            "align_max_step_mm" => self.align_max_step_mm = number(value, key)?,
            "align_max_step_deg" => self.align_max_step_deg = number(value, key)?,
            "align_max_total_mm" => self.align_max_total_mm = number(value, key)?,
            "align_max_steps" => self.align_max_steps = count(value, key)?,
            "align_gain" => self.align_gain = number(value, key)?,
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
        if !(self.focus_reference.is_finite() && self.focus_reference > 0.0) {
            return Err("focus_reference must be more than 0".into());
        }
        crate::edge::check(&self.edge_settings())?;
        crate::server::link_settings(self)?;
        crate::correlate::check(&self.correlate_settings())
            .map_err(|e| format!("correlate_diameter and correlate_min_score: {e}"))?;
        crate::train::check(&self.train_settings()).map_err(|e| format!("train_*: {e}"))?;
        crate::turnpair::check(&self.pair_settings()).map_err(|e| format!("pair_*: {e}"))?;
        crate::discovery::check(&self.find_settings())
            .map_err(|e| format!("find_server_*: {e}"))?;
        crate::mjpeg::check(&self.mjpeg_settings()).map_err(|e| format!("mjpeg_*: {e}"))?;
        crate::dotcard::check(&self.card_settings()).map_err(|e| format!("card_*: {e}"))?;
        if self.light_saturation_level == 0 || self.light_saturation_level > 255 {
            return Err("light_saturation_level must be from 1 to 255".into());
        }
        crate::light::check(&self.light_settings()).map_err(|e| format!("light_*: {e}"))?;
        crate::tape::check(&self.tape_settings()).map_err(|e| format!("tape_*: {e}"))?;
        if !(crate::perspective::MIN_SIDE..=crate::perspective::MAX_SIDE_LIMIT)
            .contains(&self.rectify_max_side)
        {
            return Err(format!(
                "rectify_max_side must be from {} to {}",
                crate::perspective::MIN_SIDE,
                crate::perspective::MAX_SIDE_LIMIT
            ));
        }
        crate::pads::check(&self.pad_settings()).map_err(|e| format!("pads_tolerance: {e}"))?;
        crate::diff::check(&self.diff_settings())
            .map_err(|e| format!("diff_threshold, diff_min_area and diff_max_changed: {e}"))?;
        crate::orientation::check(&self.orientation_settings())?;
        if !(self.footprint_tolerance_mm.is_finite() && self.footprint_tolerance_mm > 0.0) {
            return Err("footprint_tolerance_mm must be more than 0".into());
        }
        crate::sharpmap::check(&self.sharp_settings())?;
        if !(self.board_square_mm.is_finite() && self.board_square_mm >= 0.0) {
            return Err("board_square_mm must be 0 (not known) or more".into());
        }
        for (v, name) in [
            (self.board_max_aspect, "board_max_aspect"),
            (self.board_max_spread, "board_max_spread"),
        ] {
            if !(v.is_finite() && v > 0.0 && v <= 1.0) {
                return Err(format!("{name} must be more than 0 and at most 1"));
            }
        }
        self.validate_capture().map_err(|e| e.to_string())?;
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

    /// The values of the live camera, the calibration and the alignment of a board, each checked by the module that uses it; the error names the key.
    fn validate_capture(&self) -> Result<(), crate::capture::CapError> {
        use crate::capture::{choice, CameraId, CapError, Code};
        choice::check_backend(&self.capture_backend)?;
        if !self.capture_camera.is_empty() {
            CameraId::parse(&self.capture_camera)?;
        }
        choice::parse_format(&self.capture_format)?;
        choice::parse_profile(&self.capture_profile)?;
        if crate::capture::log::Level::from_name(&self.capture_log_level).is_none() {
            return Err(CapError::with_key(
                Code::BackendUnavailable,
                "capture_log_level",
                "it must be error, warn, info or debug",
            ));
        }
        crate::capture::budget::check_settings(&self.budget_settings())?;
        crate::capture::health::check(&self.health_settings())?;
        crate::capture::settle::check(&self.settle_settings())?;
        self.board_spec()
            .check()
            .map_err(|e| CapError::with_key(Code::CalibFewCorners, "calib_squares_x", e))?;
        crate::charuco::solve::check_settings(&self.calib_settings())?;
        let off = |v: f64, max: f64, key: &'static str| {
            if v.is_finite() && v.abs() <= max {
                Ok(())
            } else {
                Err(CapError::with_key(
                    Code::CalibResidual,
                    key,
                    format!("it must be from -{max} to {max}"),
                ))
            }
        };
        off(self.camera_offset_x_mm, 1000.0, "camera_offset_x_mm")?;
        off(self.camera_offset_y_mm, 1000.0, "camera_offset_y_mm")?;
        off(self.camera_offset_deg, 360.0, "camera_offset_deg")?;
        crate::boardaim::check(&self.aim_settings())
    }

    /// The values of a text: lines of `key = value`, blank lines and lines that start with `#` left out. A key that is written twice, an unknown key or a bad value is an error with the line.
    pub fn parse(text: &str) -> Result<Config, String> {
        // a byte order mark is not part of the first key
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
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
        use std::io::Read;
        let mut bytes = Vec::new();
        // one byte past the limit is enough to know it is too large; a device or a pipe tells no size
        std::fs::File::open(path)
            .and_then(|f| f.take(65_537).read_to_end(&mut bytes))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if bytes.len() > 65_536 {
            return Err(format!(
                "{}: a configuration file is not larger than 64 KiB",
                path.display()
            ));
        }
        let text = String::from_utf8(bytes)
            .map_err(|_| format!("{}: the file is not text", path.display()))?;
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
            "focus_reference = 0",
            "focus_reference = -3",
            "focus_reference = soon",
            "edge_step_px = 0",
            "edge_step_px = 0.01",
            "edge_step_px = 2",
            "edge_max_uncertainty_px = 0",
            "edge_max_uncertainty_px = 9",
            "quality_min_mean = 200
quality_max_mean = 100",
            "quality_max_mean = 300",
            "shots_part_height = -0.5",
            "server_address = nohost",
            "server_address = host:0",
            "server_address = a b:1",
            "server_token = has space",
            "server_machine = a/b",
            "server_timeout_s = 0",
            "server_timeout_s = 121",
            "server_timeout_s = soon",
            "server_retries = 6",
            "server_retries = -1",
            "server_mdns = sometimes",
            "orientation_body_contrast = 0",
            "orientation_body_contrast = 256",
            "orientation_mark_contrast = 0",
            "orientation_min_mark_px = 0",
            "orientation_min_mark_px = -1",
            "orientation_tolerance_deg = 91",
            "footprint_tolerance_mm = 0",
            "footprint_tolerance_mm = -0.1",
            "sharpmap_cells = 1",
            "sharpmap_cells = 65",
            "sharpmap_min_share = 0",
            "sharpmap_min_share = 1.1",
            "sharpmap_min_contrast = 256",
            "board_square_mm = -1",
            "board_max_aspect = 0",
            "board_max_aspect = 2",
            "board_max_spread = 0",
            "board_max_spread = nan",
            "correlate_diameter = 3",
            "correlate_diameter = 200",
            "correlate_min_score = 0",
            "correlate_min_score = 1.1",
            "train_min_side = 3",
            "train_max_side = 7",
            "train_max_side = 257",
            "train_min_contrast = 256",
            "train_agree_score = 0",
            "train_max_ambiguity = 1",
            "train_min_score = 0",
            "train_turn_step_deg = 0.5",
            "train_turn_step_deg = 46",
            "train_turn_max_deg = 181",
            "pair_tolerance_mm = 0",
            "pair_tolerance_mm = 51",
            "pair_min_marks = 1",
            "pair_min_marks = 25",
            "pair_max_turn_deg = 181",
            "find_server_max = 0",
            "find_server_max = 65",
            "find_server_private_only = maybe",
            "find_server_tries = 0",
            "find_server_tries = 6",
            "mjpeg_max_bytes = 10",
            "mjpeg_max_pixels = 30000000",
            "card_columns = 2",
            "card_dot_mm = 9",
            "card_paper_w_mm = 10",
            "light_cells = 1",
            "light_saturation_level = 0",
            "light_saturation_level = 256",
            "light_max_saturated = 2",
            "light_max_tilt = 3",
            "tape_hole_mm = 9",
            "tape_min_holes = 1",
            "tape_size_tolerance = 2",
            "tape_max_pixels = 1",
            "rectify_max_side = 1",
            "rectify_max_side = 16385",
            "pads_tolerance = 0",
            "pads_tolerance = 2",
            "diff_threshold = 255",
            "diff_threshold = 300",
            "diff_min_area = 0",
            "diff_max_changed = 1.5",
            "diff_max_changed = -0.1",
            "capture_backend = directshow",
            "capture_camera = nonsense",
            "capture_format = 640x480",
            "capture_profile = colour=3",
            "capture_log_level = loud",
            "capture_usb_mbps = 5",
            "capture_mjpeg_bits_px = 0.1",
            "capture_black_mean = 300",
            "capture_frozen_frames = 1",
            "capture_flicker_max = 0",
            "capture_powerline_hz = 55",
            "capture_settle_frames = 500",
            "capture_grab_tries = 0",
            "capture_grab_tries = 2
capture_settle_frames = 3",
            "calib_squares_x = 2",
            "calib_squares_y = 30",
            "calib_square_mm = 0.5",
            "calib_marker_seed = -1",
            "calib_min_corners = 3",
            "calib_max_residual_px = 0",
            "calib_max_method_gap = 2",
            "calib_check_mm = -5",
            "calib_check_tol_mm = 0",
            "calib_allow_unchecked = maybe",
            "camera_offset_x_mm = 2000",
            "camera_offset_y_mm = nan",
            "camera_offset_deg = 361",
            "align_tolerance_mm = 0",
            "align_tolerance_deg = 6",
            "align_max_step_mm = 51",
            "align_max_step_deg = 0",
            "align_max_total_mm = 501",
            "align_max_steps = 0",
            "align_gain = 0.05",
            "align_gain = 1.5",
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
