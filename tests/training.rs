//! A mark trained from an example is found again where the truth that drew it puts it, turned by the angle that drew it, and the files and rectangles that are put in whatever they are never
//! panic. The pictures are drawn here (a ring with a bar from its middle to one side and a dot on the other, so that it has no symmetry), with noise from a generator with a fixed seed that reads no clock, so
//! that a failure comes back the same way and says its seed and its round. No photograph of a real board has been used.

use eh3d_vision_studio::image::Gray;
use eh3d_vision_studio::sim::SimRng;
use eh3d_vision_studio::train::{
    ambiguity, locate, train, Rect, Template, TrainSettings, BAD_FILE, MAX_FILE, MAX_SAMPLES,
};

const SEED: u64 = 0x5EED_0043;

/// A number in [0, 1).
fn unit(rng: &mut SimRng) -> f64 {
    rng.below(1_000_000) as f64 / 1_000_000.0
}

/// The picture of a board with a mark at (cx, cy) turned by `turn` degrees clockwise on the screen: each pixel is the average of 3 by 3 points, so that the centre can lie between
/// pixels, and a noise of up to `noise` grey levels in each direction is added.
fn board(w: usize, h: usize, cx: f64, cy: f64, turn: f64, noise: u64, rng: &mut SimRng) -> Gray {
    let (s, c) = turn.to_radians().sin_cos();
    let mut data = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let mut dark = 0u32;
            for sy in 0..3 {
                for sx in 0..3 {
                    let (px, py) = (
                        x as f64 - 0.5 + (f64::from(sx) + 0.5) / 3.0,
                        y as f64 - 0.5 + (f64::from(sy) + 0.5) / 3.0,
                    );
                    // the point as seen from the mark, turned back
                    let (u, v) = (px - cx, py - cy);
                    let (a, b) = (u * c + v * s, -u * s + v * c);
                    let r = a.hypot(b);
                    let ring = (4.5..7.0).contains(&r);
                    let bar = (0.0..=9.5).contains(&a) && b.abs() <= 1.6;
                    let dot = (a + 2.5).hypot(b - 8.0) <= 2.0;
                    if ring || bar || dot {
                        dark += 1;
                    }
                }
            }
            let share = f64::from(dark) / 9.0;
            let mut v = 215.0 * (1.0 - share) + 40.0 * share;
            if noise > 0 {
                v += rng.below(2 * noise + 1) as f64 - noise as f64;
            }
            data.push(v.round().clamp(0.0, 255.0) as u8);
        }
    }
    Gray::new(w, h, data).unwrap()
}

fn around(cx: usize, cy: usize, half: usize) -> Rect {
    Rect {
        x: cx - half,
        y: cy - half,
        w: 2 * half + 1,
        h: 2 * half + 1,
    }
}

fn wrap_diff(a: f64, b: f64) -> f64 {
    let mut d = (a - b) % 360.0;
    if d > 180.0 {
        d -= 360.0;
    }
    if d <= -180.0 {
        d += 360.0;
    }
    d.abs()
}

/// What each round found against the truth: the error of the place in pixels and of the angle in degrees.
fn round(seed: u64, i: u64, turn_max: f64) -> (f64, f64, usize) {
    let mut rng = SimRng::new(seed ^ (i + 1).wrapping_mul(0x9E37_79B9));
    let (w, h) = (64usize, 64usize);
    // the example: the mark upright, at the middle of a picture of its own
    let example = board(49, 49, 24.0, 24.0, 0.0, 3, &mut rng);
    let s = TrainSettings {
        turn_max_deg: turn_max,
        turn_step_deg: 5.0,
        ..TrainSettings::default()
    };
    let t = train(&example, &[around(24, 24, 11)], &s).unwrap();
    // the truth: where and how much it lies in the picture
    let (cx, cy) = (26.0 + 12.0 * unit(&mut rng), 26.0 + 12.0 * unit(&mut rng));
    let turn = if turn_max > 0.0 {
        (unit(&mut rng) * 2.0 - 1.0) * turn_max.min(179.0)
    } else {
        0.0
    };
    let pic = board(w, h, cx, cy, turn, 3, &mut rng);
    let found = locate(&pic, &t, &s).unwrap();
    assert!(!found.is_empty(), "seed {seed:#x} round {i}: nothing found");
    let f = found[0];
    (
        (f.x - cx).hypot(f.y - cy),
        wrap_diff(f.angle_deg, turn),
        found.len(),
    )
}

#[test]
fn an_upright_mark_is_found_where_the_truth_puts_it() {
    for i in 0..20 {
        let (place, angle, n) = round(SEED, i, 0.0);
        assert!(place < 0.35, "seed {SEED:#x} round {i}: off by {place} px");
        assert!(
            angle < 1e-9,
            "no turn is searched, so the angle is 0: {angle}"
        );
        assert_eq!(n, 1, "round {i}: {n} marks");
    }
}

#[test]
fn a_turned_mark_is_found_with_the_place_and_the_angle_of_the_truth() {
    let (mut worst_place, mut worst_angle) = (0.0f64, 0.0f64);
    for i in 0..16 {
        let (place, angle, n) = round(SEED, i, 180.0);
        worst_place = worst_place.max(place);
        worst_angle = worst_angle.max(angle);
        assert!(place < 0.6, "seed {SEED:#x} round {i}: off by {place} px");
        assert!(
            angle < 3.0,
            "seed {SEED:#x} round {i}: turn off by {angle} degrees"
        );
        assert_eq!(n, 1, "round {i}: {n} marks");
    }
    // the tolerance is a bound, not a target: say how near the worst round came
    assert!(
        worst_place < 0.6 && worst_angle < 3.0,
        "{worst_place} {worst_angle}"
    );
}

#[test]
fn the_same_seed_gives_the_same_answer() {
    assert_eq!(round(SEED, 3, 180.0), round(SEED, 3, 180.0));
    assert_ne!(round(SEED, 3, 180.0), round(SEED, 4, 180.0));
}

#[test]
fn two_marks_in_a_picture_are_two_answers_and_a_board_without_the_mark_is_none() {
    let mut rng = SimRng::new(SEED);
    let example = board(49, 49, 24.0, 24.0, 0.0, 2, &mut rng);
    let s = TrainSettings::default();
    let t = train(&example, &[around(24, 24, 11)], &s).unwrap();
    let mut two = board(100, 50, 28.0, 25.0, 0.0, 0, &mut rng);
    let other = board(100, 50, 70.0, 24.0, 0.0, 0, &mut rng);
    // put the second mark into the first picture: the darker pixel of the two
    for (a, b) in two.data.iter_mut().zip(&other.data) {
        *a = (*a).min(*b);
    }
    let found = locate(&two, &t, &s).unwrap();
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].score >= found[1].score);
    let empty = Gray::blank(100, 50, 215).unwrap();
    assert!(locate(&empty, &t, &s).unwrap().is_empty());
}

#[test]
fn a_sharp_mark_is_less_ambiguous_than_a_ramp_and_a_stripe() {
    let mut rng = SimRng::new(SEED);
    let pic = board(49, 49, 24.0, 24.0, 0.0, 0, &mut rng);
    let t = train(&pic, &[around(24, 24, 11)], &TrainSettings::default()).unwrap();
    let mark = ambiguity(&t.data, t.w, t.h);
    let ramp: Vec<u8> = (0..21 * 21).map(|i| ((i % 21) * 12) as u8).collect();
    let stripe: Vec<u8> = (0..21 * 21)
        .map(|i| if (8..13).contains(&(i % 21)) { 30 } else { 200 })
        .collect();
    assert!(mark < 0.9, "{mark}");
    assert!(ambiguity(&ramp, 21, 21) > 0.99);
    assert!(ambiguity(&stripe, 21, 21) > 0.99);
    assert_eq!(ambiguity(&[1, 2, 3], 2, 2), 1.0);
}

/// A byte file that is mostly the file of a template and a little wrong.
fn mutate(rng: &mut SimRng, good: &[u8]) -> Vec<u8> {
    let mut v = good.to_vec();
    for _ in 0..=rng.below(4) {
        match rng.below(5) {
            0 if !v.is_empty() => {
                let i = rng.below(v.len() as u64) as usize;
                v[i] = rng.below(256) as u8;
            }
            1 if !v.is_empty() => v.truncate(rng.below(v.len() as u64) as usize),
            2 => v.extend((0..rng.below(40)).map(|_| rng.below(256) as u8)),
            3 if !v.is_empty() => {
                let i = rng.below(v.len() as u64) as usize;
                v.remove(i);
            }
            _ => v.insert(
                rng.below(v.len() as u64 + 1) as usize,
                b"0123456789 \n-"[rng.below(13) as usize],
            ),
        }
    }
    v
}

#[test]
fn files_and_rectangles_of_any_shape_never_panic() {
    let mut rng = SimRng::new(SEED ^ 0xF00D);
    let pic = board(49, 49, 24.0, 24.0, 0.0, 2, &mut rng);
    let t = train(&pic, &[around(24, 24, 11)], &TrainSettings::default()).unwrap();
    let good = t.to_bytes();
    let mut accepted = 0;
    for round in 0..600 {
        let bytes = match round % 3 {
            0 => mutate(&mut rng, &good),
            1 => (0..rng.below(300)).map(|_| rng.below(256) as u8).collect(),
            _ => {
                // a header that lies about the size, with a body of a random length
                let mut v = format!(
                    "EH3DMARK 1\n{} {} {}\n",
                    rng.below(400),
                    rng.below(400),
                    rng.below(100)
                )
                .into_bytes();
                v.extend((0..rng.below(2000)).map(|_| rng.below(256) as u8));
                v
            }
        };
        match Template::from_bytes(&bytes) {
            Ok(t) => {
                accepted += 1;
                assert_eq!(t.data.len(), t.w * t.h, "round {round}");
                assert!(bytes.len() <= MAX_FILE);
                assert!((1..=MAX_SAMPLES).contains(&t.samples));
                // what was read is what is written back
                assert_eq!(Template::from_bytes(&t.to_bytes()).unwrap(), t);
            }
            Err(e) => assert!(e.contains(BAD_FILE) || e.contains("sample_flat"), "{e}"),
        }
    }
    assert!(accepted < 600);
    // rectangles anywhere, including ones whose arithmetic could overflow, on pictures of any size
    let s = TrainSettings::default();
    for round in 0..400 {
        let (w, h) = (rng.below(60) as usize + 1, rng.below(60) as usize + 1);
        let data: Vec<u8> = (0..w * h).map(|_| rng.below(256) as u8).collect();
        let img = Gray::new(w, h, data).unwrap();
        let edge = [0u64, 1, 7, 8, 9, 60, 1 << 31, u64::from(u32::MAX), 1 << 40];
        let pick = |rng: &mut SimRng| -> usize {
            if rng.below(2) == 0 {
                rng.below(70) as usize
            } else {
                // a value that does not fit 32 bits is cut by the `as`, which is what a 32-bit build would meet
                edge[rng.below(edge.len() as u64) as usize] as usize
            }
        };
        let n = rng.below(4) as usize;
        let rects: Vec<Rect> = (0..n)
            .map(|_| Rect {
                x: pick(&mut rng),
                y: pick(&mut rng),
                w: pick(&mut rng),
                h: pick(&mut rng),
            })
            .collect();
        match train(&img, &rects, &s) {
            Ok(t) => {
                assert_eq!(t.data.len(), t.w * t.h, "round {round}");
                // and a template that was trained can be searched for in a picture of any size
                let found = locate(&img, &t, &s);
                if let Ok(v) = found {
                    assert!(v.iter().all(|f| f.score >= s.min_score && f.x.is_finite()));
                }
            }
            Err(e) => assert!(e.contains('_'), "every refusal has a code: {e}"),
        }
    }
}
