//! Grey levels from a PNG or a PGM file, with limits so that a hostile file cannot ask for gigabytes.

use std::io::Cursor;

pub const MAX_SIDE: usize = 20_000;
pub const MAX_PIXELS: usize = 25_000_000;

/// 0 is black and 255 is white.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gray {
    pub w: usize,
    pub h: usize,
    pub data: Vec<u8>,
}

impl Gray {
    pub fn new(w: usize, h: usize, data: Vec<u8>) -> Result<Gray, String> {
        if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE || w * h > MAX_PIXELS {
            return Err(format!("{w} x {h} pixels: each side must be from 1 to {MAX_SIDE} and the image at most {MAX_PIXELS} pixels"));
        }
        if data.len() != w * h {
            return Err(format!(
                "{w} x {h} pixels need {} values, not {}",
                w * h,
                data.len()
            ));
        }
        Ok(Gray { w, h, data })
    }

    pub fn blank(w: usize, h: usize, value: u8) -> Result<Gray, String> {
        Gray::new(w, h, vec![value; w.saturating_mul(h).min(MAX_PIXELS + 1)])
    }

    pub fn at(&self, x: usize, y: usize) -> u8 {
        self.data[y * self.w + x]
    }

    /// Paints a disc (for tests and for drawing marks on a picture).
    pub fn disc(&mut self, cx: f64, cy: f64, r: f64, value: u8) {
        let (x0, x1) = (
            (cx - r).floor().max(0.0) as usize,
            ((cx + r).ceil().max(0.0) as usize).min(self.w.saturating_sub(1)),
        );
        let (y0, y1) = (
            (cy - r).floor().max(0.0) as usize,
            ((cy + r).ceil().max(0.0) as usize).min(self.h.saturating_sub(1)),
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                if ((x as f64 - cx).powi(2) + (y as f64 - cy).powi(2)).sqrt() <= r {
                    self.data[y * self.w + x] = value;
                }
            }
        }
    }
}

fn number(bytes: &[u8], i: &mut usize) -> Result<usize, String> {
    loop {
        while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
            *i += 1;
        }
        if *i < bytes.len() && bytes[*i] == b'#' {
            while *i < bytes.len() && bytes[*i] != b'\n' {
                *i += 1;
            }
        } else {
            break;
        }
    }
    let start = *i;
    while *i < bytes.len() && bytes[*i].is_ascii_digit() {
        *i += 1;
    }
    std::str::from_utf8(&bytes[start..*i])
        .ok()
        .filter(|s| !s.is_empty() && s.len() < 10)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| "the PGM header has something that is not a number".to_string())
}

/// A PGM file, P2 or P5, 8 bits.
pub fn from_pgm(bytes: &[u8]) -> Result<Gray, String> {
    if bytes.len() < 2 || bytes[0] != b'P' || !(bytes[1] == b'2' || bytes[1] == b'5') {
        return Err("not a PGM file (it must begin with P2 or P5)".into());
    }
    let mut i = 2;
    let (w, h, max) = (
        number(bytes, &mut i)?,
        number(bytes, &mut i)?,
        number(bytes, &mut i)?,
    );
    if max == 0
        || max > 255
        || w == 0
        || h == 0
        || w > MAX_SIDE
        || h > MAX_SIDE
        || w * h > MAX_PIXELS
    {
        return Err(format!(
            "{w} x {h} pixels with a maximum of {max} cannot be read"
        ));
    }
    let scale = |v: usize| (v.min(max) * 255 / max) as u8;
    let mut data = Vec::with_capacity(w * h);
    if bytes[1] == b'5' {
        i += 1;
        data.extend(
            bytes
                .get(i..i + w * h)
                .ok_or("the PGM file is shorter than its header says")?
                .iter()
                .map(|&b| scale(usize::from(b))),
        );
    } else {
        for _ in 0..w * h {
            data.push(scale(number(bytes, &mut i)?));
        }
    }
    Gray::new(w, h, data)
}

/// A PNG file (grey, colour or with a palette, 8 or 16 bits); transparent parts count as white.
pub fn from_png(bytes: &[u8]) -> Result<Gray, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("the PNG cannot be read: {e}"))?;
    let (w, h) = (reader.info().width as usize, reader.info().height as usize);
    if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE || w * h > MAX_PIXELS {
        return Err(format!("{w} x {h} pixels is too large or empty"));
    }
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("the PNG cannot be read: {e}"))?;
    let (channels, alpha, colour) = match frame.color_type {
        png::ColorType::Grayscale => (1, false, false),
        png::ColorType::GrayscaleAlpha => (2, true, false),
        png::ColorType::Rgb => (3, false, true),
        png::ColorType::Rgba => (4, true, true),
        png::ColorType::Indexed => return Err("a PNG with a palette was not expanded".into()),
    };
    let data = buf[..frame.buffer_size()]
        .chunks_exact(channels)
        .map(|px| {
            let luma = if colour {
                (299 * u32::from(px[0]) + 587 * u32::from(px[1]) + 114 * u32::from(px[2])) / 1000
            } else {
                u32::from(px[0])
            };
            let a = if alpha {
                u32::from(px[channels - 1])
            } else {
                255
            };
            ((luma * a + 255 * (255 - a)) / 255) as u8
        })
        .collect();
    Gray::new(w, h, data)
}

pub fn from_bytes(bytes: &[u8]) -> Result<Gray, String> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        from_png(bytes)
    } else {
        from_pgm(bytes)
    }
}
