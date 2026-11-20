//! Which camera, which format and which settings of the camera the configuration asks for. The backend is named (`auto`, `sim`, `v4l2`, `mfoundation`, `libcamera`), the camera by its
//! identifier (`capture_camera`, not by its number), the format as `640x480@30:yuyv`, and a profile of the camera as words such as `exposure=120 gain=10 focus=auto`: the profile is applied
//! property by property and the first property that the camera refuses stops it with the name of the property, so a profile is never half applied without being said.

use super::{Camera, CameraInfo, CapError, Code, FormatDesc, PixelFormat, Property, PropertyValue};

/// The names of the backends that the configuration may ask for.
pub const BACKENDS: [&str; 5] = ["auto", "sim", "v4l2", "mfoundation", "libcamera"];

/// Whether `name` is a backend that can be asked for.
pub fn check_backend(name: &str) -> Result<(), CapError> {
    if BACKENDS.contains(&name) {
        Ok(())
    } else {
        Err(CapError::with_key(
            Code::BackendUnavailable,
            "capture_backend",
            format!("{name:?} is not one of {}", BACKENDS.join(", ")),
        ))
    }
}

/// The format written as `WIDTHxHEIGHT@FPS:PIXEL`, or `None` for the empty text (the camera chooses).
pub fn parse_format(text: &str) -> Result<Option<FormatDesc>, CapError> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let bad = |why: &str| {
        CapError::with_key(
            Code::FormatInvalid,
            "capture_format",
            format!("{text:?}: {why}"),
        )
    };
    let (size, rest) = text
        .split_once('@')
        .ok_or_else(|| bad("it must be WIDTHxHEIGHT@FPS:PIXEL"))?;
    let (fps, pixel) = rest
        .split_once(':')
        .ok_or_else(|| bad("it must be WIDTHxHEIGHT@FPS:PIXEL"))?;
    let (w, h) = size
        .split_once('x')
        .ok_or_else(|| bad("the size is WIDTHxHEIGHT"))?;
    let num = |s: &str| s.parse::<u32>().ok().filter(|&v| v > 0 && v <= 20_000);
    let (Some(width), Some(height), Some(fps)) = (num(w), num(h), num(fps).filter(|&v| v <= 1000))
    else {
        return Err(bad("the size is from 1 to 20000, the rate from 1 to 1000"));
    };
    let pixel = PixelFormat::from_name(pixel)
        .ok_or_else(|| bad("the pixel format is yuyv, mjpeg, rgb24 or gray8"))?;
    Ok(Some(FormatDesc {
        width,
        height,
        fps,
        pixel,
    }))
}

/// The format written as text, the way `parse_format` reads it.
pub fn format_text(f: &FormatDesc) -> String {
    format!("{}x{}@{}:{}", f.width, f.height, f.fps, f.pixel.name())
}

/// The properties of a profile written as `name=value` or `name=auto` separated by spaces; a property twice is an error.
pub fn parse_profile(text: &str) -> Result<Vec<(Property, PropertyValue)>, CapError> {
    let mut out: Vec<(Property, PropertyValue)> = Vec::new();
    for word in text.split_whitespace() {
        let bad = |code, why: String| {
            CapError::with_key(code, "capture_profile", format!("{word:?}: {why}"))
        };
        let (name, value) = word
            .split_once('=')
            .ok_or_else(|| bad(Code::PropertyUnsupported, "it must be name=value".into()))?;
        let p = Property::from_name(name).ok_or_else(|| {
            bad(
                Code::PropertyUnsupported,
                "that is not a property of a camera".into(),
            )
        })?;
        if out.iter().any(|(q, _)| *q == p) {
            return Err(bad(
                Code::PropertyOutOfRange,
                "the property is written twice".into(),
            ));
        }
        let v = if value == "auto" {
            PropertyValue::auto()
        } else {
            let n = value
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| bad(Code::PropertyOutOfRange, "the value is not a number".into()))?;
            PropertyValue::fixed(n)
        };
        out.push((p, v));
    }
    Ok(out)
}

/// Applies a profile to a camera property by property. The first property that the camera refuses stops it, and the error says which and how many were already set.
pub fn apply_profile(
    cam: &mut dyn Camera,
    profile: &[(Property, PropertyValue)],
) -> Result<usize, CapError> {
    for (done, (p, v)) in profile.iter().enumerate() {
        cam.set(*p, *v).map_err(|e| {
            CapError::with_key(
                e.code,
                "capture_profile",
                format!(
                    "{}: {} ({done} properties were set before it)",
                    p.name(),
                    e.detail
                ),
            )
        })?;
    }
    Ok(profile.len())
}

/// The format to open: the one asked for when the camera lists it, else the usable one with the most pixels at a rate of 15 or more (the usable one with the highest rate when none reaches
/// 15). A format that the camera does not list is `format_unsupported`, never replaced by another without being asked.
pub fn choose_format(
    info: &CameraInfo,
    wanted: Option<FormatDesc>,
) -> Result<FormatDesc, CapError> {
    let usable = info.usable_formats();
    if let Some(w) = wanted {
        return usable.contains(&w).then_some(w).ok_or_else(|| {
            CapError::with_key(
                Code::FormatUnsupported,
                "capture_format",
                format!("{} is not listed by {}", format_text(&w), info.name),
            )
        });
    }
    let fast: Vec<&FormatDesc> = usable.iter().filter(|f| f.fps >= 15).collect();
    let pick = if fast.is_empty() {
        usable
            .iter()
            .max_by_key(|f| (f.fps, u64::from(f.width) * u64::from(f.height)))
    } else {
        fast.into_iter()
            .max_by_key(|f| (u64::from(f.width) * u64::from(f.height), f.fps))
    };
    pick.copied().ok_or_else(|| {
        CapError::with_key(
            Code::FormatInvalid,
            "capture_format",
            format!("{} lists no format with a size and a rate", info.name),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::sim::{home, SimBackend, Spec, World};
    use crate::capture::{CameraId, CaptureBackend};

    #[test]
    fn a_format_is_written_and_read_back() {
        for text in [
            "640x480@30:yuyv",
            "1920x1080@5:mjpeg",
            "1x1@1:gray8",
            "320x240@120:rgb24",
        ] {
            let f = parse_format(text).unwrap().unwrap();
            assert_eq!(format_text(&f), text);
        }
        assert_eq!(parse_format("").unwrap(), None);
        assert_eq!(parse_format("  ").unwrap(), None);
    }

    #[test]
    fn a_format_with_a_mistake_is_refused_and_names_the_key() {
        for bad in [
            "640x480",
            "640x480@30",
            "640@30:yuyv",
            "0x480@30:yuyv",
            "640x480@0:yuyv",
            "640x480@1001:yuyv",
            "640x480@30:h264",
            "axb@30:yuyv",
            "99999x480@30:yuyv",
            "-1x480@30:yuyv",
        ] {
            let e = parse_format(bad).unwrap_err();
            assert_eq!(e.code, Code::FormatInvalid, "{bad}");
            assert_eq!(e.key, Some("capture_format"));
        }
    }

    #[test]
    fn a_profile_is_read_in_the_order_it_is_written() {
        let p = parse_profile("exposure=120 gain=10  focus=auto").unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(p[0], (Property::Exposure, PropertyValue::fixed(120.0)));
        assert_eq!(p[2], (Property::Focus, PropertyValue::auto()));
        assert!(parse_profile("").unwrap().is_empty());
    }

    #[test]
    fn a_profile_with_a_mistake_is_refused_and_names_the_key() {
        for bad in [
            "exposure",
            "colour=3",
            "exposure=abc",
            "exposure=nan",
            "gain=1 gain=2",
            "=5",
        ] {
            let e = parse_profile(bad).unwrap_err();
            assert_eq!(e.key, Some("capture_profile"), "{bad}");
        }
    }

    fn open() -> Box<dyn Camera> {
        let mut b = SimBackend::new(1, World::board(home()).shared());
        let spec = Spec::standard(1, "1-1");
        let id: CameraId = spec.id.clone();
        b.add(spec);
        b.open(
            &id,
            FormatDesc {
                width: 480,
                height: 320,
                fps: 30,
                pixel: PixelFormat::Gray8,
            },
        )
        .unwrap()
    }

    #[test]
    fn a_profile_is_applied_to_the_camera_and_stops_at_the_first_property_it_refuses() {
        let mut cam = open();
        let p = parse_profile("exposure=150 gain=20 focus=auto").unwrap();
        assert_eq!(apply_profile(cam.as_mut(), &p).unwrap(), 3);
        assert_eq!(cam.get(Property::Exposure).unwrap().value, 150.0);
        assert!(cam.get(Property::Focus).unwrap().auto);
        let q = parse_profile("gain=30 hue=3 exposure=200").unwrap();
        let e = apply_profile(cam.as_mut(), &q).unwrap_err();
        assert_eq!(e.code, Code::PropertyUnsupported);
        assert!(
            e.detail.contains("hue") && e.detail.contains("1 properties were set"),
            "{e}"
        );
        assert_eq!(cam.get(Property::Gain).unwrap().value, 30.0);
        assert_eq!(
            cam.get(Property::Exposure).unwrap().value,
            150.0,
            "nothing after the refusal was set"
        );
        let r = parse_profile("gain=500").unwrap();
        assert_eq!(
            apply_profile(cam.as_mut(), &r).unwrap_err().code,
            Code::PropertyOutOfRange
        );
    }

    #[test]
    fn the_format_asked_for_is_used_only_if_the_camera_lists_it() {
        let mut b = SimBackend::new(1, World::board(home()).shared());
        b.add(Spec::standard(1, "1-1"));
        let info = b.enumerate().unwrap().remove(0);
        let ask = parse_format("640x480@30:mjpeg").unwrap();
        assert_eq!(choose_format(&info, ask).unwrap().pixel, PixelFormat::Mjpeg);
        let missing = parse_format("640x480@60:mjpeg").unwrap();
        let e = choose_format(&info, missing).unwrap_err();
        assert_eq!(e.code, Code::FormatUnsupported);
        assert_eq!(e.key, Some("capture_format"));
        let zero = FormatDesc {
            width: 0,
            height: 0,
            fps: 0,
            pixel: PixelFormat::Yuyv,
        };
        assert!(choose_format(&info, Some(zero)).is_err());
    }

    #[test]
    fn with_no_format_asked_the_biggest_usable_one_at_a_good_rate_is_chosen() {
        let f = |w, h, fps| FormatDesc {
            width: w,
            height: h,
            fps,
            pixel: PixelFormat::Mjpeg,
        };
        let info = |formats| CameraInfo {
            name: "c".into(),
            id: CameraId {
                vendor: 1,
                product: 1,
                serial: String::new(),
                port: String::new(),
            },
            formats,
        };
        let a = info(vec![
            f(0, 0, 0),
            f(1920, 1080, 5),
            f(1280, 720, 30),
            f(640, 480, 60),
        ]);
        assert_eq!(choose_format(&a, None).unwrap(), f(1280, 720, 30));
        let slow = info(vec![f(1920, 1080, 5), f(640, 480, 10)]);
        assert_eq!(choose_format(&slow, None).unwrap(), f(640, 480, 10));
        let none = info(vec![f(0, 0, 0)]);
        assert_eq!(
            choose_format(&none, None).unwrap_err().code,
            Code::FormatInvalid
        );
    }

    #[test]
    fn the_backends_that_can_be_named_are_the_five() {
        for b in BACKENDS {
            assert!(check_backend(b).is_ok());
        }
        let e = check_backend("directshow").unwrap_err();
        assert_eq!(e.key, Some("capture_backend"));
    }
}
