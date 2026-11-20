//! The budget of a USB bus. Every camera of a bus shares what the bus can carry, so two cameras that work alone can leave one of them black when they share a hub. The load of a stream is its
//! size times its rate times the bits of a pixel; a motion JPEG stream is counted at a number of bits for each pixel that the configuration sets (`capture_mjpeg_bits_px`). When a bus asks for
//! more than `capture_usb_mbps` the answer says how much, and what to try: a smaller picture, a lower rate, motion JPEG instead of raw frames, or another bus.

use super::{CameraId, CapError, Code, FormatDesc, PixelFormat};

/// The keys of the configuration that the errors of this module name.
pub const KEYS_NAMED: &[&str] = &["capture_usb_mbps", "capture_mjpeg_bits_px"];

/// What the budget is made of.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetSettings {
    /// What one bus carries, in megabits a second, for all the cameras on it.
    pub usb_mbps: f64,
    /// Bits for each pixel of a motion JPEG stream.
    pub mjpeg_bits_px: f64,
}

impl Default for BudgetSettings {
    fn default() -> Self {
        BudgetSettings {
            usb_mbps: 320.0,
            mjpeg_bits_px: 2.0,
        }
    }
}

/// Whether the values can be used.
pub fn check_settings(s: &BudgetSettings) -> Result<(), CapError> {
    if !(s.usb_mbps.is_finite() && (10.0..=100_000.0).contains(&s.usb_mbps)) {
        return Err(CapError::with_key(
            Code::UsbBandwidth,
            "capture_usb_mbps",
            "it must be from 10 to 100000 megabits a second",
        ));
    }
    if !(s.mjpeg_bits_px.is_finite() && (0.5..=8.0).contains(&s.mjpeg_bits_px)) {
        return Err(CapError::with_key(
            Code::UsbBandwidth,
            "capture_mjpeg_bits_px",
            "it must be from 0.5 to 8 bits for each pixel",
        ));
    }
    Ok(())
}

/// What a stream asks of the bus, in megabits a second.
pub fn mbps(f: &FormatDesc, s: &BudgetSettings) -> f64 {
    let bits = f.pixel.bits_per_pixel().map_or(s.mjpeg_bits_px, f64::from);
    f64::from(f.width) * f64::from(f.height) * f64::from(f.fps) * bits / 1.0e6
}

/// The bus of a port such as `1-2.3` (bus 1); `?` when the port is not known, so that cameras whose port is not known are counted together, which is the careful way.
pub fn bus_of(port: &str) -> String {
    match port.split_once('-') {
        Some((bus, _)) if !bus.is_empty() => bus.to_string(),
        _ => "?".to_string(),
    }
}

/// What to try when a bus asks for too much.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Advice {
    /// Raw frames are in the load: motion JPEG carries the same picture in a fraction of the bytes.
    PreferMjpeg,
    LowerResolution,
    LowerRate,
    /// More than one camera is on the bus: put one on another bus or another root hub.
    OtherBus,
}

/// The load of one bus.
#[derive(Debug, Clone, PartialEq)]
pub struct BusLoad {
    pub bus: String,
    pub mbps: f64,
    pub streams: usize,
    pub over: bool,
    pub advice: Vec<Advice>,
}

/// The load of each bus for the streams that are asked for, with the advice when it is over the budget.
pub fn load(streams: &[(CameraId, FormatDesc)], s: &BudgetSettings) -> Vec<BusLoad> {
    let mut buses: Vec<BusLoad> = Vec::new();
    let mut raw: Vec<bool> = Vec::new();
    for (id, f) in streams {
        let bus = bus_of(&id.port);
        let at = match buses.iter().position(|b| b.bus == bus) {
            Some(i) => i,
            None => {
                buses.push(BusLoad {
                    bus,
                    mbps: 0.0,
                    streams: 0,
                    over: false,
                    advice: Vec::new(),
                });
                raw.push(false);
                buses.len() - 1
            }
        };
        buses[at].mbps += mbps(f, s);
        buses[at].streams += 1;
        raw[at] |= f.pixel != PixelFormat::Mjpeg;
    }
    for (b, has_raw) in buses.iter_mut().zip(raw) {
        b.over = b.mbps > s.usb_mbps;
        if b.over {
            if has_raw {
                b.advice.push(Advice::PreferMjpeg);
            }
            b.advice.push(Advice::LowerResolution);
            b.advice.push(Advice::LowerRate);
            if b.streams > 1 {
                b.advice.push(Advice::OtherBus);
            }
        }
    }
    buses
}

/// The loads when every bus is inside the budget; the first bus that is not is an error that names the key and says the load.
pub fn check(
    streams: &[(CameraId, FormatDesc)],
    s: &BudgetSettings,
) -> Result<Vec<BusLoad>, CapError> {
    check_settings(s)?;
    let loads = load(streams, s);
    if let Some(b) = loads.iter().find(|b| b.over) {
        return Err(CapError::with_key(
            Code::UsbBandwidth,
            "capture_usb_mbps",
            format!(
                "bus {}: {:.0} Mbit/s asked of {:.0} ({} streams); try {}",
                b.bus,
                b.mbps,
                s.usb_mbps,
                b.streams,
                b.advice
                    .iter()
                    .map(|a| format!("{a:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }
    Ok(loads)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(port: &str) -> CameraId {
        CameraId {
            vendor: 1,
            product: 2,
            serial: String::new(),
            port: port.into(),
        }
    }

    fn f(w: u32, h: u32, fps: u32, pixel: PixelFormat) -> FormatDesc {
        FormatDesc {
            width: w,
            height: h,
            fps,
            pixel,
        }
    }

    #[test]
    fn the_load_of_raw_frames_is_size_times_rate_times_bits() {
        let s = BudgetSettings::default();
        let yuyv = mbps(&f(1920, 1080, 6, PixelFormat::Yuyv), &s);
        assert!((yuyv - 199.0).abs() < 1.0, "{yuyv}");
        assert!(mbps(&f(1920, 1080, 30, PixelFormat::Yuyv), &s) > 900.0);
        let small = mbps(&f(640, 480, 120, PixelFormat::Mjpeg), &s);
        assert!((small - 73.7).abs() < 0.2, "{small}");
    }

    #[test]
    fn two_cameras_of_motion_jpeg_fit_a_bus_and_the_same_two_in_yuyv_do_not() {
        let s = BudgetSettings::default();
        let mjpeg = f(1920, 1080, 30, PixelFormat::Mjpeg);
        assert!(check(&[(id("1-1"), mjpeg), (id("1-2"), mjpeg)], &s).is_ok());
        let yuyv = f(1920, 1080, 30, PixelFormat::Yuyv);
        let e = check(&[(id("1-1"), yuyv), (id("1-2"), yuyv)], &s).unwrap_err();
        assert_eq!(e.code, Code::UsbBandwidth);
        assert_eq!(e.key, Some("capture_usb_mbps"));
        assert!(e.detail.contains("PreferMjpeg") && e.detail.contains("OtherBus"));
    }

    #[test]
    fn cameras_on_another_bus_do_not_share_the_load() {
        let s = BudgetSettings::default();
        let big = f(1920, 1080, 30, PixelFormat::Mjpeg);
        let loads = check(
            &[(id("1-1"), big), (id("2-1"), big), (id("1-3.2"), big)],
            &s,
        );
        let loads = loads.unwrap();
        assert_eq!(loads.len(), 2);
        assert_eq!(loads.iter().find(|b| b.bus == "1").unwrap().streams, 2);
        assert_eq!(loads.iter().find(|b| b.bus == "2").unwrap().streams, 1);
    }

    #[test]
    fn cameras_whose_port_is_not_known_are_counted_together() {
        assert_eq!(bus_of(""), "?");
        assert_eq!(bus_of("csi0"), "?");
        assert_eq!(bus_of("3-4.1.2"), "3");
        let s = BudgetSettings::default();
        let big = f(1920, 1080, 30, PixelFormat::Yuyv);
        assert!(check(&[(id(""), big)], &s).is_err());
        let mid = f(1280, 720, 10, PixelFormat::Yuyv);
        let two = check(&[(id(""), mid), (id("csi0"), mid)], &s).unwrap();
        assert_eq!(two.len(), 1);
        assert_eq!(two[0].streams, 2);
    }

    #[test]
    fn a_budget_that_is_not_a_number_is_refused_and_names_its_key() {
        for bad in [f64::NAN, 0.0, 5.0, 1.0e9] {
            let s = BudgetSettings {
                usb_mbps: bad,
                ..BudgetSettings::default()
            };
            assert_eq!(
                check_settings(&s).unwrap_err().key,
                Some("capture_usb_mbps")
            );
        }
        let s = BudgetSettings {
            mjpeg_bits_px: 0.1,
            ..BudgetSettings::default()
        };
        assert_eq!(
            check(&[], &s).unwrap_err().key,
            Some("capture_mjpeg_bits_px")
        );
    }

    #[test]
    fn a_load_exactly_at_the_budget_is_allowed() {
        let s = BudgetSettings {
            usb_mbps: 100.0,
            mjpeg_bits_px: 2.0,
        };
        let at = f(1000, 500, 100, PixelFormat::Mjpeg);
        assert!((mbps(&at, &s) - 100.0).abs() < 1e-9);
        assert!(check(&[(id("1-1"), at)], &s).is_ok());
        let over = f(1000, 501, 100, PixelFormat::Mjpeg);
        assert!(check(&[(id("1-1"), over)], &s).is_err());
    }
}
