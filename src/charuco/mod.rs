//! The board of calibration. A chessboard of squares has, in each of its white squares, a small marker of 4 by 4 bits with a black border; every marker has a number, so the part of the board
//! that the camera sees says which corners of the chessboard are in the picture (a chessboard alone cannot say where in the board it is looking, nor which way it is turned). The markers
//! come from a dictionary that is made from a seed, so the same seed gives the same markers on every run and no list of codes is copied from anywhere; every code differs from every other
//! code, turned any way, in at least three bits, so a bit that is read wrong is corrected and a marker that is turned is told from another. The board can be drawn for a given view
//! (`render`), which is what the tests and the simulated camera show.

pub mod card;
pub mod detect;
pub mod solve;
pub mod store;

use crate::image::Gray;
use crate::perspective::Homography;
use crate::sim::SimRng;

/// How the board is laid out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardSpec {
    /// Squares along x and along y.
    pub cols: usize,
    pub rows: usize,
    /// The side of a square in millimetres.
    pub square_mm: f64,
    /// The seed of the dictionary of markers.
    pub seed: u64,
}

impl Default for BoardSpec {
    fn default() -> Self {
        BoardSpec {
            cols: 8,
            rows: 6,
            square_mm: 5.0,
            seed: 1,
        }
    }
}

/// The side of a marker as a share of the side of its square.
pub const MARKER_SHARE: f64 = 0.6;
/// The bits on a side of a marker, and the cells with its border.
pub const BITS: usize = 4;
pub const CELLS: usize = BITS + 2;

impl BoardSpec {
    /// Whether the layout can be made and found: from 3 to 20 squares each way, a square of 1 to 100 millimetres.
    pub fn check(&self) -> Result<(), String> {
        if !(3..=20).contains(&self.cols) || !(3..=20).contains(&self.rows) {
            return Err("the board needs from 3 to 20 squares along each side".into());
        }
        if !(self.square_mm.is_finite() && (1.0..=100.0).contains(&self.square_mm)) {
            return Err("the square must be from 1 to 100 millimetres".into());
        }
        Ok(())
    }

    /// The inner corners (where four squares meet): `(cols - 1) * (rows - 1)`.
    pub fn corner_count(&self) -> usize {
        (self.cols - 1) * (self.rows - 1)
    }

    /// The markers: one in each white square (the squares where `i + j` is odd).
    pub fn marker_count(&self) -> usize {
        (0..self.rows)
            .flat_map(|j| (0..self.cols).map(move |i| (i, j)))
            .filter(|&(i, j)| (i + j) % 2 == 1)
            .count()
    }

    /// The number of the marker in the white square `(i, j)`, in the order of reading the board.
    pub fn marker_id(&self, i: usize, j: usize) -> Option<usize> {
        if i >= self.cols || j >= self.rows || (i + j) % 2 != 1 {
            return None;
        }
        Some(
            (0..j)
                .map(|jj| (0..self.cols).filter(|&ii| (ii + jj) % 2 == 1).count())
                .sum::<usize>()
                + (0..i).filter(|&ii| (ii + j) % 2 == 1).count(),
        )
    }

    /// The square of a marker.
    pub fn marker_square(&self, id: usize) -> Option<(usize, usize)> {
        (0..self.rows)
            .flat_map(|j| (0..self.cols).map(move |i| (i, j)))
            .find(|&(i, j)| self.marker_id(i, j) == Some(id))
    }

    /// The four corners of a marker in millimetres on the board (x right, y down), starting at the top left and going round clockwise.
    pub fn marker_corners_mm(&self, id: usize) -> Option<[(f64, f64); 4]> {
        let (i, j) = self.marker_square(id)?;
        let s = self.square_mm;
        let (cx, cy) = ((i as f64 + 0.5) * s, (j as f64 + 0.5) * s);
        let half = MARKER_SHARE * s / 2.0;
        Some([
            (cx - half, cy - half),
            (cx + half, cy - half),
            (cx + half, cy + half),
            (cx - half, cy + half),
        ])
    }

    /// Where the inner corner `(i, j)` is in millimetres (`i` from 1 to `cols - 1`, `j` from 1 to `rows - 1`).
    pub fn corner_mm(&self, i: usize, j: usize) -> (f64, f64) {
        (i as f64 * self.square_mm, j as f64 * self.square_mm)
    }

    /// The size of the board in millimetres.
    pub fn size_mm(&self) -> (f64, f64) {
        (
            self.cols as f64 * self.square_mm,
            self.rows as f64 * self.square_mm,
        )
    }
}

/// The codes of the markers, 16 bits each (bit `r * 4 + c` is the cell in row `r` and column `c`, 1 for white).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dictionary {
    pub codes: Vec<u16>,
}

/// The code turned a quarter of a turn clockwise.
pub fn rotate_cw(code: u16) -> u16 {
    let mut out = 0u16;
    for r in 0..BITS {
        for c in 0..BITS {
            let bit = (code >> ((BITS - 1 - c) * BITS + r)) & 1;
            out |= bit << (r * BITS + c);
        }
    }
    out
}

fn hamming(a: u16, b: u16) -> u32 {
    (a ^ b).count_ones()
}

/// The fewest bits in which two different markers, or a marker and itself turned, differ.
pub const MIN_DISTANCE: u32 = 3;

impl Dictionary {
    /// `count` codes made from the seed, each at least `MIN_DISTANCE` bits away from every other code turned any way and from itself turned. Up to 64 codes.
    pub fn new(seed: u64, count: usize) -> Result<Dictionary, String> {
        if count == 0 || count > 64 {
            return Err("a dictionary has from 1 to 64 markers".into());
        }
        let mut rng = SimRng::new(seed ^ 0xD1C7_10A4);
        let mut codes: Vec<u16> = Vec::new();
        for _ in 0..200_000 {
            if codes.len() == count {
                break;
            }
            let c = rng.below(65_536) as u16;
            let turns = [
                c,
                rotate_cw(c),
                rotate_cw(rotate_cw(c)),
                rotate_cw(rotate_cw(rotate_cw(c))),
            ];
            let ones = c.count_ones();
            if !(5..=11).contains(&ones) {
                continue;
            }
            let self_ok = (1..4).all(|k| hamming(c, turns[k]) >= MIN_DISTANCE);
            let others_ok = codes
                .iter()
                .all(|&d| turns.iter().all(|&t| hamming(t, d) >= MIN_DISTANCE));
            if self_ok && others_ok {
                codes.push(c);
            }
        }
        if codes.len() < count {
            return Err(format!(
                "only {} markers could be made at the distance of {MIN_DISTANCE} bits",
                codes.len()
            ));
        }
        Ok(Dictionary { codes })
    }

    /// The dictionary of a board.
    pub fn for_board(spec: &BoardSpec) -> Result<Dictionary, String> {
        spec.check()?;
        Dictionary::new(spec.seed, spec.marker_count())
    }

    /// The marker that the bits are, and how many quarter turns clockwise bring the bits read to the marker. A code within one bit of a marker (turned any way) is that marker; a code
    /// that is that near two markers, or none, is not one.
    pub fn identify(&self, bits: u16) -> Option<(usize, u8)> {
        let mut turned = bits;
        let mut found: Option<(usize, u8)> = None;
        for k in 0..4u8 {
            for (id, &code) in self.codes.iter().enumerate() {
                if hamming(turned, code) <= (MIN_DISTANCE - 1) / 2 {
                    if found.is_some() {
                        return None;
                    }
                    found = Some((id, k));
                }
            }
            turned = rotate_cw(turned);
        }
        found
    }
}

/// The grey of the board at a point in millimetres (x right, y down); the paper outside the board is light.
pub fn value_at(spec: &BoardSpec, dict: &Dictionary, x: f64, y: f64) -> f64 {
    const BLACK: f64 = 25.0;
    const WHITE: f64 = 235.0;
    let (w, h) = spec.size_mm();
    if x < 0.0 || y < 0.0 || x >= w || y >= h {
        return WHITE;
    }
    let s = spec.square_mm;
    let (i, j) = ((x / s) as usize, (y / s) as usize);
    if (i + j) % 2 == 0 {
        return BLACK;
    }
    let Some(id) = spec.marker_id(i, j) else {
        return WHITE;
    };
    let side = MARKER_SHARE * s;
    let (u, v) = (
        (x - (i as f64 + 0.5) * s + side / 2.0) / side,
        (y - (j as f64 + 0.5) * s + side / 2.0) / side,
    );
    if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
        return WHITE;
    }
    let (c, r) = ((u * CELLS as f64) as usize, (v * CELLS as f64) as usize);
    if r == 0 || c == 0 || r == CELLS - 1 || c == CELLS - 1 {
        return BLACK;
    }
    let bit = (dict.codes[id] >> ((r - 1) * BITS + (c - 1))) & 1;
    if bit == 1 {
        WHITE
    } else {
        BLACK
    }
}

/// Draws the board as the camera sees it: `to_px` takes millimetres on the board to pixels. Each pixel is the mean of 3 by 3 samples, so the edges are soft the way a lens makes them,
/// and `noise` adds up to that many grey levels either way from a generator that is seeded.
pub fn render(
    spec: &BoardSpec,
    dict: &Dictionary,
    to_px: &Homography,
    w: usize,
    h: usize,
    noise: u8,
    seed: u64,
) -> Result<Gray, String> {
    let back = to_px
        .inverse()
        .ok_or("the view cannot be turned back: it is flat")?;
    let mut rng = SimRng::new(seed);
    let mut data = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0;
            for sy in 0..3 {
                for sx in 0..3 {
                    let p = (
                        x as f64 + (sx as f64 + 0.5) / 3.0,
                        y as f64 + (sy as f64 + 0.5) / 3.0,
                    );
                    sum += back
                        .apply(p)
                        .map_or(235.0, |(mx, my)| value_at(spec, dict, mx, my));
                }
            }
            let mut v = sum / 9.0;
            if noise > 0 {
                v += rng.below(2 * u64::from(noise) + 1) as f64 - f64::from(noise);
            }
            data.push(v.round().clamp(0.0, 255.0) as u8);
        }
    }
    Gray::new(w, h, data)
}

/// The view of a board seen from straight above: `px_per_mm` pixels for a millimetre, the board turned `deg` degrees and its middle at `(cx, cy)` in the picture.
pub fn view_flat(spec: &BoardSpec, px_per_mm: f64, deg: f64, cx: f64, cy: f64) -> Homography {
    let (bw, bh) = spec.size_mm();
    let (s, c) = deg.to_radians().sin_cos();
    let k = px_per_mm;
    let (ox, oy) = (bw / 2.0, bh / 2.0);
    // pixel = centre + k * R * (mm - board middle)
    Homography([
        k * c,
        -k * s,
        cx - k * (c * ox - s * oy),
        k * s,
        k * c,
        cy - k * (s * ox + c * oy),
        0.0,
        0.0,
        1.0,
    ])
}

/// The view of a board seen with one side nearer than the other: the flat view, then the four corners of the board are moved by `tilt` (a share of the width of the board in pixels,
/// bigger on the near side).
pub fn view_tilted(
    spec: &BoardSpec,
    px_per_mm: f64,
    deg: f64,
    cx: f64,
    cy: f64,
    tilt: f64,
) -> Option<Homography> {
    let flat = view_flat(spec, px_per_mm, deg, cx, cy);
    let (bw, bh) = spec.size_mm();
    let corners = [(0.0, 0.0), (bw, 0.0), (bw, bh), (0.0, bh)];
    let mut to = [(0.0, 0.0); 4];
    for (k, &p) in corners.iter().enumerate() {
        let (x, y) = flat.apply(p)?;
        let (mx, my) = flat.apply((bw / 2.0, bh / 2.0))?;
        // points on the near side (the bottom) are pushed outward, those on the far side pulled in
        let near = if p.1 > bh / 2.0 { 1.0 } else { -1.0 };
        let f = 1.0 + near * tilt;
        to[k] = (mx + (x - mx) * f, my + (y - my));
    }
    Homography::from_points(&corners, &to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turn_four_times_is_the_same_code_and_a_turn_moves_the_cells() {
        for code in [0x1234u16, 0xBEEF, 0x0001, 0x8000, 0xFFFF] {
            let t = rotate_cw(rotate_cw(rotate_cw(rotate_cw(code))));
            assert_eq!(t, code);
        }
        // cell (row 0, column 0) goes to (row 0, column 3) with a quarter of a turn clockwise
        assert_eq!(rotate_cw(0b0001), 0b1000);
        assert_eq!(rotate_cw(0x0001), 1 << 3);
    }

    #[test]
    fn the_same_seed_gives_the_same_dictionary_and_another_seed_another() {
        let a = Dictionary::new(1, 24).unwrap();
        assert_eq!(a, Dictionary::new(1, 24).unwrap());
        assert_ne!(a, Dictionary::new(2, 24).unwrap());
        assert_eq!(a.codes.len(), 24);
    }

    #[test]
    fn every_code_is_far_from_every_other_code_turned_and_from_itself_turned() {
        let d = Dictionary::new(9, 30).unwrap();
        for (i, &a) in d.codes.iter().enumerate() {
            let mut t = a;
            for k in 0..4 {
                if k > 0 {
                    assert!(hamming(a, t) >= MIN_DISTANCE, "code {i} turned {k}");
                }
                for (j, &b) in d.codes.iter().enumerate() {
                    if i != j {
                        assert!(hamming(t, b) >= MIN_DISTANCE);
                    }
                }
                t = rotate_cw(t);
            }
        }
    }

    #[test]
    fn a_marker_is_identified_whatever_its_turn_and_with_one_bit_wrong() {
        let d = Dictionary::new(3, 20).unwrap();
        for (id, &code) in d.codes.iter().enumerate() {
            // the bits read after the board is turned m quarter turns clockwise
            let mut read = code;
            for m in 0..4u8 {
                assert_eq!(
                    d.identify(read),
                    Some((id, (4 - m) % 4)),
                    "marker {id} turn {m}"
                );
                for bit in 0..16 {
                    assert_eq!(d.identify(read ^ (1 << bit)).map(|x| x.0), Some(id));
                }
                read = rotate_cw(read);
            }
        }
        assert_eq!(d.identify(0), None);
        assert_eq!(d.identify(0xFFFF), None);
    }

    #[test]
    fn too_many_markers_or_none_is_refused() {
        assert!(Dictionary::new(1, 0).is_err());
        assert!(Dictionary::new(1, 65).is_err());
    }

    #[test]
    fn the_board_counts_its_corners_and_markers_and_numbers_the_white_squares_in_order() {
        let s = BoardSpec::default();
        assert_eq!(s.corner_count(), 7 * 5);
        assert_eq!(s.marker_count(), 24);
        let ids: Vec<usize> = (0..s.rows)
            .flat_map(|j| (0..s.cols).map(move |i| (i, j)))
            .filter_map(|(i, j)| s.marker_id(i, j))
            .collect();
        assert_eq!(ids, (0..24).collect::<Vec<_>>());
        assert_eq!(s.marker_id(0, 0), None);
        assert_eq!(s.marker_id(1, 0), Some(0));
        assert_eq!(s.marker_square(23), Some((6, 5)));
        assert_eq!(s.marker_id(99, 0), None);
    }

    #[test]
    fn the_corners_of_a_marker_are_inside_its_square_and_clockwise() {
        let s = BoardSpec::default();
        let c = s.marker_corners_mm(0).unwrap();
        assert!(c[0].0 < c[1].0 && c[0].1 < c[3].1);
        assert!((c[1].0 - c[0].0 - 3.0).abs() < 1e-9);
        assert!(c[0].0 > 5.0 && c[1].0 < 10.0);
        assert!(s.marker_corners_mm(24).is_none());
    }

    #[test]
    fn a_layout_that_cannot_be_made_is_refused() {
        let ok = BoardSpec::default();
        assert!(ok.check().is_ok());
        for bad in [
            BoardSpec { cols: 2, ..ok },
            BoardSpec { rows: 21, ..ok },
            BoardSpec {
                square_mm: 0.5,
                ..ok
            },
            BoardSpec {
                square_mm: f64::NAN,
                ..ok
            },
        ] {
            assert!(bad.check().is_err());
        }
    }

    #[test]
    fn the_board_is_drawn_black_and_white_with_the_markers_inside_the_white_squares() {
        let s = BoardSpec::default();
        let d = Dictionary::for_board(&s).unwrap();
        assert!(value_at(&s, &d, 2.5, 2.5) < 100.0, "square (0,0) is black");
        assert!(
            value_at(&s, &d, 5.2, 0.2) > 200.0,
            "the margin of a white square"
        );
        assert!(value_at(&s, &d, -1.0, 2.0) > 200.0, "the paper");
        // the border of the marker of the square (1,0) is black
        assert!(value_at(&s, &d, 5.0 + 1.0 + 0.1, 1.0 + 0.1,) < 100.0);
    }

    #[test]
    fn a_flat_view_puts_the_middle_of_the_board_at_the_point_asked() {
        let s = BoardSpec::default();
        let v = view_flat(&s, 8.0, 30.0, 320.0, 240.0);
        let (bw, bh) = s.size_mm();
        let (x, y) = v.apply((bw / 2.0, bh / 2.0)).unwrap();
        assert!((x - 320.0).abs() < 1e-9 && (y - 240.0).abs() < 1e-9);
        let (x1, y1) = v.apply((bw / 2.0 + 1.0, bh / 2.0)).unwrap();
        assert!((((x1 - x).powi(2) + (y1 - y).powi(2)).sqrt() - 8.0).abs() < 1e-9);
    }

    #[test]
    fn a_picture_is_drawn_in_the_size_asked_with_soft_edges_and_repeats_with_its_seed() {
        let s = BoardSpec::default();
        let d = Dictionary::for_board(&s).unwrap();
        let v = view_flat(&s, 8.0, 5.0, 160.0, 120.0);
        let a = render(&s, &d, &v, 320, 240, 2, 5).unwrap();
        assert_eq!((a.w, a.h), (320, 240));
        assert_eq!(a, render(&s, &d, &v, 320, 240, 2, 5).unwrap());
        assert_ne!(a, render(&s, &d, &v, 320, 240, 2, 6).unwrap());
        let soft = a.data.iter().filter(|&&p| (60..200).contains(&p)).count();
        assert!(soft > 50, "the edges are soft: {soft}");
        let flat = Homography([0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
        assert!(render(&s, &d, &flat, 10, 10, 0, 1).is_err());
    }
}
