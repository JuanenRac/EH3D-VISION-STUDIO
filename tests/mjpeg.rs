//! The reader of motion JPEG frames (`mjpeg`): pictures coded by a small coder of the test (baseline, with its own tables, one or three components, restart intervals) are read back within a
//! step or two of a grey level, and a frame that is cut at every place, has a byte changed, a marker put where none can be, a size of billions of pixels or only noise is an error with a
//! code that does not change and never a panic or an allocation without a limit. The seed is fixed.

use eh3d_vision_studio::capture::pixels::mjpeg_to_luma;
use eh3d_vision_studio::image::Gray;
use eh3d_vision_studio::mjpeg::{
    code_of, decode, JpegSettings, BAD_DATA, BAD_MARKER, BAD_SIZE, BAD_TABLE, CODES, FILE_TOO_BIG,
    NOT_JPEG, TRUNCATED, UNSUPPORTED,
};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

struct Writer {
    out: Vec<u8>,
    acc: u32,
    n: u32,
}

impl Writer {
    fn bits(&mut self, value: u32, count: u32) {
        for i in (0..count).rev() {
            self.acc = (self.acc << 1) | ((value >> i) & 1);
            self.n += 1;
            if self.n == 8 {
                self.byte();
            }
        }
    }
    fn byte(&mut self) {
        let b = self.acc as u8;
        self.out.push(b);
        if b == 0xFF {
            self.out.push(0);
        }
        self.acc = 0;
        self.n = 0;
    }
    fn flush(&mut self) {
        while self.n != 0 {
            self.bits(1, 1);
        }
    }
}

/// The symbols of the table of the coefficients that are not the first, in the order of their codes, which are all of 8 bits.
fn ac_symbols() -> Vec<u8> {
    let mut v = vec![0x00u8, 0xF0];
    for r in 0..16u8 {
        for s in 1..=10u8 {
            v.push(r << 4 | s);
        }
    }
    v
}

fn fdct(block: &[f32; 64]) -> [f32; 64] {
    let mut out = [0f32; 64];
    for v in 0..8 {
        for u in 0..8 {
            let mut sum = 0f64;
            for y in 0..8 {
                for x in 0..8 {
                    sum += f64::from(block[y * 8 + x])
                        * ((2 * x + 1) as f64 * u as f64 * std::f64::consts::PI / 16.0).cos()
                        * ((2 * y + 1) as f64 * v as f64 * std::f64::consts::PI / 16.0).cos();
                }
            }
            let c = |k: usize| {
                if k == 0 {
                    std::f64::consts::FRAC_1_SQRT_2
                } else {
                    1.0
                }
            };
            out[v * 8 + u] = (0.25 * c(u) * c(v) * sum) as f32;
        }
    }
    out
}

fn category(v: i32) -> u32 {
    if v == 0 {
        0
    } else {
        32 - v.unsigned_abs().leading_zeros()
    }
}

fn put_value(w: &mut Writer, v: i32, cat: u32) {
    if cat == 0 {
        return;
    }
    let raw = if v > 0 { v } else { v - 1 };
    let bits = (raw as u32) & ((1 << cat) - 1);
    w.bits(bits, cat);
}

struct Options {
    colour: bool,
    restart: usize,
    step: u8,
}

fn block_at(g: &Gray, bx: usize, by: usize) -> [f32; 64] {
    let mut b = [0f32; 64];
    for y in 0..8 {
        for x in 0..8 {
            let (px, py) = ((bx * 8 + x).min(g.w - 1), (by * 8 + y).min(g.h - 1));
            b[y * 8 + x] = f32::from(g.data[py * g.w + px]) - 128.0;
        }
    }
    b
}

fn write_block(w: &mut Writer, b: &[f32; 64], pred: &mut i32, step: u8, syms: &[u8]) {
    let f = fdct(b);
    let q: Vec<i32> = (0..64)
        .map(|k| (f[ZIGZAG[k]] / f32::from(step)).round() as i32)
        .collect();
    let diff = q[0] - *pred;
    *pred = q[0];
    let cat = category(diff);
    w.bits(cat, 4);
    put_value(w, diff, cat);
    let code_of = |s: u8| syms.iter().position(|&x| x == s).unwrap() as u32;
    let mut run = 0u32;
    let last = (1..64).rev().find(|&k| q[k] != 0).unwrap_or(0);
    for &value in q.iter().take(last + 1).skip(1) {
        if value == 0 {
            run += 1;
            continue;
        }
        while run > 15 {
            w.bits(code_of(0xF0), 8);
            run -= 16;
        }
        let cat = category(value);
        assert!(cat <= 10, "the test picture is too sharp for the tables");
        w.bits(code_of((run << 4 | cat) as u8), 8);
        put_value(w, value, cat);
        run = 0;
    }
    if last < 63 {
        w.bits(code_of(0x00), 8);
    }
}

fn marker(out: &mut Vec<u8>, m: u8, body: &[u8]) {
    out.extend_from_slice(&[0xFF, m]);
    out.extend_from_slice(&((body.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(body);
}

/// A baseline JPEG of the picture with tables of its own: the first coefficient with codes of 4 bits, the others of 8 bits, and a table of quantisation with one step everywhere. With
/// `colour` the picture is the luma with 2 by 2 sampling and two components of colour that are flat.
fn encode(g: &Gray, o: &Options) -> Vec<u8> {
    let syms = ac_symbols();
    let mut out = vec![0xFF, 0xD8];
    marker(&mut out, 0xDB, &[&[0u8][..], &[o.step; 64][..]].concat());
    let mut dc = vec![0x00u8];
    let mut counts = [0u8; 16];
    counts[3] = 12;
    dc.extend_from_slice(&counts);
    dc.extend(0..12u8);
    marker(&mut out, 0xC4, &dc);
    let mut ac = vec![0x10u8];
    let mut counts = [0u8; 16];
    counts[7] = 162;
    ac.extend_from_slice(&counts);
    ac.extend_from_slice(&syms);
    marker(&mut out, 0xC4, &ac);
    if o.restart > 0 {
        marker(&mut out, 0xDD, &(o.restart as u16).to_be_bytes());
    }
    let mut sof = vec![8u8];
    sof.extend_from_slice(&(g.h as u16).to_be_bytes());
    sof.extend_from_slice(&(g.w as u16).to_be_bytes());
    let (unit, comps) = if o.colour {
        sof.extend_from_slice(&[3, 1, 0x22, 0, 2, 0x11, 0, 3, 0x11, 0]);
        (16usize, 3usize)
    } else {
        sof.extend_from_slice(&[1, 1, 0x11, 0]);
        (8, 1)
    };
    marker(&mut out, 0xC0, &sof);
    let mut sos = vec![comps as u8];
    for c in 1..=comps as u8 {
        sos.extend_from_slice(&[c, 0x00]);
    }
    sos.extend_from_slice(&[0, 63, 0]);
    marker(&mut out, 0xDA, &sos);
    let mut w = Writer {
        out: Vec::new(),
        acc: 0,
        n: 0,
    };
    let (mx, my) = (g.w.div_ceil(unit), g.h.div_ceil(unit));
    let mut preds = [0i32; 3];
    let zero = [0f32; 64];
    let mut n = 0usize;
    for y in 0..my {
        for x in 0..mx {
            if o.restart > 0 && n > 0 && n % o.restart < 1 {
                w.flush();
                w.out
                    .extend_from_slice(&[0xFF, 0xD0 + ((n / o.restart - 1) % 8) as u8]);
                preds = [0; 3];
            }
            n += 1;
            if o.colour {
                for (bx, by) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let b = block_at(g, x * 2 + bx, y * 2 + by);
                    write_block(&mut w, &b, &mut preds[0], o.step, &syms);
                }
                write_block(&mut w, &zero, &mut preds[1], o.step, &syms);
                write_block(&mut w, &zero, &mut preds[2], o.step, &syms);
            } else {
                let b = block_at(g, x, y);
                write_block(&mut w, &b, &mut preds[0], o.step, &syms);
            }
        }
    }
    w.flush();
    out.extend_from_slice(&w.out);
    out.extend_from_slice(&[0xFF, 0xD9]);
    out
}

/// A smooth picture: a slope with a soft bump, whose coefficients are small.
fn picture(w: usize, h: usize) -> Gray {
    let data: Vec<u8> = (0..w * h)
        .map(|i| {
            let (x, y) = ((i % w) as f64, (i / w) as f64);
            let bump = 40.0
                * (-(((x - w as f64 / 2.0).powi(2) + (y - h as f64 / 2.0).powi(2)) / 80.0)).exp();
            (60.0 + x * 2.0 + y + bump).clamp(0.0, 255.0) as u8
        })
        .collect();
    Gray::new(w, h, data).unwrap()
}

fn close(a: &Gray, b: &Gray, within: i32) -> bool {
    a.w == b.w
        && a.h == b.h
        && a.data
            .iter()
            .zip(&b.data)
            .all(|(&x, &y)| (i32::from(x) - i32::from(y)).abs() <= within)
}

fn known(e: &str) -> bool {
    CODES.contains(&code_of(e))
}

#[test]
fn a_coded_grey_picture_is_read_back_within_two_levels() {
    for (w, h) in [(8, 8), (17, 13), (64, 48), (1, 1), (33, 7)] {
        let g = picture(w, h);
        let bytes = encode(
            &g,
            &Options {
                colour: false,
                restart: 0,
                step: 1,
            },
        );
        let read = decode(&bytes, &JpegSettings::default()).unwrap();
        assert!(close(&g, &read, 2), "{w} by {h}");
    }
}

#[test]
fn a_coded_colour_picture_gives_its_brightness_and_the_restarts_do_not_change_it() {
    let g = picture(37, 21);
    for restart in [0usize, 1, 2, 3] {
        let bytes = encode(
            &g,
            &Options {
                colour: true,
                restart,
                step: 1,
            },
        );
        let read = decode(&bytes, &JpegSettings::default()).unwrap();
        assert!(close(&g, &read, 2), "restart {restart}");
    }
    let grey = encode(
        &picture(40, 24),
        &Options {
            colour: false,
            restart: 2,
            step: 2,
        },
    );
    assert!(close(
        &picture(40, 24),
        &decode(&grey, &JpegSettings::default()).unwrap(),
        4
    ));
}

#[test]
fn the_path_of_the_camera_reads_a_coded_frame() {
    let g = picture(32, 16);
    let bytes = encode(
        &g,
        &Options {
            colour: false,
            restart: 0,
            step: 1,
        },
    );
    assert!(close(&g, &mjpeg_to_luma(&bytes).unwrap(), 2));
}

#[test]
fn a_frame_cut_at_any_place_is_an_error_with_a_known_code() {
    for colour in [false, true] {
        let bytes = encode(
            &picture(20, 18),
            &Options {
                colour,
                restart: 2,
                step: 1,
            },
        );
        for cut in 0..bytes.len() {
            let e = decode(&bytes[..cut], &JpegSettings::default())
                .expect_err(&format!("cut at {cut}"));
            assert!(known(&e), "{e}");
            if cut >= 2 {
                assert!(
                    [TRUNCATED, BAD_DATA, BAD_TABLE, BAD_MARKER].contains(&code_of(&e)),
                    "cut at {cut}: {e}"
                );
            }
        }
    }
}

#[test]
fn a_frame_with_bytes_changed_or_markers_put_in_is_never_a_panic() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let bytes = encode(
        &picture(24, 16),
        &Options {
            colour: true,
            restart: 3,
            step: 1,
        },
    );
    let mut errors = 0usize;
    for _ in 0..4000 {
        let mut b = bytes.clone();
        for _ in 0..1 + rng.below(4) {
            let at = rng.below(b.len());
            match rng.below(5) {
                0 => b[at] = rng.next() as u8,
                1 => b[at] ^= 1 << rng.below(8),
                2 => {
                    b.insert(at, 0xFF);
                    b.insert(at + 1, (rng.next() % 256) as u8);
                }
                3 => {
                    b.remove(at);
                }
                _ => {
                    let end = (at + 1 + rng.below(8)).min(b.len());
                    b.drain(at..end);
                }
            }
            if b.len() < 4 {
                break;
            }
        }
        match decode(&b, &JpegSettings::default()) {
            Ok(g) => assert!(g.w * g.h <= 25_000_000 && g.data.len() == g.w * g.h),
            Err(e) => {
                errors += 1;
                assert!(known(&e), "{e}");
            }
        }
    }
    assert!(errors > 200, "{errors} of 4000 were refused");
}

#[test]
fn noise_and_short_inputs_are_refused_with_their_codes() {
    let mut rng = Rng(7);
    for n in 0..200usize {
        let mut b = vec![0xFF, 0xD8];
        b.extend((0..n).map(|_| rng.next() as u8));
        if let Err(e) = decode(&b, &JpegSettings::default()) {
            assert!(known(&e), "{e}");
        }
    }
    for b in [
        &[][..],
        &[0xFF][..],
        &[0xFF, 0xD8][..],
        &b"GIF89a"[..],
        &b"PNG"[..],
    ] {
        let e = decode(b, &JpegSettings::default()).unwrap_err();
        assert!(known(&e), "{e}");
    }
    assert!(decode(b"GIF89a", &JpegSettings::default())
        .unwrap_err()
        .starts_with(NOT_JPEG));
}

#[test]
fn a_size_of_billions_of_pixels_is_refused_before_anything_is_made_of_it() {
    let good = encode(
        &picture(16, 16),
        &Options {
            colour: false,
            restart: 0,
            step: 1,
        },
    );
    let sof = good.windows(2).position(|w| w == [0xFF, 0xC0]).unwrap();
    for (h, w) in [
        (65535u16, 65535u16),
        (65535, 1),
        (1, 65535),
        (20_001, 8),
        (5000, 5001),
    ] {
        let mut b = good.clone();
        b[sof + 5..sof + 7].copy_from_slice(&h.to_be_bytes());
        b[sof + 7..sof + 9].copy_from_slice(&w.to_be_bytes());
        let e = decode(&b, &JpegSettings::default()).unwrap_err();
        assert!(e.starts_with(BAD_SIZE), "{h} by {w}: {e}");
    }
    let mut zero = good.clone();
    zero[sof + 5..sof + 9].copy_from_slice(&[0, 0, 0, 16]);
    assert!(decode(&zero, &JpegSettings::default())
        .unwrap_err()
        .starts_with(BAD_SIZE));
    let tight = JpegSettings {
        max_bytes: 1 << 20,
        max_pixels: 64,
    };
    assert!(decode(&good, &tight).unwrap_err().starts_with(BAD_SIZE));
}

#[test]
fn a_frame_over_the_limit_of_bytes_and_a_kind_that_is_not_read_are_refused() {
    let good = encode(
        &picture(16, 16),
        &Options {
            colour: false,
            restart: 0,
            step: 1,
        },
    );
    let small = JpegSettings {
        max_bytes: 1024,
        max_pixels: 1_000_000,
    };
    let mut big = good.clone();
    big.resize(2000, 0);
    assert!(decode(&big, &small).unwrap_err().starts_with(FILE_TOO_BIG));
    assert!(decode(&good, &small).is_ok());
    let sof = good.windows(2).position(|w| w == [0xFF, 0xC0]).unwrap();
    let mut progressive = good.clone();
    progressive[sof + 1] = 0xC2;
    assert!(decode(&progressive, &JpegSettings::default())
        .unwrap_err()
        .starts_with(UNSUPPORTED));
    let mut twelve = good.clone();
    twelve[sof + 4] = 12;
    assert!(decode(&twelve, &JpegSettings::default())
        .unwrap_err()
        .starts_with(UNSUPPORTED));
    let mut four = good.clone();
    four[sof + 9] = 4;
    assert!(decode(&four, &JpegSettings::default()).is_err());
}

#[test]
fn tables_that_cannot_be_are_refused() {
    let good = encode(
        &picture(16, 16),
        &Options {
            colour: false,
            restart: 0,
            step: 1,
        },
    );
    // a step of quantisation of 0
    let dqt = good.windows(2).position(|w| w == [0xFF, 0xDB]).unwrap();
    let mut zero = good.clone();
    zero[dqt + 5] = 0;
    assert!(decode(&zero, &JpegSettings::default())
        .unwrap_err()
        .starts_with(BAD_TABLE));
    // more codes of 4 bits than there are
    let dht = good.windows(2).position(|w| w == [0xFF, 0xC4]).unwrap();
    let mut over = good.clone();
    over[dht + 5 + 3] = 17;
    assert!(decode(&over, &JpegSettings::default()).is_err());
    // a marker that says it is shorter than its own length field
    let mut short = good.clone();
    short[dqt + 2..dqt + 4].copy_from_slice(&[0, 1]);
    assert!(decode(&short, &JpegSettings::default())
        .unwrap_err()
        .starts_with(BAD_MARKER));
    // the data before the header of the frame
    let mut early = vec![0xFF, 0xD8];
    early.extend_from_slice(&[0xFF, 0xDA, 0, 8, 1, 1, 0, 0, 63, 0, 1, 2, 3]);
    assert!(decode(&early, &JpegSettings::default())
        .unwrap_err()
        .starts_with(BAD_MARKER));
}

#[test]
fn what_is_read_twice_gives_the_same_picture() {
    let g = picture(29, 31);
    let bytes = encode(
        &g,
        &Options {
            colour: true,
            restart: 2,
            step: 2,
        },
    );
    assert_eq!(
        decode(&bytes, &JpegSettings::default()).unwrap(),
        decode(&bytes, &JpegSettings::default()).unwrap()
    );
}
