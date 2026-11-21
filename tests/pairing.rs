//! The marks of a board turned by any angle, with one missing and some specks extra, are paired with the design as the truth that drew them says, and whatever points are put in the pairing
//! never panics. The designs, the turns, the noise and the order come from a generator with a fixed seed that reads no clock, so a failure comes back the same way and says its seed and
//! its round. Only drawn boards: no photograph of a real board has been used.

use eh3d_vision_studio::align::fit;
use eh3d_vision_studio::sim::SimRng;
use eh3d_vision_studio::turnpair::{
    pair_turned, PairSettings, MAX_DESIGN, MAX_MEASURED, PAIR_AMBIGUOUS, PAIR_NONE,
};

const SEED: u64 = 0x5EED_0044;

/// A number in [0, 1).
fn unit(rng: &mut SimRng) -> f64 {
    rng.below(1_000_000) as f64 / 1_000_000.0
}

/// A design of `n` marks in a box of 150 by 100 mm, none closer than 8 mm to another (so they are told apart) and none at the middle of the others (which would make a symmetry).
fn design(rng: &mut SimRng, n: usize) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    while out.len() < n {
        let p = (unit(rng) * 150.0, unit(rng) * 100.0);
        if out.iter().all(|q| (q.0 - p.0).hypot(q.1 - p.1) >= 8.0) {
            out.push(p);
        }
    }
    out
}

struct Case {
    design: Vec<(f64, f64)>,
    found: Vec<(f64, f64)>,
    /// For each mark found, the index of the mark of the design that drew it, or `None` for a speck.
    truth: Vec<Option<usize>>,
    turn: f64,
}

fn case(seed: u64, i: u64) -> Case {
    let mut rng = SimRng::new(seed ^ (i + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let n = 4 + rng.below(5) as usize;
    let d = design(&mut rng, n);
    let turn = (unit(&mut rng) * 2.0 - 1.0) * 179.9;
    let (sn, cs) = turn.to_radians().sin_cos();
    let (dx, dy) = (unit(&mut rng) * 40.0 - 20.0, unit(&mut rng) * 40.0 - 20.0);
    let missing = if rng.below(2) == 0 {
        Some(rng.below(n as u64) as usize)
    } else {
        None
    };
    let mut found: Vec<(f64, f64)> = Vec::new();
    let mut truth: Vec<Option<usize>> = Vec::new();
    for (k, p) in d.iter().enumerate() {
        if missing == Some(k) {
            continue;
        }
        // a noise of up to 0.08 mm
        let (nx, ny) = ((unit(&mut rng) - 0.5) * 0.16, (unit(&mut rng) - 0.5) * 0.16);
        found.push((cs * p.0 - sn * p.1 + dx + nx, sn * p.0 + cs * p.1 + dy + ny));
        truth.push(Some(k));
    }
    for _ in 0..rng.below(3) {
        found.push((unit(&mut rng) * 200.0 - 25.0, unit(&mut rng) * 150.0 - 25.0));
        truth.push(None);
    }
    // the order in which the camera found them
    for k in (1..found.len()).rev() {
        let j = rng.below(k as u64 + 1) as usize;
        found.swap(k, j);
        truth.swap(k, j);
    }
    Case {
        design: d,
        found,
        truth,
        turn,
    }
}

#[test]
fn a_board_turned_a_lot_is_paired_with_the_design() {
    let s = PairSettings::default();
    let (mut paired, mut refused) = (0, 0);
    for i in 0..200 {
        let c = case(SEED, i);
        match pair_turned(&c.design, &c.found, &s) {
            Ok(p) => {
                paired += 1;
                // every pair is the pair of the truth
                for &(d, f) in &p.pairs {
                    assert_eq!(
                        c.truth[f],
                        Some(d),
                        "seed {SEED:#x} round {i}: mark {f} of the picture was drawn by {:?}, not {d}",
                        c.truth[f]
                    );
                }
                // and none that the truth has was left out
                let drawn = c.truth.iter().flatten().count();
                assert_eq!(p.pairs.len(), drawn, "seed {SEED:#x} round {i}");
                assert_eq!(p.missing.len(), c.design.len() - drawn);
                assert_eq!(p.extra.len(), c.found.len() - drawn);
                let mut diff = (p.turn_deg - c.turn) % 360.0;
                if diff > 180.0 {
                    diff -= 360.0;
                }
                if diff < -180.0 {
                    diff += 360.0;
                }
                assert!(
                    diff.abs() < 0.5,
                    "seed {SEED:#x} round {i}: turn off by {diff}"
                );
                assert!(
                    p.rms_mm < 0.15,
                    "seed {SEED:#x} round {i}: rms {}",
                    p.rms_mm
                );
                // the fit of the paired marks is the same board
                let (d, m) = p.points(&c.design, &c.found);
                let t = fit(&d, &m, false).unwrap();
                assert!((t.rms_mm - p.rms_mm).abs() < 0.2);
            }
            Err(e) => {
                refused += 1;
                assert!(
                    e.starts_with(PAIR_AMBIGUOUS) || e.starts_with(PAIR_NONE),
                    "seed {SEED:#x} round {i}: {e}"
                );
            }
        }
    }
    // a refusal is allowed, a wrong pairing is not, and most boards are paired
    assert!(paired >= 190, "paired {paired}, refused {refused}");
}

#[test]
fn the_same_seed_gives_the_same_pairing() {
    let s = PairSettings::default();
    let c = case(SEED, 7);
    assert_eq!(
        pair_turned(&c.design, &c.found, &s),
        pair_turned(&c.design, &c.found, &s)
    );
    let other = case(SEED, 8);
    assert_ne!(c.found, other.found);
}

#[test]
fn a_limit_on_the_turn_leaves_out_the_ways_that_turn_more() {
    let c = case(SEED, 11);
    let wide = PairSettings::default();
    let p = pair_turned(&c.design, &c.found, &wide).unwrap();
    let narrow = PairSettings {
        max_turn_deg: (p.turn_deg.abs() - 5.0).max(0.0),
        ..wide
    };
    // with the true turn forbidden, the answer is never the true one
    if let Ok(q) = pair_turned(&c.design, &c.found, &narrow) {
        assert_ne!(q.pairs, p.pairs);
    }
}

#[test]
fn points_of_any_kind_never_panic_and_the_work_stays_bounded() {
    let mut rng = SimRng::new(SEED ^ 0xBEEF);
    let weird = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0e300,
        -1.0e7,
        0.0,
        0.5,
        1.0e-300,
    ];
    let s = PairSettings::default();
    for round in 0..300 {
        let (nd, nm) = (rng.below(30) as usize, rng.below(60) as usize);
        let point = |rng: &mut SimRng| -> (f64, f64) {
            if rng.below(6) == 0 {
                (
                    weird[rng.below(weird.len() as u64) as usize],
                    weird[rng.below(weird.len() as u64) as usize],
                )
            } else {
                (unit(rng) * 100.0, unit(rng) * 100.0)
            }
        };
        let d: Vec<_> = (0..nd).map(|_| point(&mut rng)).collect();
        let m: Vec<_> = (0..nm).map(|_| point(&mut rng)).collect();
        match pair_turned(&d, &m, &s) {
            Ok(p) => {
                assert!(p.pairs.len() >= s.min_marks, "round {round}");
                assert!(p.turn_deg.is_finite() && p.rms_mm.is_finite());
            }
            Err(e) => assert!(e.starts_with("pair_") || e.contains("must be"), "{e}"),
        }
    }
    // a grid of the most marks the pairing takes, against the most marks found: many ways look alike, and it still ends with an answer or a refusal
    let grid: Vec<(f64, f64)> = (0..MAX_DESIGN)
        .map(|k| ((k % 6) as f64 * 10.0, (k / 6) as f64 * 10.0))
        .collect();
    let mut found: Vec<(f64, f64)> = grid.iter().map(|p| (p.0 + 3.0, p.1 - 2.0)).collect();
    while found.len() < MAX_MEASURED {
        found.push((unit(&mut rng) * 100.0, unit(&mut rng) * 100.0));
    }
    let narrow = PairSettings {
        max_turn_deg: 10.0,
        ..s
    };
    let p = pair_turned(&grid, &found, &narrow).unwrap();
    assert_eq!(p.pairs.len(), MAX_DESIGN);
    assert!((p.dx - 3.0).abs() < 1e-6 && (p.dy + 2.0).abs() < 1e-6);
}
