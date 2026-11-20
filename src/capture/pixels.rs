//! The readers of the bytes of a frame: packed YUV (YUYV), three bytes of colour, one byte of grey, and the header of a motion JPEG frame. Every reader checks the length before it looks at a
//! byte, so a frame that is cut, too long or of an odd width is refused with a code and never read past its end. Motion JPEG is read as far as its header and its ending (the size, whether it
//! is cut); the pictures of the simulated camera carry their pixels in a stored form that this module also reads, and a real JPEG whose pixels need the decoder is `frame_not_decoded`.

use super::{CapError, Code};
use crate::image::Gray;

fn expect_len(bytes: &[u8], w: usize, h: usize, per_pixel_x2: usize) -> Result<(), CapError> {
    if w == 0 || h == 0 {
        return Err(CapError::new(Code::FrameBadSize, "the size is 0"));
    }
    let want = w
        .checked_mul(h)
        .and_then(|p| p.checked_mul(per_pixel_x2))
        .map(|v| v / 2)
        .ok_or_else(|| CapError::new(Code::FrameBadSize, "the size is too large"))?;
    if bytes.len() < want {
        return Err(CapError::new(
            Code::FrameTruncated,
            format!("{} bytes of {want}", bytes.len()),
        ));
    }
    if bytes.len() > want {
        return Err(CapError::new(
            Code::FrameBadSize,
            format!("{} bytes where {want} are expected", bytes.len()),
        ));
    }
    Ok(())
}

fn gray(w: usize, h: usize, data: Vec<u8>) -> Result<Gray, CapError> {
    Gray::new(w, h, data).map_err(|e| CapError::new(Code::FrameBadSize, e))
}

/// The brightness of a frame of YUYV (Y0 U Y1 V): the Y of each pixel. The width must be even.
pub fn yuyv_to_luma(bytes: &[u8], w: usize, h: usize) -> Result<Gray, CapError> {
    if !w.is_multiple_of(2) {
        return Err(CapError::new(
            Code::FrameBadSize,
            "YUYV needs an even width",
        ));
    }
    expect_len(bytes, w, h, 4)?;
    gray(w, h, bytes.iter().step_by(2).copied().collect())
}

/// The colour of a frame of YUYV as red, green and blue bytes (BT.601, full range).
pub fn yuyv_to_rgb(bytes: &[u8], w: usize, h: usize) -> Result<Vec<u8>, CapError> {
    if !w.is_multiple_of(2) {
        return Err(CapError::new(
            Code::FrameBadSize,
            "YUYV needs an even width",
        ));
    }
    expect_len(bytes, w, h, 4)?;
    let mut out = Vec::with_capacity(w * h * 3);
    for quad in bytes.as_chunks::<4>().0 {
        let (u, v) = (f64::from(quad[1]) - 128.0, f64::from(quad[3]) - 128.0);
        for y in [quad[0], quad[2]] {
            let y = f64::from(y);
            let clip = |x: f64| x.round().clamp(0.0, 255.0) as u8;
            out.push(clip(y + 1.402 * v));
            out.push(clip(y - 0.344_136 * u - 0.714_136 * v));
            out.push(clip(y + 1.772 * u));
        }
    }
    Ok(out)
}

/// The brightness of a frame of red, green and blue bytes (BT.601).
pub fn rgb24_to_luma(bytes: &[u8], w: usize, h: usize) -> Result<Gray, CapError> {
    expect_len(bytes, w, h, 6)?;
    let data = bytes
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| {
            ((u32::from(p[0]) * 299 + u32::from(p[1]) * 587 + u32::from(p[2]) * 114 + 500) / 1000)
                as u8
        })
        .collect();
    gray(w, h, data)
}

/// A frame of one byte for each pixel.
pub fn gray8_to_luma(bytes: &[u8], w: usize, h: usize) -> Result<Gray, CapError> {
    expect_len(bytes, w, h, 2)?;
    gray(w, h, bytes.to_vec())
}

/// What the header of a motion JPEG frame says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JpegInfo {
    pub width: usize,
    pub height: usize,
    /// Colour components (1 grey, 3 colour).
    pub components: u8,
    /// Whether the data ends with the end-of-image marker.
    pub complete: bool,
}

/// The start marker of an image and the marker that ends it.
const SOI: [u8; 2] = [0xFF, 0xD8];
const EOI: [u8; 2] = [0xFF, 0xD9];
/// The marker of the application segments that the simulated camera fills with the pixels.
const APP15: u8 = 0xEF;
/// The label that opens such a segment.
const STORED: &[u8] = b"EH3DRAW\0";
/// The most bytes of a picture a segment carries (65535 less its length field and the label).
pub const STORED_CHUNK: usize = 65_535 - 2 - 8;
/// The most pixels the stored form is read for.
const MAX_STORED: usize = 25_000_000;

/// The size, the components and the ending of a motion JPEG frame. A frame that does not start as a JPEG is `frame_not_decoded`; one that is cut before its size or its end is
/// `frame_truncated`; a table that says its length is more than there is left is `frame_truncated` too.
pub fn jpeg_info(bytes: &[u8]) -> Result<JpegInfo, CapError> {
    if bytes.len() < 2 || bytes[..2] != SOI {
        return Err(CapError::new(
            Code::FrameNotDecoded,
            "the frame does not start as a JPEG picture",
        ));
    }
    let mut at = 2;
    let mut size: Option<(usize, usize, u8)> = None;
    loop {
        // find the next marker: 0xFF followed by a byte that is not 0 or 0xFF
        while at < bytes.len() && bytes[at] != 0xFF {
            at += 1;
        }
        while at < bytes.len() && bytes[at] == 0xFF {
            at += 1;
        }
        let Some(&marker) = bytes.get(at) else {
            return Err(CapError::new(Code::FrameTruncated, "no end of picture"));
        };
        at += 1;
        match marker {
            0x00 | 0x01 | 0xD0..=0xD7 => continue,
            0xD9 => break,
            _ => {}
        }
        let Some(len) = bytes
            .get(at..at + 2)
            .map(|l| usize::from(l[0]) << 8 | usize::from(l[1]))
        else {
            return Err(CapError::new(Code::FrameTruncated, "cut inside a table"));
        };
        if len < 2 || at + len > bytes.len() {
            return Err(CapError::new(
                Code::FrameTruncated,
                "a table is longer than the frame",
            ));
        }
        if matches!(marker, 0xC0..=0xC2) {
            let body = &bytes[at + 2..at + len];
            if body.len() < 6 {
                return Err(CapError::new(Code::FrameTruncated, "the size is cut"));
            }
            let (h, w) = (
                usize::from(body[1]) << 8 | usize::from(body[2]),
                usize::from(body[3]) << 8 | usize::from(body[4]),
            );
            if w == 0 || h == 0 {
                return Err(CapError::new(Code::FrameBadSize, "the size is 0"));
            }
            size = Some((w, h, body[5]));
        }
        at += len;
        if marker == 0xDA {
            // the entropy-coded data follows; the end of the picture is the last marker
            let tail = &bytes[at..];
            let trimmed = tail.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
            let complete = trimmed >= 2 && tail[trimmed - 2..trimmed] == EOI;
            let (width, height, components) = size.ok_or_else(|| {
                CapError::new(Code::FrameNotDecoded, "the data starts before the size")
            })?;
            if !complete {
                return Err(CapError::new(Code::FrameTruncated, "no end of picture"));
            }
            return Ok(JpegInfo {
                width,
                height,
                components,
                complete,
            });
        }
    }
    let (width, height, components) =
        size.ok_or_else(|| CapError::new(Code::FrameNotDecoded, "the picture has no size"))?;
    Ok(JpegInfo {
        width,
        height,
        components,
        complete: true,
    })
}

/// The bytes of a motion JPEG frame in the stored form: the markers of a JPEG with a size, and the pixels in segments `APP15`. This is what the simulated camera sends; no camera does.
pub fn stored_jpeg(luma: &Gray) -> Vec<u8> {
    let mut out = SOI.to_vec();
    for chunk in luma.data.chunks(STORED_CHUNK) {
        out.extend_from_slice(&[0xFF, APP15]);
        out.extend_from_slice(&((chunk.len() + 2 + STORED.len()) as u16).to_be_bytes());
        out.extend_from_slice(STORED);
        out.extend_from_slice(chunk);
    }
    // the frame header: length 11, 8 bits, height, width, one component
    out.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x0B, 0x08]);
    out.extend_from_slice(&(luma.h as u16).to_be_bytes());
    out.extend_from_slice(&(luma.w as u16).to_be_bytes());
    out.extend_from_slice(&[0x01, 0x01, 0x11, 0x00]);
    // the start of the data: length 8, one component, no spectral selection, then nothing to code
    out.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00]);
    out.extend_from_slice(&EOI);
    out
}

/// The brightness of a motion JPEG frame in the stored form. A frame whose pixels are coded needs a decoder, and is `frame_not_decoded`.
pub fn mjpeg_to_luma(bytes: &[u8]) -> Result<Gray, CapError> {
    let info = jpeg_info(bytes)?;
    let want = info
        .width
        .checked_mul(info.height)
        .filter(|&n| n <= MAX_STORED)
        .ok_or_else(|| CapError::new(Code::FrameBadSize, "the picture is too large"))?;
    let mut data: Vec<u8> = Vec::with_capacity(want);
    let mut at = 2;
    while at + 4 <= bytes.len() && bytes[at] == 0xFF {
        let (marker, len) = (
            bytes[at + 1],
            usize::from(bytes[at + 2]) << 8 | usize::from(bytes[at + 3]),
        );
        if marker != APP15 {
            at += 2 + len;
            continue;
        }
        let body = bytes.get(at + 4..at + 2 + len).unwrap_or(&[]);
        if let Some(pixels) = body.strip_prefix(STORED) {
            if data.len() + pixels.len() > want {
                return Err(CapError::new(
                    Code::FrameBadSize,
                    "more pixels than the size says",
                ));
            }
            data.extend_from_slice(pixels);
        }
        at += 2 + len;
    }
    if data.is_empty() {
        return Err(CapError::new(
            Code::FrameNotDecoded,
            "the pixels of a coded JPEG picture need the decoder, which this program does not have yet",
        ));
    }
    if data.len() < want {
        return Err(CapError::new(
            Code::FrameTruncated,
            format!("{} pixels of {want}", data.len()),
        ));
    }
    gray(info.width, info.height, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(w: usize, h: usize) -> Gray {
        Gray::new(w, h, (0..w * h).map(|i| (i * 7 % 256) as u8).collect()).unwrap()
    }

    fn yuyv_of(g: &Gray) -> Vec<u8> {
        g.data.iter().flat_map(|&y| [y, 128]).collect()
    }

    #[test]
    fn yuyv_gives_back_the_brightness_and_grey_chroma_gives_grey_colour() {
        let g = ramp(8, 4);
        let bytes = yuyv_of(&g);
        assert_eq!(yuyv_to_luma(&bytes, 8, 4).unwrap(), g);
        let rgb = yuyv_to_rgb(&bytes, 8, 4).unwrap();
        assert_eq!(rgb.len(), 8 * 4 * 3);
        for (i, px) in rgb.chunks(3).enumerate() {
            assert_eq!(px, [g.data[i]; 3]);
        }
    }

    #[test]
    fn a_frame_of_the_wrong_length_or_of_an_odd_width_is_refused_with_its_code() {
        let bytes = yuyv_of(&ramp(8, 4));
        assert_eq!(
            yuyv_to_luma(&bytes[..bytes.len() - 1], 8, 4)
                .unwrap_err()
                .code,
            Code::FrameTruncated
        );
        let mut long = bytes.clone();
        long.push(0);
        assert_eq!(
            yuyv_to_luma(&long, 8, 4).unwrap_err().code,
            Code::FrameBadSize
        );
        assert_eq!(
            yuyv_to_luma(&bytes, 7, 4).unwrap_err().code,
            Code::FrameBadSize
        );
        assert!(yuyv_to_luma(&[], 0, 4).is_err());
        assert!(yuyv_to_luma(&bytes, usize::MAX, 2).is_err());
    }

    #[test]
    fn colour_goes_to_brightness_with_the_usual_weights() {
        let rgb = [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
        let g = rgb24_to_luma(&rgb, 4, 1).unwrap();
        assert_eq!(g.data, vec![76, 150, 29, 255]);
        assert_eq!(
            gray8_to_luma(&[1, 2, 3, 4], 2, 2).unwrap().data,
            [1, 2, 3, 4]
        );
        assert!(gray8_to_luma(&[1, 2, 3], 2, 2).is_err());
    }

    #[test]
    fn a_stored_motion_jpeg_frame_gives_back_its_pixels_and_its_header_is_read() {
        let g = ramp(320, 240);
        let bytes = stored_jpeg(&g);
        let info = jpeg_info(&bytes).unwrap();
        assert_eq!((info.width, info.height, info.complete), (320, 240, true));
        assert_eq!(mjpeg_to_luma(&bytes).unwrap(), g);
        assert!(bytes.len() > 320 * 240);
    }

    #[test]
    fn a_cut_motion_jpeg_frame_is_truncated_wherever_it_is_cut() {
        let bytes = stored_jpeg(&ramp(16, 8));
        for cut in 0..bytes.len() {
            let e = mjpeg_to_luma(&bytes[..cut]).unwrap_err();
            assert!(
                matches!(e.code, Code::FrameTruncated | Code::FrameNotDecoded),
                "cut at {cut}: {e}"
            );
        }
        assert_eq!(
            jpeg_info(&bytes[..bytes.len() - 2]).unwrap_err().code,
            Code::FrameTruncated
        );
    }

    #[test]
    fn a_frame_that_is_not_a_jpeg_or_that_needs_the_decoder_says_so() {
        assert_eq!(jpeg_info(b"PNG").unwrap_err().code, Code::FrameNotDecoded);
        assert_eq!(jpeg_info(&[]).unwrap_err().code, Code::FrameNotDecoded);
        // a header and an end with no stored pixels is a coded picture
        let mut coded = stored_jpeg(&ramp(4, 4));
        let first = coded.iter().position(|&b| b == 0xEF).unwrap() - 1;
        let len = usize::from(coded[first + 2]) << 8 | usize::from(coded[first + 3]);
        coded.drain(first..first + 2 + len);
        assert!(jpeg_info(&coded).is_ok());
        assert_eq!(
            mjpeg_to_luma(&coded).unwrap_err().code,
            Code::FrameNotDecoded
        );
    }

    #[test]
    fn a_table_that_claims_more_than_the_frame_has_is_refused() {
        let mut bytes = stored_jpeg(&ramp(8, 8));
        bytes[4] = 0xFF;
        bytes[5] = 0xFF;
        assert!(jpeg_info(&bytes).is_err());
        let zero = [
            0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x0B, 8, 0, 0, 0, 5, 1, 1, 0x11, 0, 0xFF, 0xD9,
        ];
        assert_eq!(jpeg_info(&zero).unwrap_err().code, Code::FrameBadSize);
    }
}
