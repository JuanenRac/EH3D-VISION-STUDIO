//! Pairing the marks that the camera found with the marks of the design when the board lies turned by any angle, or when a mark is missing or an extra speck was found. The pairing by
//! place (`align::pair_nearest`) is exact when the board is nearly where the design says and refuses what it cannot pair; this one tries every way two marks of the design can be two
//! marks that were found, works out the turn and the shift that each way implies, and keeps the way that explains most marks within the tolerance.
//!
//! It never guesses: when two different ways explain the same number of marks about as well (a design with a symmetry of its own, such as the four corners of a rectangle turned by a
//! half turn, or only two marks found) the answer is the error `pair_ambiguous`, and the turn allowed (`pair_max_turn_deg`) is what a person narrows to tell them apart. A mirrored board
//! is not looked for: it is a different board, and `more::a_mirrored_board_does_not_fit_and_is_not_accepted` keeps it out. The work is bounded by the number of marks of each side.

use crate::align::{accept, fit, Transform};

/// The code of the refusal of a design or a set of marks over the limits.
pub const PAIR_TOO_MANY: &str = "pair_too_many";
/// The code of the refusal of a point that is not a number or lies absurdly far.
pub const PAIR_BAD_POINT: &str = "pair_bad_point";
/// The code of the refusal of an answer that explains fewer marks than `pair_min_marks`.
pub const PAIR_FEW: &str = "pair_few";
/// The code of the refusal when no way of pairing was found at all.
pub const PAIR_NONE: &str = "pair_none";
/// The code of the refusal when two different ways explain the marks about equally well.
pub const PAIR_AMBIGUOUS: &str = "pair_ambiguous";

/// The most marks of a design that are looked at, and the most marks found in a picture.
pub const MAX_DESIGN: usize = 24;
pub const MAX_MEASURED: usize = 48;
/// No coordinate may be farther than this from the origin, in millimetres.
pub const MAX_COORD_MM: f64 = 1.0e6;

/// What is asked of the pairing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairSettings {
    /// How far a mark found may lie from where a way of pairing puts it for the mark to count as paired, in millimetres.
    pub tolerance_mm: f64,
    /// The least marks that must be paired (2 up).
    pub min_marks: usize,
    /// The most that the board may be turned, in degrees (0 to 180): a way that turns it more is not tried.
    pub max_turn_deg: f64,
    /// The most that the board may be bigger or smaller than its design (0.01 is one percent).
    pub max_stretch: f64,
}

impl Default for PairSettings {
    fn default() -> Self {
        PairSettings {
            tolerance_mm: 0.5,
            min_marks: 3,
            max_turn_deg: 180.0,
            max_stretch: 0.01,
        }
    }
}

/// Whether the settings make sense; the reason when they do not.
pub fn check(s: &PairSettings) -> Result<(), String> {
    if !(s.tolerance_mm.is_finite() && s.tolerance_mm > 0.0 && s.tolerance_mm <= 50.0) {
        return Err("the tolerance of the pairing must be more than 0 and at most 50 mm".into());
    }
    if !(2..=MAX_DESIGN).contains(&s.min_marks) {
        return Err(format!(
            "the least marks of the pairing must be from 2 to {MAX_DESIGN}"
        ));
    }
    if !(s.max_turn_deg.is_finite() && (0.0..=180.0).contains(&s.max_turn_deg)) {
        return Err("the turn of the pairing must be from 0 to 180 degrees".into());
    }
    if !(s.max_stretch.is_finite() && (0.0..=0.5).contains(&s.max_stretch)) {
        return Err("the stretch of the pairing must be from 0 to 0.5".into());
    }
    Ok(())
}

/// Two lists of points of the same length and order: the design points and the marks found that go with them.
pub type PointLists = (Vec<(f64, f64)>, Vec<(f64, f64)>);

/// The way the marks were paired.
#[derive(Debug, Clone, PartialEq)]
pub struct Pairing {
    /// `(index in the design, index in the marks found)`, in the order of the design.
    pub pairs: Vec<(usize, usize)>,
    /// The marks of the design that no mark found was paired with.
    pub missing: Vec<usize>,
    /// The marks found that no mark of the design was paired with.
    pub extra: Vec<usize>,
    /// The turn of the board in degrees (counter-clockwise, as `align::Transform`), its shift and what is left between the paired marks.
    pub turn_deg: f64,
    pub dx: f64,
    pub dy: f64,
    pub rms_mm: f64,
}

impl Pairing {
    /// The paired marks as two lists of the same length and order, ready for `align::fit`.
    pub fn points(&self, design: &[(f64, f64)], measured: &[(f64, f64)]) -> PointLists {
        self.pairs
            .iter()
            .map(|&(i, j)| (design[i], measured[j]))
            .unzip()
    }
}

/// The way two marks of the design are two marks found, as a pairing of the marks within the tolerance by the nearest first, one to one.
fn associate(
    t: &Transform,
    design: &[(f64, f64)],
    measured: &[(f64, f64)],
    tol: f64,
) -> Vec<(usize, usize)> {
    let mut cand: Vec<(f64, usize, usize)> = Vec::new();
    for (i, d) in design.iter().enumerate() {
        let p = t.apply(*d);
        for (j, m) in measured.iter().enumerate() {
            let dist = (p.0 - m.0).hypot(p.1 - m.1);
            if dist <= tol {
                cand.push((dist, i, j));
            }
        }
    }
    cand.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.cmp(&b.1))
            .then(a.2.cmp(&b.2))
    });
    let (mut taken, mut used) = (vec![false; design.len()], vec![false; measured.len()]);
    let mut out = Vec::new();
    for (_, i, j) in cand {
        if !taken[i] && !used[j] {
            taken[i] = true;
            used[j] = true;
            out.push((i, j));
        }
    }
    out.sort_unstable();
    out
}

/// Pairs the marks found with the design whatever the turn of the board (up to `max_turn_deg`), a mark missing or an extra one. The design and the marks found are in millimetres.
pub fn pair_turned(
    design: &[(f64, f64)],
    measured: &[(f64, f64)],
    s: &PairSettings,
) -> Result<Pairing, String> {
    check(s)?;
    if design.len() > MAX_DESIGN || measured.len() > MAX_MEASURED {
        return Err(format!(
            "{PAIR_TOO_MANY}: {} design marks and {} marks found; at most {MAX_DESIGN} and {MAX_MEASURED}",
            design.len(),
            measured.len()
        ));
    }
    if design.iter().chain(measured).any(|p| {
        !(p.0.is_finite() && p.1.is_finite())
            || p.0.abs() > MAX_COORD_MM
            || p.1.abs() > MAX_COORD_MM
    }) {
        return Err(format!(
            "{PAIR_BAD_POINT}: a point is not a number or lies too far"
        ));
    }
    if design.len() < s.min_marks || measured.len() < s.min_marks {
        return Err(format!(
            "{PAIR_FEW}: {} design marks and {} marks found, and {} must be paired",
            design.len(),
            measured.len(),
            s.min_marks
        ));
    }
    let tol = s.tolerance_mm;
    // two marks closer than this say too little about the turn
    let baseline = 4.0 * tol;
    let mut best_count = 0usize;
    let mut ways: Vec<(Vec<(usize, usize)>, Transform)> = Vec::new();
    for i in 0..design.len() {
        for j in 0..design.len() {
            if i == j {
                continue;
            }
            let (di, dj) = (design[i], design[j]);
            let dd = (dj.0 - di.0).hypot(dj.1 - di.1);
            if dd < baseline {
                continue;
            }
            let ad = (dj.1 - di.1).atan2(dj.0 - di.0);
            for a in 0..measured.len() {
                for b in 0..measured.len() {
                    if a == b {
                        continue;
                    }
                    let (ma, mb) = (measured[a], measured[b]);
                    let dm = (mb.0 - ma.0).hypot(mb.1 - ma.1);
                    if (dm - dd).abs() > dd * s.max_stretch + 2.0 * tol {
                        continue;
                    }
                    let turn = (mb.1 - ma.1).atan2(mb.0 - ma.0) - ad;
                    let turn_deg = {
                        let mut t = turn.to_degrees();
                        while t > 180.0 {
                            t -= 360.0;
                        }
                        while t <= -180.0 {
                            t += 360.0;
                        }
                        t
                    };
                    if turn_deg.abs() > s.max_turn_deg + 1e-9 {
                        continue;
                    }
                    // the way: the board turned about the mark i of the design so that it lands on the mark a
                    let (sn, cs) = turn.sin_cos();
                    let rigid = Transform {
                        dx: ma.0 - (cs * di.0 - sn * di.1),
                        dy: ma.1 - (sn * di.0 + cs * di.1),
                        theta_deg: turn_deg,
                        scale: 1.0,
                        rms_mm: 0.0,
                    };
                    let first = associate(&rigid, design, measured, tol);
                    if first.len() < s.min_marks || first.len() < best_count {
                        continue;
                    }
                    let (dp, mp): (Vec<_>, Vec<_>) =
                        first.iter().map(|&(x, y)| (design[x], measured[y])).unzip();
                    let Ok(t) = fit(&dp, &mp, true) else { continue };
                    if t.theta_deg.abs() > s.max_turn_deg + 1e-9
                        || accept(&t, tol, s.max_stretch).is_err()
                    {
                        continue;
                    }
                    let pairs = associate(&t, design, measured, tol);
                    if pairs.len() < s.min_marks || pairs.len() < best_count {
                        continue;
                    }
                    let (dp, mp): (Vec<_>, Vec<_>) =
                        pairs.iter().map(|&(x, y)| (design[x], measured[y])).unzip();
                    let Ok(t) = fit(&dp, &mp, true) else { continue };
                    if accept(&t, tol, s.max_stretch).is_err() {
                        continue;
                    }
                    if pairs.len() > best_count {
                        best_count = pairs.len();
                        ways.clear();
                    }
                    if !ways.iter().any(|(p, _)| *p == pairs) {
                        ways.push((pairs, t));
                    }
                }
            }
        }
    }
    if ways.is_empty() {
        return Err(format!(
            "{PAIR_NONE}: no way of pairing explains {} marks within {tol} mm",
            s.min_marks
        ));
    }
    ways.sort_by(|a, b| {
        a.1.rms_mm
            .partial_cmp(&b.1.rms_mm)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    let (pairs, t) = ways[0].clone();
    if let Some((_, other)) = ways.get(1) {
        if other.rms_mm <= (2.0 * t.rms_mm).max(tol / 4.0) {
            return Err(format!(
                "{PAIR_AMBIGUOUS}: two ways of pairing {} marks fit about as well ({:.3} and {:.3} mm); narrow the turn that is allowed",
                best_count, t.rms_mm, other.rms_mm
            ));
        }
    }
    let missing = (0..design.len())
        .filter(|i| !pairs.iter().any(|p| p.0 == *i))
        .collect();
    let extra = (0..measured.len())
        .filter(|j| !pairs.iter().any(|p| p.1 == *j))
        .collect();
    Ok(Pairing {
        pairs,
        missing,
        extra,
        turn_deg: t.theta_deg,
        dx: t.dx,
        dy: t.dy,
        rms_mm: t.rms_mm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A design with no symmetry.
    const DESIGN: [(f64, f64); 5] = [
        (0.0, 0.0),
        (80.0, 0.0),
        (80.0, 50.0),
        (10.0, 45.0),
        (35.0, 20.0),
    ];

    fn place(turn_deg: f64, dx: f64, dy: f64) -> Vec<(f64, f64)> {
        let (s, c) = turn_deg.to_radians().sin_cos();
        DESIGN
            .iter()
            .map(|p| (c * p.0 - s * p.1 + dx, s * p.0 + c * p.1 + dy))
            .collect()
    }

    #[test]
    fn a_board_turned_a_lot_is_paired_with_the_design() {
        for turn in [-170.0, -90.0, -35.0, 0.0, 27.0, 90.0, 133.0, 179.0] {
            let found = place(turn, 12.0, -7.0);
            // found in another order than the design
            let shuffled = [found[3], found[0], found[4], found[2], found[1]];
            let p = pair_turned(&DESIGN, &shuffled, &PairSettings::default()).unwrap();
            assert_eq!(
                p.pairs,
                vec![(0, 1), (1, 4), (2, 3), (3, 0), (4, 2)],
                "{turn}"
            );
            assert!((p.turn_deg - turn).abs() < 1e-6, "{turn}: {}", p.turn_deg);
            assert!(p.missing.is_empty() && p.extra.is_empty());
            assert!(p.rms_mm < 1e-6);
        }
    }

    #[test]
    fn a_missing_mark_and_an_extra_speck_are_told_from_the_marks() {
        let mut found = place(75.0, 5.0, 5.0);
        found.remove(2);
        found.push((33.3, 41.1));
        let p = pair_turned(&DESIGN, &found, &PairSettings::default()).unwrap();
        assert_eq!(p.missing, vec![2]);
        assert_eq!(p.extra, vec![4]);
        assert_eq!(p.pairs.len(), 4);
        let (d, m) = p.points(&DESIGN, &found);
        let t = fit(&d, &m, false).unwrap();
        assert!((t.theta_deg - 75.0).abs() < 1e-6);
    }

    #[test]
    fn what_cannot_be_told_apart_is_refused_instead_of_guessed() {
        // the four corners of a rectangle fit themselves when turned by a half turn
        let rect = [(0.0, 0.0), (80.0, 0.0), (80.0, 50.0), (0.0, 50.0)];
        let turned: Vec<(f64, f64)> = rect.iter().map(|p| (80.0 - p.0, 50.0 - p.1)).collect();
        let e = pair_turned(&rect, &turned, &PairSettings::default()).unwrap_err();
        assert!(e.starts_with(PAIR_AMBIGUOUS), "{e}");
        // narrowing the turn that is allowed tells them apart: the board is not turned half way round
        let narrow = PairSettings {
            max_turn_deg: 20.0,
            ..PairSettings::default()
        };
        let slightly: Vec<(f64, f64)> = rect.iter().map(|p| (p.0 + 1.0, p.1 - 1.0)).collect();
        let p = pair_turned(&rect, &slightly, &narrow).unwrap();
        assert_eq!(p.pairs, vec![(0, 0), (1, 1), (2, 2), (3, 3)]);
    }

    #[test]
    fn marks_that_do_not_make_the_design_are_refused_with_their_code() {
        let d = PairSettings::default();
        let found = place(10.0, 0.0, 0.0);
        let mut off = found.clone();
        off[1].0 += 20.0;
        off[2].1 -= 15.0;
        off[4].0 += 9.0;
        assert!(pair_turned(&DESIGN, &off, &d)
            .unwrap_err()
            .starts_with(PAIR_NONE));
        assert!(pair_turned(&DESIGN, &found[..2], &d)
            .unwrap_err()
            .starts_with(PAIR_FEW));
        let mut nan = found.clone();
        nan[0].0 = f64::NAN;
        assert!(pair_turned(&DESIGN, &nan, &d)
            .unwrap_err()
            .starts_with(PAIR_BAD_POINT));
        let far = [(1.0e7, 0.0); 5];
        assert!(pair_turned(&DESIGN, &far, &d)
            .unwrap_err()
            .starts_with(PAIR_BAD_POINT));
        let many = vec![(0.0, 0.0); MAX_MEASURED + 1];
        assert!(pair_turned(&DESIGN, &many, &d)
            .unwrap_err()
            .starts_with(PAIR_TOO_MANY));
        // a turn that is not allowed leaves no way
        let narrow = PairSettings {
            max_turn_deg: 5.0,
            ..d
        };
        let turned = place(90.0, 0.0, 0.0);
        assert!(pair_turned(&DESIGN, &turned, &narrow)
            .unwrap_err()
            .starts_with(PAIR_NONE));
    }

    #[test]
    fn the_settings_with_no_sense_are_refused() {
        let d = PairSettings::default();
        assert!(check(&d).is_ok());
        for bad in [
            PairSettings {
                tolerance_mm: 0.0,
                ..d
            },
            PairSettings {
                tolerance_mm: 51.0,
                ..d
            },
            PairSettings { min_marks: 1, ..d },
            PairSettings {
                min_marks: MAX_DESIGN + 1,
                ..d
            },
            PairSettings {
                max_turn_deg: 181.0,
                ..d
            },
            PairSettings {
                max_turn_deg: f64::NAN,
                ..d
            },
            PairSettings {
                max_stretch: 0.6,
                ..d
            },
        ] {
            assert!(check(&bad).is_err(), "{bad:?}");
        }
    }
}
