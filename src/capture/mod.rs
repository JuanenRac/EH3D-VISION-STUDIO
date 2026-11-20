//! A live camera behind one trait. A backend lists the cameras by name with an identifier that does not change between runs, says which formats each one gives (YUYV, MJPEG, RGB, grey),
//! opens one at a format, hands over frames with a counter and the original bytes, and answers for each property (exposure, focus, gain, white balance, zoom...) with its range or with "this
//! camera has no such thing". The backend of Windows, the one of V4L2 and the one of libcamera are thin and sit behind features that are off by default; the backend that is always there is the
//! simulated one, which draws frames and fails on purpose, so that everything above the camera is tested without a camera. Nothing here has been run on a real camera.

pub mod budget;
pub mod camid;
pub mod choice;
pub mod fourcc;
pub mod health;
#[cfg(all(feature = "libcamera", target_os = "linux"))]
pub mod libcam;
pub mod log;
#[cfg(all(feature = "mfoundation", target_os = "windows"))]
pub mod mf;
pub mod pixels;
pub mod settle;
pub mod sim;
#[cfg(all(feature = "v4l2", target_os = "linux"))]
pub mod v4l2;

use crate::image::Gray;
use crate::lang::{Lang, Msg};
use std::fmt;

/// How the bytes of a frame are arranged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    /// Packed YUV 4:2:2, two pixels in four bytes (Y0 U Y1 V).
    Yuyv,
    /// A JPEG picture for each frame.
    Mjpeg,
    /// Three bytes for each pixel, red first.
    Rgb24,
    /// One byte for each pixel.
    Gray8,
}

impl PixelFormat {
    /// Every format, in the order of the lists.
    pub const ALL: [PixelFormat; 4] = [
        PixelFormat::Yuyv,
        PixelFormat::Mjpeg,
        PixelFormat::Rgb24,
        PixelFormat::Gray8,
    ];

    /// The name that is written in the configuration and in the lists.
    pub fn name(self) -> &'static str {
        match self {
            PixelFormat::Yuyv => "yuyv",
            PixelFormat::Mjpeg => "mjpeg",
            PixelFormat::Rgb24 => "rgb24",
            PixelFormat::Gray8 => "gray8",
        }
    }

    /// The format with that name.
    pub fn from_name(name: &str) -> Option<PixelFormat> {
        PixelFormat::ALL.into_iter().find(|f| f.name() == name)
    }

    /// Bits of a pixel on the wire; `None` for MJPEG, which depends on the picture.
    pub fn bits_per_pixel(self) -> Option<u32> {
        match self {
            PixelFormat::Yuyv => Some(16),
            PixelFormat::Rgb24 => Some(24),
            PixelFormat::Gray8 => Some(8),
            PixelFormat::Mjpeg => None,
        }
    }
}

/// One way a camera can give frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FormatDesc {
    pub width: u32,
    pub height: u32,
    /// Frames each second; 0 means that the camera did not say (the format is then not usable).
    pub fps: u32,
    pub pixel: PixelFormat,
}

impl FormatDesc {
    /// A format that has a size and a rate. A size of 0 or a rate of 0 is what a camera that is not really there (a CSI camera seen through V4L2) lists.
    pub fn is_usable(&self) -> bool {
        self.width > 0 && self.height > 0 && self.fps > 0
    }
}

/// The camera properties that are looked after.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Property {
    Exposure,
    Focus,
    Zoom,
    WhiteBalance,
    Gain,
    Brightness,
    Contrast,
    Saturation,
    Gamma,
    Hue,
    Sharpness,
    Backlight,
    /// The frequency of the mains light, 50 or 60 (the camera removes the flicker of it).
    PowerlineHz,
}

impl Property {
    /// Every property, once.
    pub const ALL: [Property; 13] = [
        Property::Exposure,
        Property::Focus,
        Property::Zoom,
        Property::WhiteBalance,
        Property::Gain,
        Property::Brightness,
        Property::Contrast,
        Property::Saturation,
        Property::Gamma,
        Property::Hue,
        Property::Sharpness,
        Property::Backlight,
        Property::PowerlineHz,
    ];

    /// The name used in the configuration and in the lists.
    pub fn name(self) -> &'static str {
        match self {
            Property::Exposure => "exposure",
            Property::Focus => "focus",
            Property::Zoom => "zoom",
            Property::WhiteBalance => "white_balance",
            Property::Gain => "gain",
            Property::Brightness => "brightness",
            Property::Contrast => "contrast",
            Property::Saturation => "saturation",
            Property::Gamma => "gamma",
            Property::Hue => "hue",
            Property::Sharpness => "sharpness",
            Property::Backlight => "backlight",
            Property::PowerlineHz => "powerline_hz",
        }
    }

    /// The property with that name.
    pub fn from_name(name: &str) -> Option<Property> {
        Property::ALL.into_iter().find(|p| p.name() == name)
    }
}

/// What a camera allows for one property.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyRange {
    pub min: f64,
    pub max: f64,
    /// The smallest change; 0 when any value goes.
    pub step: f64,
    pub default: f64,
    /// Whether the camera can set the property by itself (auto exposure, auto focus...).
    pub auto_supported: bool,
}

impl PropertyRange {
    /// Whether `value` is inside the range and on a step.
    pub fn allows(&self, value: f64) -> bool {
        if !value.is_finite() || value < self.min || value > self.max {
            return false;
        }
        if self.step > 0.0 {
            let k = (value - self.min) / self.step;
            return (k - k.round()).abs() < 1e-6;
        }
        true
    }
}

/// The value of a property: a number, or "the camera decides".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyValue {
    pub value: f64,
    pub auto: bool,
}

impl PropertyValue {
    /// A fixed value.
    pub fn fixed(value: f64) -> PropertyValue {
        PropertyValue { value, auto: false }
    }
    /// Left to the camera.
    pub fn auto() -> PropertyValue {
        PropertyValue {
            value: 0.0,
            auto: true,
        }
    }
}

/// How a camera is told from the others: the same camera gives the same key on every run and on every port.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CameraId {
    pub vendor: u16,
    pub product: u16,
    /// The serial number of the device; empty when it has none.
    pub serial: String,
    /// Where it is plugged (bus and ports), or the name of the CSI connector; empty when unknown.
    pub port: String,
}

/// What a backend knows about one camera.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraInfo {
    pub name: String,
    pub id: CameraId,
    pub formats: Vec<FormatDesc>,
}

impl CameraInfo {
    /// The formats that can be used: the ones with a size and a rate.
    pub fn usable_formats(&self) -> Vec<FormatDesc> {
        self.formats
            .iter()
            .copied()
            .filter(FormatDesc::is_usable)
            .collect()
    }
}

/// One frame from a camera.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// The number of the frame since the camera was opened, from 1 without gaps in what the camera counted (a gap is a lost frame).
    pub seq: u64,
    /// When the frame was taken, in microseconds on the clock of the camera (the simulated camera counts its own).
    pub time_us: u64,
    /// The brightness of each pixel, ready for the readings of the program.
    pub luma: Gray,
    /// The bytes as the camera sent them (the JPEG of a motion JPEG frame), for evidence and for work without the decoding.
    pub raw: Vec<u8>,
    pub raw_format: PixelFormat,
}

/// The codes of the errors of the capture; they do not change, and each has its words in the eight languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    BackendUnavailable,
    BackendUnsupported,
    CameraNotFound,
    CameraDisconnected,
    CameraBusy,
    FormatUnsupported,
    FormatInvalid,
    PropertyUnsupported,
    PropertyOutOfRange,
    PropertyAutoUnsupported,
    FrameTimeout,
    FrameTruncated,
    FrameBadSize,
    FrameNotDecoded,
    StreamBlack,
    StreamFrozen,
    UsbBandwidth,
    NoNewFrames,
    CalibFewCorners,
    CalibAmbiguous,
    CalibResidual,
    CalibMethodsDisagree,
    CalibCheckFailed,
    CalibUnverified,
    CalibFileBad,
    AlignNoMarks,
    AlignStepLimit,
    AlignDiverging,
    AlignNotConverged,
    AlignTotalLimit,
    AlignStageError,
    AlignCameraUnsafe,
}

impl Code {
    /// Every code, once.
    pub const ALL: [Code; 32] = [
        Code::BackendUnavailable,
        Code::BackendUnsupported,
        Code::CameraNotFound,
        Code::CameraDisconnected,
        Code::CameraBusy,
        Code::FormatUnsupported,
        Code::FormatInvalid,
        Code::PropertyUnsupported,
        Code::PropertyOutOfRange,
        Code::PropertyAutoUnsupported,
        Code::FrameTimeout,
        Code::FrameTruncated,
        Code::FrameBadSize,
        Code::FrameNotDecoded,
        Code::StreamBlack,
        Code::StreamFrozen,
        Code::UsbBandwidth,
        Code::NoNewFrames,
        Code::CalibFewCorners,
        Code::CalibAmbiguous,
        Code::CalibResidual,
        Code::CalibMethodsDisagree,
        Code::CalibCheckFailed,
        Code::CalibUnverified,
        Code::CalibFileBad,
        Code::AlignNoMarks,
        Code::AlignStepLimit,
        Code::AlignDiverging,
        Code::AlignNotConverged,
        Code::AlignTotalLimit,
        Code::AlignStageError,
        Code::AlignCameraUnsafe,
    ];

    /// The text that starts an error and does not change.
    pub fn as_str(self) -> &'static str {
        match self {
            Code::BackendUnavailable => "backend_unavailable",
            Code::BackendUnsupported => "backend_unsupported",
            Code::CameraNotFound => "camera_not_found",
            Code::CameraDisconnected => "camera_disconnected",
            Code::CameraBusy => "camera_busy",
            Code::FormatUnsupported => "format_unsupported",
            Code::FormatInvalid => "format_invalid",
            Code::PropertyUnsupported => "property_unsupported",
            Code::PropertyOutOfRange => "property_out_of_range",
            Code::PropertyAutoUnsupported => "property_auto_unsupported",
            Code::FrameTimeout => "frame_timeout",
            Code::FrameTruncated => "frame_truncated",
            Code::FrameBadSize => "frame_bad_size",
            Code::FrameNotDecoded => "frame_not_decoded",
            Code::StreamBlack => "stream_black",
            Code::StreamFrozen => "stream_frozen",
            Code::UsbBandwidth => "usb_bandwidth",
            Code::NoNewFrames => "no_new_frames",
            Code::CalibFewCorners => "calib_few_corners",
            Code::CalibAmbiguous => "calib_ambiguous",
            Code::CalibResidual => "calib_residual",
            Code::CalibMethodsDisagree => "calib_methods_disagree",
            Code::CalibCheckFailed => "calib_check_failed",
            Code::CalibUnverified => "calib_unverified",
            Code::CalibFileBad => "calib_file_bad",
            Code::AlignNoMarks => "align_no_marks",
            Code::AlignStepLimit => "align_step_limit",
            Code::AlignDiverging => "align_diverging",
            Code::AlignNotConverged => "align_not_converged",
            Code::AlignTotalLimit => "align_total_limit",
            Code::AlignStageError => "align_stage_error",
            Code::AlignCameraUnsafe => "align_camera_unsafe",
        }
    }

    /// The sentence that says what the code means, in a language.
    pub fn text(self, lang: Lang) -> &'static str {
        let m = match self {
            Code::BackendUnavailable => Msg::CapBackendUnavailable,
            Code::BackendUnsupported => Msg::CapBackendUnsupported,
            Code::CameraNotFound => Msg::CapCameraNotFound,
            Code::CameraDisconnected => Msg::CapCameraDisconnected,
            Code::CameraBusy => Msg::CapCameraBusy,
            Code::FormatUnsupported => Msg::CapFormatUnsupported,
            Code::FormatInvalid => Msg::CapFormatInvalid,
            Code::PropertyUnsupported => Msg::CapPropertyUnsupported,
            Code::PropertyOutOfRange => Msg::CapPropertyOutOfRange,
            Code::PropertyAutoUnsupported => Msg::CapPropertyAutoUnsupported,
            Code::FrameTimeout => Msg::CapFrameTimeout,
            Code::FrameTruncated => Msg::CapFrameTruncated,
            Code::FrameBadSize => Msg::CapFrameBadSize,
            Code::FrameNotDecoded => Msg::CapFrameNotDecoded,
            Code::StreamBlack => Msg::CapStreamBlack,
            Code::StreamFrozen => Msg::CapStreamFrozen,
            Code::UsbBandwidth => Msg::CapUsbBandwidth,
            Code::NoNewFrames => Msg::CapNoNewFrames,
            Code::CalibFewCorners => Msg::CapCalibFewCorners,
            Code::CalibAmbiguous => Msg::CapCalibAmbiguous,
            Code::CalibResidual => Msg::CapCalibResidual,
            Code::CalibMethodsDisagree => Msg::CapCalibMethodsDisagree,
            Code::CalibCheckFailed => Msg::CapCalibCheckFailed,
            Code::CalibUnverified => Msg::CapCalibUnverified,
            Code::CalibFileBad => Msg::CapCalibFileBad,
            Code::AlignNoMarks => Msg::CapAlignNoMarks,
            Code::AlignStepLimit => Msg::CapAlignStepLimit,
            Code::AlignDiverging => Msg::CapAlignDiverging,
            Code::AlignNotConverged => Msg::CapAlignNotConverged,
            Code::AlignTotalLimit => Msg::CapAlignTotalLimit,
            Code::AlignStageError => Msg::CapAlignStageError,
            Code::AlignCameraUnsafe => Msg::CapAlignCameraUnsafe,
        };
        m.tr(lang)
    }
}

/// An error of the capture: its code, the key of the configuration that governs it (when one does) and the detail.
#[derive(Debug, Clone, PartialEq)]
pub struct CapError {
    pub code: Code,
    /// The key of `vision.cfg` that the person changes to change the result; `None` when no key governs it.
    pub key: Option<&'static str>,
    pub detail: String,
}

impl CapError {
    /// An error with no key.
    pub fn new(code: Code, detail: impl Into<String>) -> CapError {
        CapError {
            code,
            key: None,
            detail: detail.into(),
        }
    }

    /// An error that names the key of the configuration that governs it.
    pub fn with_key(code: Code, key: &'static str, detail: impl Into<String>) -> CapError {
        CapError {
            code,
            key: Some(key),
            detail: detail.into(),
        }
    }

    /// The error as a person reads it in a language: the sentence of the code, the key and the detail.
    pub fn text(&self, lang: Lang) -> String {
        let mut out = self.code.text(lang).to_string();
        if let Some(k) = self.key {
            out.push_str(&format!(" [{k}]"));
        }
        if !self.detail.is_empty() {
            out.push_str(": ");
            out.push_str(&self.detail);
        }
        out
    }
}

impl fmt::Display for CapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code.as_str())?;
        if let Some(k) = self.key {
            write!(f, ": {k}")?;
        }
        write!(f, ": {}", self.detail)
    }
}

impl std::error::Error for CapError {}

impl From<CapError> for String {
    fn from(e: CapError) -> String {
        e.to_string()
    }
}

/// A source of cameras.
pub trait CaptureBackend {
    /// The name of the backend (`sim`, `v4l2`, `mfoundation`, `libcamera`).
    fn name(&self) -> &'static str;

    /// What the frames of this backend prove (a word of `convention::EVIDENCE`): `camera` for a real one, `simulator` for the simulated one.
    fn evidence(&self) -> &'static str {
        "camera"
    }

    /// The cameras that are there. A camera with no usable format is not listed (a CSI camera seen through V4L2 shows a size of 0 and no rate).
    fn enumerate(&mut self) -> Result<Vec<CameraInfo>, CapError>;

    /// Opens the camera at a format that it lists.
    fn open(&mut self, id: &CameraId, format: FormatDesc) -> Result<Box<dyn Camera>, CapError>;
}

/// An open camera.
pub trait Camera {
    fn info(&self) -> &CameraInfo;

    /// What the frames of this camera prove, as the backend that opened it says.
    fn evidence(&self) -> &'static str {
        "camera"
    }

    fn format(&self) -> FormatDesc;

    /// The range of a property, or `None` when this camera has no such property.
    fn property_range(&self, p: Property) -> Option<PropertyRange>;

    /// The value of a property; `property_unsupported` when the camera has none.
    fn get(&self, p: Property) -> Result<PropertyValue, CapError>;

    /// Sets a property inside its range, or leaves it to the camera when it can.
    fn set(&mut self, p: Property, v: PropertyValue) -> Result<(), CapError>;

    /// Whether a frame has arrived that `grab` has not given yet.
    fn has_new_frame(&self) -> bool;

    /// How many frames the camera has made since it was opened (the ones `grab` gave and the ones that are waiting).
    fn frame_count(&self) -> u64;

    /// The next frame, or the reason there is none.
    fn grab(&mut self) -> Result<Frame, CapError>;

    /// Lets go of the camera; asking for frames after that is `camera_disconnected`.
    fn close(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_a_distinct_name_and_a_sentence_in_each_of_the_eight_languages() {
        for (i, a) in Code::ALL.iter().enumerate() {
            assert!(!Code::ALL[i + 1..].iter().any(|b| b.as_str() == a.as_str()));
            assert!(a
                .as_str()
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_'));
            for l in Lang::ALL {
                assert!(a.text(l).trim().len() > 3, "{a:?} in {}", l.code());
            }
        }
    }

    #[test]
    fn an_error_names_its_code_and_the_key_that_governs_it() {
        let e = CapError::with_key(Code::CalibResidual, "calib_max_residual_px", "0.9 px");
        assert_eq!(
            e.to_string(),
            "calib_residual: calib_max_residual_px: 0.9 px"
        );
        assert!(e.text(Lang::Es).contains("[calib_max_residual_px]"));
        assert!(CapError::new(Code::CameraBusy, "x")
            .to_string()
            .starts_with("camera_busy: "));
    }

    #[test]
    fn the_keys_that_errors_name_are_keys_of_the_configuration() {
        for k in crate::capture::budget::KEYS_NAMED
            .iter()
            .chain(crate::capture::health::KEYS_NAMED)
            .chain(crate::capture::settle::KEYS_NAMED)
        {
            assert!(crate::config::KEYS.contains(k), "{k} is not a key");
        }
    }

    #[test]
    fn a_range_allows_only_values_inside_it_on_a_step() {
        let r = PropertyRange {
            min: 0.0,
            max: 10.0,
            step: 2.0,
            default: 4.0,
            auto_supported: false,
        };
        assert!(r.allows(4.0) && r.allows(0.0) && r.allows(10.0));
        assert!(!r.allows(3.0) && !r.allows(12.0) && !r.allows(-2.0) && !r.allows(f64::NAN));
    }

    #[test]
    fn formats_and_properties_are_found_by_their_names() {
        for f in PixelFormat::ALL {
            assert_eq!(PixelFormat::from_name(f.name()), Some(f));
        }
        for p in Property::ALL {
            assert_eq!(Property::from_name(p.name()), Some(p));
        }
        assert_eq!(Property::ALL.len(), 13);
        assert_eq!(PixelFormat::from_name("h264"), None);
    }

    #[test]
    fn a_format_with_no_size_or_no_rate_is_not_usable() {
        let f = |w, h, fps| FormatDesc {
            width: w,
            height: h,
            fps,
            pixel: PixelFormat::Yuyv,
        };
        assert!(f(640, 480, 30).is_usable());
        assert!(
            !f(0, 0, 0).is_usable() && !f(640, 480, 0).is_usable() && !f(0, 480, 30).is_usable()
        );
    }
}
