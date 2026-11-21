//! A reader of the pictures of a motion JPEG frame that never trusts the bytes. It reads the baseline kind of JPEG (one scan or several, sequential, 8 bits, Huffman, with restart
//! intervals; one component or three, any sampling from 1 to 4) and gives the brightness of the picture, which is what every search of the program works on. The colour components are
//! read to find their place in the data and then thrown away.
//!
//! Every frame is hostile until it has been read: the size of the file, the number of pixels and the tables are checked before anything is made from them, nothing is allocated from
//! a number of the frame beyond the limits of [`JpegSettings`], and a frame that is cut, has a marker where none can be, a table that cannot be, a code that is not in its table or a
//! size that is absurd is an error with a code that does not change and never a panic. A kind of JPEG that is not read (progressive, arithmetic, 12 bits, four components) is the
//! error [`UNSUPPORTED`] and not a guess. The picture is given only when the frame ends with its end marker.

/// The code of the refusal of bytes that do not begin as a JPEG picture.
pub const NOT_JPEG: &str = "jpeg_not_jpeg";
/// The code of the refusal of a frame larger than the limit of bytes.
pub const FILE_TOO_BIG: &str = "jpeg_file_too_big";
/// The code of the refusal of a frame that is cut before the picture is whole, or has no end.
pub const TRUNCATED: &str = "jpeg_truncated";
/// The code of the refusal of a marker that is where it cannot be or whose length is wrong.
pub const BAD_MARKER: &str = "jpeg_bad_marker";
/// The code of the refusal of a table of quantisation or of Huffman codes that cannot be, or that is not there when the data needs it.
pub const BAD_TABLE: &str = "jpeg_bad_table";
/// The code of the refusal of a size of 0 or with more pixels than the limit.
pub const BAD_SIZE: &str = "jpeg_bad_size";
/// The code of the refusal of a kind of JPEG that is not read.
pub const UNSUPPORTED: &str = "jpeg_unsupported";
/// The code of the refusal of coded data that does not follow its tables.
pub const BAD_DATA: &str = "jpeg_bad_data";

/// Every code of this reader, in the order of the list above.
pub const CODES: [&str; 8] = [
    NOT_JPEG,
    FILE_TOO_BIG,
    TRUNCATED,
    BAD_MARKER,
    BAD_TABLE,
    BAD_SIZE,
    UNSUPPORTED,
    BAD_DATA,
];

/// The limits of the reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JpegSettings {
    /// The most bytes of a frame.
    pub max_bytes: usize,
    /// The most pixels of a picture.
    pub max_pixels: usize,
}

impl Default for JpegSettings {
    fn default() -> Self {
        JpegSettings {
            max_bytes: 16_777_216,
            max_pixels: 25_000_000,
        }
    }
}

/// The least and the most of each limit that any setting may ask for.
pub const BYTES_RANGE: (usize, usize) = (1024, 268_435_456);
pub const PIXELS_RANGE: (usize, usize) = (64, 25_000_000);

/// Whether the settings make sense; the reason when they do not.
pub fn check(s: &JpegSettings) -> Result<(), String> {
    if !(BYTES_RANGE.0..=BYTES_RANGE.1).contains(&s.max_bytes) {
        return Err(format!(
            "the most bytes of a frame must be from {} to {}",
            BYTES_RANGE.0, BYTES_RANGE.1
        ));
    }
    if !(PIXELS_RANGE.0..=PIXELS_RANGE.1).contains(&s.max_pixels) {
        return Err(format!(
            "the most pixels of a frame must be from {} to {}",
            PIXELS_RANGE.0, PIXELS_RANGE.1
        ));
    }
    Ok(())
}

/// The code at the start of a refusal of this reader (`jpeg_truncated: ...` gives `jpeg_truncated`); the whole text when it does not start with one.
pub fn code_of(error: &str) -> &str {
    error.split(':').next().unwrap_or(error)
}

fn fail(code: &str, why: &str) -> String {
    format!("{code}: {why}")
}

/// The order of the coefficients of a block of 8 by 8 in the data (zigzag), as the place of each in the block row after row.
const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// A table of Huffman codes ready to decode: for each length the first code, the last code and the place of its first symbol.
#[derive(Clone)]
struct Huffman {
    min_code: [i32; 17],
    max_code: [i32; 18],
    first: [usize; 17],
    symbols: Vec<u8>,
}

impl Huffman {
    fn build(counts: &[u8; 16], symbols: &[u8]) -> Result<Huffman, String> {
        let total: usize = counts.iter().map(|&c| usize::from(c)).sum();
        if total == 0 || total > 256 || total != symbols.len() {
            return Err(fail(
                BAD_TABLE,
                "a table of codes has no symbols or too many",
            ));
        }
        let mut h = Huffman {
            min_code: [0; 17],
            max_code: [-1; 18],
            first: [0; 17],
            symbols: symbols.to_vec(),
        };
        let (mut code, mut at) = (0i32, 0usize);
        for len in 1..=16usize {
            let n = i32::from(counts[len - 1]);
            h.first[len] = at;
            h.min_code[len] = code;
            if n > 0 {
                h.max_code[len] = code + n - 1;
            }
            code += n;
            if code > (1 << len) {
                return Err(fail(
                    BAD_TABLE,
                    "the codes of a table do not fit their lengths",
                ));
            }
            at += n as usize;
            code <<= 1;
        }
        Ok(h)
    }
}

/// The bits of the coded data, with the 0 that follows an 0xFF taken out. A marker in the data stops it: nothing more is read, and a request for more bits is a cut frame.
struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    buf: u32,
    have: u32,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8], pos: usize) -> Bits<'a> {
        Bits {
            data,
            pos,
            buf: 0,
            have: 0,
        }
    }

    fn fill(&mut self) -> Result<(), String> {
        let Some(&b) = self.data.get(self.pos) else {
            return Err(fail(TRUNCATED, "the coded data ends inside the picture"));
        };
        if b == 0xFF {
            match self.data.get(self.pos + 1) {
                Some(0x00) => self.pos += 2,
                Some(_) => {
                    return Err(fail(TRUNCATED, "a marker interrupts the coded data"));
                }
                None => return Err(fail(TRUNCATED, "the coded data ends inside the picture")),
            }
        } else {
            self.pos += 1;
        }
        self.buf = (self.buf << 8) | u32::from(b);
        self.have += 8;
        Ok(())
    }

    fn bit(&mut self) -> Result<u32, String> {
        if self.have == 0 {
            self.fill()?;
        }
        self.have -= 1;
        Ok((self.buf >> self.have) & 1)
    }

    fn bits(&mut self, n: u32) -> Result<i32, String> {
        let mut v = 0i32;
        for _ in 0..n {
            v = (v << 1) | self.bit()? as i32;
        }
        Ok(v)
    }

    fn symbol(&mut self, h: &Huffman) -> Result<u8, String> {
        let mut code = 0i32;
        for len in 1..=16usize {
            code = (code << 1) | self.bit()? as i32;
            if h.max_code[len] >= 0 && code <= h.max_code[len] && code >= h.min_code[len] {
                let k = h.first[len] + (code - h.min_code[len]) as usize;
                return h
                    .symbols
                    .get(k)
                    .copied()
                    .ok_or_else(|| fail(BAD_DATA, "a code points outside its table"));
            }
        }
        Err(fail(BAD_DATA, "a code is not in its table"))
    }

    /// Drops the bits left of the byte and takes the restart marker that must be next.
    fn restart(&mut self) -> Result<(), String> {
        self.have = 0;
        self.buf = 0;
        let mut at = self.pos;
        while self.data.get(at) == Some(&0xFF) && self.data.get(at + 1) == Some(&0xFF) {
            at += 1;
        }
        match (self.data.get(at), self.data.get(at + 1)) {
            (Some(0xFF), Some(m)) if (0xD0..=0xD7).contains(m) => {
                self.pos = at + 2;
                Ok(())
            }
            (None, _) | (Some(_), None) => Err(fail(
                TRUNCATED,
                "the coded data ends before a restart marker",
            )),
            _ => Err(fail(BAD_DATA, "a restart marker is not where it should be")),
        }
    }
}

/// The value of `n` bits of the data as the number they stand for (the first bit says the sign).
fn extend(v: i32, n: u32) -> i32 {
    if n == 0 {
        0
    } else if v < (1 << (n - 1)) {
        v - (1 << n) + 1
    } else {
        v
    }
}

#[derive(Clone, Copy)]
struct Component {
    id: u8,
    h: usize,
    v: usize,
    tq: usize,
}

struct Frame {
    width: usize,
    height: usize,
    comps: Vec<Component>,
    hmax: usize,
    vmax: usize,
    mcus_x: usize,
    mcus_y: usize,
}

fn be16(bytes: &[u8], at: usize) -> Option<usize> {
    Some(usize::from(*bytes.get(at)?) << 8 | usize::from(*bytes.get(at + 1)?))
}

fn cosines() -> [[f32; 8]; 8] {
    let mut c = [[0f32; 8]; 8];
    for (x, row) in c.iter_mut().enumerate() {
        for (u, v) in row.iter_mut().enumerate() {
            let a = if u == 0 {
                std::f64::consts::FRAC_1_SQRT_2
            } else {
                1.0
            };
            *v = (0.5 * a * (((2 * x + 1) * u) as f64 * std::f64::consts::PI / 16.0).cos()) as f32;
        }
    }
    c
}

/// The brightness of the picture in a JPEG frame, within the limits of `s`.
pub fn decode(bytes: &[u8], s: &JpegSettings) -> Result<crate::image::Gray, String> {
    check(s)?;
    if bytes.len() > s.max_bytes {
        return Err(fail(
            FILE_TOO_BIG,
            &format!(
                "the frame has {} bytes, more than the {} allowed",
                bytes.len(),
                s.max_bytes
            ),
        ));
    }
    if bytes.len() < 2 || bytes[..2] != [0xFF, 0xD8] {
        return Err(fail(NOT_JPEG, "the frame does not start as a JPEG picture"));
    }
    let mut quant: [Option<[u16; 64]>; 4] = [None; 4];
    let mut dc: [Option<Huffman>; 4] = [None, None, None, None];
    let mut ac: [Option<Huffman>; 4] = [None, None, None, None];
    let mut frame: Option<Frame> = None;
    let mut restart_interval = 0usize;
    let mut plane: Option<(Vec<u8>, usize, usize)> = None;
    let mut luma_done = false;
    let mut at = 2usize;
    loop {
        // the next marker: 0xFF and then a byte that is not 0 or 0xFF
        while at < bytes.len() && bytes[at] != 0xFF {
            at += 1;
        }
        while at < bytes.len() && bytes[at] == 0xFF {
            at += 1;
        }
        let Some(&marker) = bytes.get(at) else {
            return Err(fail(TRUNCATED, "the frame has no end marker"));
        };
        at += 1;
        match marker {
            0x00 | 0x01 | 0xD0..=0xD7 => continue,
            0xD8 => return Err(fail(BAD_MARKER, "a start marker inside the picture")),
            0xD9 => break,
            _ => {}
        }
        let len = be16(bytes, at).ok_or_else(|| fail(TRUNCATED, "cut inside a marker"))?;
        if len < 2 {
            return Err(fail(BAD_MARKER, "a marker has a length under 2"));
        }
        if at + len > bytes.len() {
            return Err(fail(TRUNCATED, "a marker is longer than the frame"));
        }
        let body = &bytes[at + 2..at + len];
        at += len;
        match marker {
            0xDB => parse_dqt(body, &mut quant)?,
            0xC4 => parse_dht(body, &mut dc, &mut ac)?,
            0xDD => {
                if body.len() != 2 {
                    return Err(fail(BAD_MARKER, "the restart interval has a wrong length"));
                }
                restart_interval = be16(body, 0).unwrap_or(0);
            }
            0xC0 | 0xC1 => {
                if frame.is_some() {
                    return Err(fail(BAD_MARKER, "the frame header comes twice"));
                }
                let f = parse_sof(body, s)?;
                let (pw, ph) = (f.mcus_x * f.comps[0].h * 8, f.mcus_y * f.comps[0].v * 8);
                plane = Some((vec![0u8; pw * ph], pw, ph));
                frame = Some(f);
            }
            0xC2 | 0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => {
                return Err(fail(UNSUPPORTED, "only the baseline kind of JPEG is read"));
            }
            0xDA => {
                let Some(f) = frame.as_ref() else {
                    return Err(fail(
                        BAD_MARKER,
                        "the coded data comes before the frame header",
                    ));
                };
                let scan = parse_sos(body, f)?;
                if !luma_done && scan.iter().any(|c| c.index == 0) {
                    if let Some((data, pw, _)) = plane.as_mut() {
                        let mut bits = Bits::new(bytes, at);
                        decode_scan(
                            &mut bits,
                            f,
                            &scan,
                            &quant,
                            &dc,
                            &ac,
                            restart_interval,
                            data,
                            *pw,
                        )?;
                        at = bits.pos;
                        luma_done = true;
                    }
                }
            }
            _ => {}
        }
    }
    let (Some(f), Some((data, pw, _)), true) = (frame, plane, luma_done) else {
        return Err(fail(TRUNCATED, "the frame ends before the coded picture"));
    };
    let mut out = Vec::with_capacity(f.width * f.height);
    let (sx, sy) = (f.comps[0].h, f.comps[0].v);
    for y in 0..f.height {
        let row = (y * sy / f.vmax) * pw;
        for x in 0..f.width {
            out.push(data[row + x * sx / f.hmax]);
        }
    }
    crate::image::Gray::new(f.width, f.height, out).map_err(|e| fail(BAD_SIZE, &e))
}

fn parse_dqt(mut body: &[u8], quant: &mut [Option<[u16; 64]>; 4]) -> Result<(), String> {
    while !body.is_empty() {
        let (pq, tq) = (body[0] >> 4, usize::from(body[0] & 15));
        if pq > 1 || tq > 3 {
            return Err(fail(
                BAD_TABLE,
                "a table of quantisation has a wrong precision or number",
            ));
        }
        let size = if pq == 0 { 64 } else { 128 };
        if body.len() < 1 + size {
            return Err(fail(BAD_TABLE, "a table of quantisation is cut"));
        }
        let mut t = [0u16; 64];
        for (i, v) in t.iter_mut().enumerate() {
            *v = if pq == 0 {
                u16::from(body[1 + i])
            } else {
                u16::from(body[1 + 2 * i]) << 8 | u16::from(body[2 + 2 * i])
            };
            if *v == 0 {
                return Err(fail(BAD_TABLE, "a table of quantisation has a step of 0"));
            }
        }
        quant[tq] = Some(t);
        body = &body[1 + size..];
    }
    Ok(())
}

fn parse_dht(
    mut body: &[u8],
    dc: &mut [Option<Huffman>; 4],
    ac: &mut [Option<Huffman>; 4],
) -> Result<(), String> {
    while !body.is_empty() {
        let (class, th) = (body[0] >> 4, usize::from(body[0] & 15));
        if class > 1 || th > 3 || body.len() < 17 {
            return Err(fail(
                BAD_TABLE,
                "a table of codes has a wrong class or number, or is cut",
            ));
        }
        let mut counts = [0u8; 16];
        counts.copy_from_slice(&body[1..17]);
        let n: usize = counts.iter().map(|&c| usize::from(c)).sum();
        if n > 256 || body.len() < 17 + n {
            return Err(fail(
                BAD_TABLE,
                "a table of codes is cut or has too many symbols",
            ));
        }
        let table = Huffman::build(&counts, &body[17..17 + n])?;
        if class == 0 {
            dc[th] = Some(table);
        } else {
            ac[th] = Some(table);
        }
        body = &body[17 + n..];
    }
    Ok(())
}

fn parse_sof(body: &[u8], s: &JpegSettings) -> Result<Frame, String> {
    if body.len() < 6 {
        return Err(fail(BAD_MARKER, "the frame header is cut"));
    }
    if body[0] != 8 {
        return Err(fail(UNSUPPORTED, "only pictures of 8 bits are read"));
    }
    let (height, width) = (be16(body, 1).unwrap_or(0), be16(body, 3).unwrap_or(0));
    let n = usize::from(body[5]);
    if body.len() != 6 + 3 * n {
        return Err(fail(BAD_MARKER, "the frame header has a wrong length"));
    }
    if width == 0 || height == 0 {
        return Err(fail(BAD_SIZE, "the size is 0"));
    }
    if width > crate::image::MAX_SIDE
        || height > crate::image::MAX_SIDE
        || (width as u64) * (height as u64) > s.max_pixels as u64
    {
        return Err(fail(
            BAD_SIZE,
            &format!(
                "{width} by {height} pixels is more than the {} allowed",
                s.max_pixels
            ),
        ));
    }
    if n != 1 && n != 3 {
        return Err(fail(
            UNSUPPORTED,
            "only pictures of one or three components are read",
        ));
    }
    let mut comps: Vec<Component> = Vec::with_capacity(n);
    for c in body[6..].chunks(3) {
        let comp = Component {
            id: c[0],
            h: usize::from(c[1] >> 4),
            v: usize::from(c[1] & 15),
            tq: usize::from(c[2]),
        };
        if !(1..=4).contains(&comp.h) || !(1..=4).contains(&comp.v) || comp.tq > 3 {
            return Err(fail(
                BAD_MARKER,
                "a component has a sampling or a table that cannot be",
            ));
        }
        if comps.iter().any(|o| o.id == comp.id) {
            return Err(fail(BAD_MARKER, "two components have the same number"));
        }
        comps.push(comp);
    }
    if n == 1 {
        // one component is one block to a unit of the data, whatever sampling it says
        comps[0].h = 1;
        comps[0].v = 1;
    }
    let hmax = comps.iter().map(|c| c.h).max().unwrap_or(1);
    let vmax = comps.iter().map(|c| c.v).max().unwrap_or(1);
    let (mcus_x, mcus_y) = (width.div_ceil(8 * hmax), height.div_ceil(8 * vmax));
    let padded = (mcus_x * comps[0].h * 8) as u64 * (mcus_y * comps[0].v * 8) as u64;
    if padded > s.max_pixels as u64 + ((width + height + 64) * 32) as u64 {
        return Err(fail(
            BAD_SIZE,
            "the size with its padding is more than the pixels allowed",
        ));
    }
    Ok(Frame {
        width,
        height,
        comps,
        hmax,
        vmax,
        mcus_x,
        mcus_y,
    })
}

struct ScanComp {
    index: usize,
    td: usize,
    ta: usize,
}

fn parse_sos(body: &[u8], f: &Frame) -> Result<Vec<ScanComp>, String> {
    let Some(&ns) = body.first() else {
        return Err(fail(BAD_MARKER, "the start of the coded data is cut"));
    };
    let ns = usize::from(ns);
    if ns == 0 || ns > f.comps.len() || body.len() != 4 + 2 * ns {
        return Err(fail(
            BAD_MARKER,
            "the start of the coded data has a wrong length",
        ));
    }
    let mut out: Vec<ScanComp> = Vec::with_capacity(ns);
    for c in body[1..1 + 2 * ns].chunks(2) {
        let Some(index) = f.comps.iter().position(|k| k.id == c[0]) else {
            return Err(fail(
                BAD_MARKER,
                "the coded data names a component that is not there",
            ));
        };
        if out.iter().any(|o| o.index == index) {
            return Err(fail(BAD_MARKER, "the coded data names a component twice"));
        }
        let (td, ta) = (usize::from(c[1] >> 4), usize::from(c[1] & 15));
        if td > 3 || ta > 3 {
            return Err(fail(
                BAD_TABLE,
                "the coded data names a table that cannot be",
            ));
        }
        out.push(ScanComp { index, td, ta });
    }
    let tail = &body[1 + 2 * ns..];
    if tail != [0, 63, 0] {
        return Err(fail(UNSUPPORTED, "only the baseline kind of JPEG is read"));
    }
    if ns > 1 && ns != f.comps.len() {
        return Err(fail(
            UNSUPPORTED,
            "a scan of some of the components together is not read",
        ));
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn decode_scan(
    bits: &mut Bits,
    f: &Frame,
    scan: &[ScanComp],
    quant: &[Option<[u16; 64]>; 4],
    dc: &[Option<Huffman>; 4],
    ac: &[Option<Huffman>; 4],
    restart_interval: usize,
    plane: &mut [u8],
    plane_w: usize,
) -> Result<(), String> {
    let missing = || fail(BAD_TABLE, "the coded data needs a table that was not given");
    let mut tables: Vec<(&Huffman, &Huffman, &[u16; 64])> = Vec::with_capacity(scan.len());
    for c in scan {
        let d = dc[c.td].as_ref().ok_or_else(missing)?;
        let a = ac[c.ta].as_ref().ok_or_else(missing)?;
        let q = quant[f.comps[c.index].tq].as_ref().ok_or_else(missing)?;
        tables.push((d, a, q));
    }
    let interleaved = scan.len() > 1;
    // the grid of MCUs of this scan: the whole frame when the components come together, else the blocks of the one component
    let (grid_x, grid_y) = if interleaved || f.comps.len() == 1 {
        (f.mcus_x, f.mcus_y)
    } else {
        let c = &f.comps[scan[0].index];
        (
            (f.width * c.h).div_ceil(f.hmax).div_ceil(8),
            (f.height * c.v).div_ceil(f.vmax).div_ceil(8),
        )
    };
    let cos = cosines();
    let mut pred = vec![0i32; scan.len()];
    let total = grid_x * grid_y;
    for n in 0..total {
        if restart_interval > 0 && n > 0 && n % restart_interval == 0 {
            bits.restart()?;
            pred.iter_mut().for_each(|p| *p = 0);
        }
        let (mx, my) = (n % grid_x, n / grid_x);
        for (k, c) in scan.iter().enumerate() {
            let comp = &f.comps[c.index];
            let (bh, bv) = if interleaved {
                (comp.h, comp.v)
            } else {
                (1, 1)
            };
            for by in 0..bv {
                for bx in 0..bh {
                    let (d, a, q) = tables[k];
                    let block = read_block(bits, d, a, q, &mut pred[k])?;
                    if c.index == 0 {
                        let (px, py) = ((mx * bh + bx) * 8, (my * bv + by) * 8);
                        put_block(&block, &cos, plane, plane_w, px, py);
                    }
                }
            }
        }
    }
    Ok(())
}

fn read_block(
    bits: &mut Bits,
    d: &Huffman,
    a: &Huffman,
    q: &[u16; 64],
    pred: &mut i32,
) -> Result<[f32; 64], String> {
    let mut block = [0f32; 64];
    let t = u32::from(bits.symbol(d)?);
    if t > 11 {
        return Err(fail(
            BAD_DATA,
            "a size of the first coefficient that cannot be",
        ));
    }
    let diff = extend(bits.bits(t)?, t);
    *pred = pred.saturating_add(diff).clamp(-32768, 32767);
    block[0] = *pred as f32 * f32::from(q[0]);
    let mut k = 1usize;
    while k < 64 {
        let rs = bits.symbol(a)?;
        let (run, size) = (usize::from(rs >> 4), u32::from(rs & 15));
        if size == 0 {
            if run == 15 {
                k += 16;
                continue;
            }
            break;
        }
        if size > 10 {
            return Err(fail(BAD_DATA, "a size of a coefficient that cannot be"));
        }
        k += run;
        if k > 63 {
            return Err(fail(
                BAD_DATA,
                "the coefficients of a block run past its end",
            ));
        }
        let v = extend(bits.bits(size)?, size);
        block[ZIGZAG[k]] = v as f32 * f32::from(q[k]);
        k += 1;
    }
    Ok(block)
}

fn put_block(
    block: &[f32; 64],
    cos: &[[f32; 8]; 8],
    plane: &mut [u8],
    plane_w: usize,
    px: usize,
    py: usize,
) {
    // along the rows first, then along the columns
    let mut tmp = [0f32; 64];
    for v in 0..8 {
        for x in 0..8 {
            tmp[v * 8 + x] = (0..8).map(|u| cos[x][u] * block[v * 8 + u]).sum();
        }
    }
    for (y, row) in cos.iter().enumerate() {
        for x in 0..8 {
            let s: f32 = (0..8).map(|v| row[v] * tmp[v * 8 + x]).sum();
            let value = (s + 128.0).round().clamp(0.0, 255.0) as u8;
            if let Some(cell) = plane.get_mut((py + y) * plane_w + px + x) {
                *cell = value;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extend_gives_the_signed_value_of_the_bits() {
        assert_eq!(extend(0, 0), 0);
        assert_eq!(extend(1, 1), 1);
        assert_eq!(extend(0, 1), -1);
        assert_eq!(extend(0b10, 2), 2);
        assert_eq!(extend(0b00, 2), -3);
    }

    #[test]
    fn a_table_that_does_not_fit_its_lengths_is_refused() {
        let mut counts = [0u8; 16];
        counts[0] = 3;
        assert!(Huffman::build(&counts, &[0, 1, 2]).is_err());
        counts[0] = 2;
        assert!(Huffman::build(&counts, &[0, 1]).is_ok());
        assert!(Huffman::build(&counts, &[0]).is_err());
    }

    #[test]
    fn the_code_is_the_start_of_the_text() {
        assert_eq!(code_of("jpeg_truncated: cut"), "jpeg_truncated");
        assert_eq!(code_of("no code"), "no code");
    }

    #[test]
    fn settings_are_checked() {
        assert!(check(&JpegSettings::default()).is_ok());
        assert!(check(&JpegSettings {
            max_bytes: 10,
            max_pixels: 100
        })
        .is_err());
        assert!(check(&JpegSettings {
            max_bytes: 4096,
            max_pixels: 0
        })
        .is_err());
    }
}
