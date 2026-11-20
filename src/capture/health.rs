//! Whether the stream of a camera can be trusted. A camera on a busy bus goes black, a camera whose driver hangs repeats the last frame, a cable that fails cuts frames or loses them, and a
//! light that is fed by a rectifier flickers. `Monitor` looks at each frame (or at the error that took its place) and says what is wrong and what is the probable cause; nothing here moves a
//! machine, it only tells the callers whether the pictures are worth measuring.

use super::{CapError, Code, Frame};

/// The keys of the configuration that the errors of this module name.
pub const KEYS_NAMED: &[&str] = &[
    "capture_black_mean",
    "capture_frozen_frames",
    "capture_flicker_max",
    "capture_powerline_hz",
];

/// What is judged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HealthSettings {
    /// A frame whose mean brightness is below this (grey levels, 0 to 255) is black.
    pub black_mean: f64,
    /// This many frames with exactly the same pixels in a row are a frozen stream.
    pub frozen_frames: usize,
    /// The depth of the flicker above which the light flickers: the difference between the brightest and the darkest frame over their sum (0 to 1).
    pub flicker_max: f64,
    /// The frequency of the mains, 50 or 60.
    pub powerline_hz: u32,
}

impl Default for HealthSettings {
    fn default() -> Self {
        HealthSettings {
            black_mean: 4.0,
            frozen_frames: 5,
            flicker_max: 0.02,
            powerline_hz: 50,
        }
    }
}

/// Whether the values can be used.
pub fn check(s: &HealthSettings) -> Result<(), CapError> {
    let bad = |key: &'static str, why: &str| Err(CapError::with_key(Code::StreamBlack, key, why));
    if !(s.black_mean.is_finite() && (0.0..=255.0).contains(&s.black_mean)) {
        return bad("capture_black_mean", "it must be from 0 to 255 grey levels");
    }
    if !(2..=1000).contains(&s.frozen_frames) {
        return bad("capture_frozen_frames", "it must be from 2 to 1000 frames");
    }
    if !(s.flicker_max.is_finite() && s.flicker_max > 0.0 && s.flicker_max <= 1.0) {
        return bad(
            "capture_flicker_max",
            "it must be more than 0 and at most 1",
        );
    }
    if s.powerline_hz != 50 && s.powerline_hz != 60 {
        return bad("capture_powerline_hz", "it must be 50 or 60");
    }
    Ok(())
}

/// What is wrong with the stream.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Finding {
    /// The frame is black; this many in a row, this one included.
    Black { run: usize },
    /// The frame is the same as this many before it.
    Frozen { repeats: usize },
    /// The frame came cut or with the wrong size.
    Damaged,
    /// This many frames were lost between the last one and this one.
    Lost { count: u64 },
    /// The brightness goes up and down by this depth from frame to frame.
    Flicker { depth: f64 },
}

impl Finding {
    /// The code of the error that goes with it.
    pub fn code(&self) -> Code {
        match self {
            Finding::Black { .. } => Code::StreamBlack,
            Finding::Frozen { .. } => Code::StreamFrozen,
            Finding::Damaged => Code::FrameTruncated,
            Finding::Lost { .. } => Code::NoNewFrames,
            Finding::Flicker { .. } => Code::StreamBlack,
        }
    }
}

/// What most probably causes a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// Another camera on the same bus takes the bandwidth.
    UsbBandwidth,
    /// The camera is covered, the light is off or the exposure is too short.
    NoLight,
    /// The driver or the camera hangs: unplug it and plug it again.
    DriverHang,
    /// The cable or the hub loses data.
    CableOrHub,
    /// The light is fed by a source that is not steady.
    MainsFlicker,
}

/// The probable cause of a finding; `bus_shared` says whether another camera is on the bus of this one.
pub fn probable_cause(f: &Finding, bus_shared: bool) -> Cause {
    match f {
        Finding::Black { .. } | Finding::Lost { .. } if bus_shared => Cause::UsbBandwidth,
        Finding::Black { .. } => Cause::NoLight,
        Finding::Frozen { .. } => Cause::DriverHang,
        Finding::Damaged | Finding::Lost { .. } => Cause::CableOrHub,
        Finding::Flicker { .. } => Cause::MainsFlicker,
    }
}

/// A hash of the pixels (FNV-1a), to tell that two frames are the same.
pub fn fingerprint(data: &[u8]) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for &b in data {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// The mean brightness of a frame.
pub fn mean(frame: &Frame) -> f64 {
    if frame.luma.data.is_empty() {
        return 0.0;
    }
    frame.luma.data.iter().map(|&v| f64::from(v)).sum::<f64>() / frame.luma.data.len() as f64
}

/// The monitor of one stream.
#[derive(Debug, Clone)]
pub struct Monitor {
    settings: HealthSettings,
    last_seq: Option<u64>,
    last_print: Option<u64>,
    same: usize,
    black_run: usize,
    /// The frames seen, the lost ones and the damaged ones.
    pub seen: u64,
    pub lost: u64,
    pub damaged: u64,
}

impl Monitor {
    /// A monitor for the settings, which must be usable.
    pub fn new(settings: HealthSettings) -> Result<Monitor, CapError> {
        check(&settings)?;
        Ok(Monitor {
            settings,
            last_seq: None,
            last_print: None,
            same: 0,
            black_run: 0,
            seen: 0,
            lost: 0,
            damaged: 0,
        })
    }

    /// Looks at a frame that arrived.
    pub fn observe(&mut self, frame: &Frame) -> Vec<Finding> {
        let mut out = Vec::new();
        self.seen += 1;
        if let Some(last) = self.last_seq {
            if frame.seq > last + 1 {
                let count = frame.seq - last - 1;
                self.lost += count;
                out.push(Finding::Lost { count });
            }
        }
        self.last_seq = Some(frame.seq.max(self.last_seq.unwrap_or(0)));
        if mean(frame) < self.settings.black_mean {
            self.black_run += 1;
            out.push(Finding::Black {
                run: self.black_run,
            });
        } else {
            self.black_run = 0;
        }
        let print = fingerprint(&frame.luma.data);
        if self.last_print == Some(print) {
            self.same += 1;
            if self.same + 1 >= self.settings.frozen_frames {
                out.push(Finding::Frozen {
                    repeats: self.same + 1,
                });
            }
        } else {
            self.same = 0;
        }
        self.last_print = Some(print);
        out
    }

    /// Looks at the error that took the place of a frame. A damaged frame is a finding; a camera that is gone is not (it is an error to be handled where it was asked for).
    pub fn observe_error(&mut self, e: &CapError) -> Option<Finding> {
        match e.code {
            Code::FrameTruncated | Code::FrameBadSize => {
                self.damaged += 1;
                Some(Finding::Damaged)
            }
            _ => None,
        }
    }

    /// Whether the brightness of the frames goes up and down: the depth of the flicker of the series of means, when it is above the limit.
    pub fn flicker(&self, means: &[f64]) -> Option<Finding> {
        let depth = flicker_depth(means)?;
        (depth > self.settings.flicker_max).then_some(Finding::Flicker { depth })
    }
}

/// The depth of the flicker of a series of mean brightness values: (brightest - darkest) / (brightest + darkest). It needs at least 8 values, all finite; `None` otherwise or when the
/// series is dark.
pub fn flicker_depth(means: &[f64]) -> Option<f64> {
    if means.len() < 8 || means.iter().any(|m| !m.is_finite()) {
        return None;
    }
    let hi = means.iter().copied().fold(f64::MIN, f64::max);
    let lo = means.iter().copied().fold(f64::MAX, f64::min);
    (hi + lo > 1e-9).then(|| (hi - lo) / (hi + lo))
}

/// The exposure, in microseconds, nearest to `wanted_us` that is a whole number of half periods of the mains (10 ms at 50 Hz, 8.33 ms at 60 Hz), and not shorter than one: an exposure of
/// that length lights every frame by the same amount whatever the phase of the mains. `hz` must be 50 or 60.
pub fn flicker_free_exposure_us(hz: u32, wanted_us: u32) -> Result<u32, CapError> {
    if hz != 50 && hz != 60 {
        return Err(CapError::with_key(
            Code::StreamBlack,
            "capture_powerline_hz",
            "it must be 50 or 60",
        ));
    }
    let half = 500_000.0 / f64::from(hz);
    let n = (f64::from(wanted_us) / half).round().max(1.0);
    Ok((n * half).round() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::PixelFormat;
    use crate::image::Gray;

    fn frame(seq: u64, value: u8, salt: u8) -> Frame {
        let mut data = vec![value; 64];
        data[0] = salt;
        Frame {
            seq,
            time_us: seq * 1000,
            luma: Gray::new(8, 8, data).unwrap(),
            raw: Vec::new(),
            raw_format: PixelFormat::Gray8,
        }
    }

    fn monitor() -> Monitor {
        Monitor::new(HealthSettings::default()).unwrap()
    }

    #[test]
    fn a_healthy_stream_has_no_finding() {
        let mut m = monitor();
        for i in 1..=20 {
            assert!(m.observe(&frame(i, 120, i as u8)).is_empty(), "frame {i}");
        }
        assert_eq!((m.seen, m.lost, m.damaged), (20, 0, 0));
    }

    #[test]
    fn black_frames_are_found_and_counted_in_a_run() {
        let mut m = monitor();
        assert!(m.observe(&frame(1, 100, 1)).is_empty());
        assert_eq!(m.observe(&frame(2, 0, 2)), vec![Finding::Black { run: 1 }]);
        assert_eq!(m.observe(&frame(3, 1, 3)), vec![Finding::Black { run: 2 }]);
        assert!(m.observe(&frame(4, 90, 4)).is_empty());
        assert_eq!(m.observe(&frame(5, 0, 5)), vec![Finding::Black { run: 1 }]);
    }

    #[test]
    fn a_frame_that_repeats_is_frozen_only_after_the_number_of_frames_asked() {
        let mut m = monitor();
        let mut found = Vec::new();
        for i in 1..=8 {
            found.push(m.observe(&frame(i, 100, 7)));
        }
        assert!(found[..4].iter().all(|f| f.is_empty()));
        assert_eq!(found[4], vec![Finding::Frozen { repeats: 5 }]);
        assert_eq!(found[7], vec![Finding::Frozen { repeats: 8 }]);
        assert!(m.observe(&frame(9, 100, 8)).is_empty());
    }

    #[test]
    fn a_gap_in_the_counter_is_lost_frames() {
        let mut m = monitor();
        m.observe(&frame(1, 100, 1));
        assert_eq!(
            m.observe(&frame(5, 100, 2)),
            vec![Finding::Lost { count: 3 }]
        );
        assert_eq!(m.lost, 3);
        assert!(m.observe(&frame(6, 100, 3)).is_empty());
        // a counter that goes back is not a loss
        assert!(m.observe(&frame(2, 100, 4)).is_empty());
    }

    #[test]
    fn a_damaged_frame_is_a_finding_and_a_camera_that_is_gone_is_not() {
        let mut m = monitor();
        let cut = CapError::new(Code::FrameTruncated, "x");
        assert_eq!(m.observe_error(&cut), Some(Finding::Damaged));
        let size = CapError::new(Code::FrameBadSize, "x");
        assert_eq!(m.observe_error(&size), Some(Finding::Damaged));
        assert_eq!(
            m.observe_error(&CapError::new(Code::CameraDisconnected, "x")),
            None
        );
        assert_eq!(m.damaged, 2);
    }

    #[test]
    fn the_probable_cause_depends_on_whether_the_bus_is_shared() {
        let black = Finding::Black { run: 3 };
        assert_eq!(probable_cause(&black, true), Cause::UsbBandwidth);
        assert_eq!(probable_cause(&black, false), Cause::NoLight);
        assert_eq!(
            probable_cause(&Finding::Frozen { repeats: 9 }, true),
            Cause::DriverHang
        );
        assert_eq!(
            probable_cause(&Finding::Lost { count: 2 }, false),
            Cause::CableOrHub
        );
        assert_eq!(
            probable_cause(&Finding::Lost { count: 2 }, true),
            Cause::UsbBandwidth
        );
        assert_eq!(probable_cause(&Finding::Damaged, true), Cause::CableOrHub);
        assert_eq!(
            probable_cause(&Finding::Flicker { depth: 0.3 }, false),
            Cause::MainsFlicker
        );
    }

    #[test]
    fn the_flicker_is_the_depth_of_the_series_and_needs_enough_frames() {
        let steady = vec![100.0; 16];
        assert_eq!(flicker_depth(&steady), Some(0.0));
        let m = monitor();
        assert!(m.flicker(&steady).is_none());
        let wave: Vec<f64> = (0..16)
            .map(|i| if i % 2 == 0 { 110.0 } else { 90.0 })
            .collect();
        let depth = flicker_depth(&wave).unwrap();
        assert!((depth - 0.1).abs() < 1e-9);
        assert!(matches!(m.flicker(&wave), Some(Finding::Flicker { .. })));
        assert_eq!(flicker_depth(&wave[..7]), None);
        let mut nan = wave.clone();
        nan[3] = f64::NAN;
        assert_eq!(flicker_depth(&nan), None);
        assert_eq!(flicker_depth(&[0.0; 10]), None);
    }

    #[test]
    fn the_exposure_that_does_not_flicker_is_a_whole_number_of_half_periods() {
        assert_eq!(flicker_free_exposure_us(50, 10_000).unwrap(), 10_000);
        assert_eq!(flicker_free_exposure_us(50, 24_000).unwrap(), 20_000);
        assert_eq!(flicker_free_exposure_us(50, 1_000).unwrap(), 10_000);
        assert_eq!(flicker_free_exposure_us(60, 17_000).unwrap(), 16_667);
        let e = flicker_free_exposure_us(55, 10_000).unwrap_err();
        assert_eq!(e.key, Some("capture_powerline_hz"));
    }

    #[test]
    fn settings_out_of_range_are_refused_and_name_their_key() {
        let base = HealthSettings::default();
        let cases = [
            (
                HealthSettings {
                    black_mean: -1.0,
                    ..base
                },
                "capture_black_mean",
            ),
            (
                HealthSettings {
                    black_mean: f64::NAN,
                    ..base
                },
                "capture_black_mean",
            ),
            (
                HealthSettings {
                    frozen_frames: 1,
                    ..base
                },
                "capture_frozen_frames",
            ),
            (
                HealthSettings {
                    frozen_frames: 1001,
                    ..base
                },
                "capture_frozen_frames",
            ),
            (
                HealthSettings {
                    flicker_max: 0.0,
                    ..base
                },
                "capture_flicker_max",
            ),
            (
                HealthSettings {
                    powerline_hz: 49,
                    ..base
                },
                "capture_powerline_hz",
            ),
        ];
        for (s, key) in cases {
            assert_eq!(Monitor::new(s).unwrap_err().key, Some(key));
        }
    }

    #[test]
    fn the_fingerprint_tells_two_frames_apart() {
        assert_eq!(fingerprint(&[1, 2, 3]), fingerprint(&[1, 2, 3]));
        assert_ne!(fingerprint(&[1, 2, 3]), fingerprint(&[1, 2, 4]));
        assert_ne!(fingerprint(&[]), fingerprint(&[0]));
    }
}
