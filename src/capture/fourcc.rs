//! The four letters that V4L2, libcamera and Media Foundation use for the way the bytes of a frame are arranged, and the clean-up of the lists of formats that a backend reads: a format with a
//! size of 0 or a rate of 0 is not a format (a CSI camera seen through V4L2 lists such ones), and a format that is listed twice is listed once.

use super::{FormatDesc, PixelFormat};

/// The four letters as a number, first letter lowest, as V4L2 writes them.
pub const fn code(letters: &[u8; 4]) -> u32 {
    (letters[0] as u32)
        | ((letters[1] as u32) << 8)
        | ((letters[2] as u32) << 16)
        | ((letters[3] as u32) << 24)
}

/// The four letters of a number.
pub fn letters(code: u32) -> [u8; 4] {
    code.to_le_bytes()
}

/// The format that a code names, when the program can read it.
pub fn format_from_code(c: u32) -> Option<PixelFormat> {
    match &letters(c) {
        b"YUYV" | b"YUY2" => Some(PixelFormat::Yuyv),
        b"MJPG" | b"JPEG" => Some(PixelFormat::Mjpeg),
        b"RGB3" | b"RGB\0" => Some(PixelFormat::Rgb24),
        b"GREY" | b"Y800" => Some(PixelFormat::Gray8),
        _ => None,
    }
}

/// The code that V4L2 uses for a format.
pub fn code_of(f: PixelFormat) -> u32 {
    match f {
        PixelFormat::Yuyv => code(b"YUYV"),
        PixelFormat::Mjpeg => code(b"MJPG"),
        PixelFormat::Rgb24 => code(b"RGB3"),
        PixelFormat::Gray8 => code(b"GREY"),
    }
}

/// The format of a Media Foundation subtype: a GUID whose first four bytes are the four letters and whose other twelve are the fixed ones of the video subtypes.
pub fn format_from_mf_subtype(guid: &[u8; 16]) -> Option<PixelFormat> {
    const TAIL: [u8; 12] = [
        0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
    ];
    if guid[4..] != TAIL {
        return None;
    }
    format_from_code(u32::from_le_bytes([guid[0], guid[1], guid[2], guid[3]]))
}

/// The port of a camera from the `bus_info` that V4L2 gives (`usb-0000:00:14.0-1.2`): the controller and the ports, as `0000_00_14.0-1.2`, in the form of a port of an identifier (no
/// colon). Whatever does not start with `usb-` or has no ports after the controller is kept as it is, with the colons and spaces turned into `_`, and the text is at most 64 characters.
pub fn port_from_bus_info(bus_info: &str) -> String {
    let rest = bus_info.strip_prefix("usb-").unwrap_or(bus_info);
    rest.chars()
        .map(|c| {
            if c.is_ascii_graphic() && c != ':' && c != '@' {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect()
}

/// The formats that can be used, once each and in the order they were listed.
pub fn clean(formats: &[FormatDesc]) -> Vec<FormatDesc> {
    let mut out: Vec<FormatDesc> = Vec::new();
    for f in formats {
        if f.is_usable() && !out.contains(f) {
            out.push(*f);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_letters_are_the_numbers_that_v4l2_uses() {
        assert_eq!(code(b"YUYV"), 0x5659_5559);
        assert_eq!(code(b"MJPG"), 0x4750_4A4D);
        assert_eq!(letters(code(b"GREY")), *b"GREY");
    }

    #[test]
    fn every_format_goes_to_its_code_and_back() {
        for f in PixelFormat::ALL {
            assert_eq!(format_from_code(code_of(f)), Some(f));
        }
        assert_eq!(format_from_code(code(b"YUY2")), Some(PixelFormat::Yuyv));
        assert_eq!(format_from_code(code(b"NV12")), None);
        assert_eq!(format_from_code(0), None);
    }

    #[test]
    fn a_media_foundation_subtype_is_read_only_with_its_fixed_tail() {
        let mut g = [0u8; 16];
        g[..4].copy_from_slice(b"YUY2");
        g[4..].copy_from_slice(&[
            0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
        ]);
        assert_eq!(format_from_mf_subtype(&g), Some(PixelFormat::Yuyv));
        g[10] ^= 1;
        assert_eq!(format_from_mf_subtype(&g), None);
    }

    #[test]
    fn the_port_of_a_usb_camera_is_the_controller_and_the_ports_without_colons() {
        assert_eq!(
            port_from_bus_info("usb-0000:00:14.0-1.2"),
            "0000_00_14.0-1.2"
        );
        assert_eq!(
            super::super::budget::bus_of(&port_from_bus_info("usb-0000:00:14.0-1.2")),
            "0000_00_14.0"
        );
        assert_eq!(port_from_bus_info("platform:csi"), "platform_csi");
        assert_eq!(port_from_bus_info("a b@c"), "a_b_c");
        assert_eq!(port_from_bus_info(&"x".repeat(100)).len(), 64);
        assert_eq!(port_from_bus_info(""), "");
        let id = super::super::CameraId {
            vendor: 1,
            product: 2,
            serial: String::new(),
            port: port_from_bus_info("usb-0000:00:14.0-1.2"),
        };
        assert_eq!(super::super::CameraId::parse(&id.key()).unwrap(), id);
    }

    #[test]
    fn formats_with_no_size_or_no_rate_are_dropped_and_repeats_are_listed_once() {
        let f = |w, h, fps| FormatDesc {
            width: w,
            height: h,
            fps,
            pixel: PixelFormat::Mjpeg,
        };
        let list = [
            f(0, 0, 0),
            f(640, 480, 30),
            f(1280, 720, 0),
            f(640, 480, 30),
            f(1280, 720, 10),
        ];
        assert_eq!(clean(&list), vec![f(640, 480, 30), f(1280, 720, 10)]);
        assert!(clean(&[f(0, 0, 0)]).is_empty());
    }
}
