//! The holes of the edge of a tape of parts (the round holes that the sprocket of a feeder pulls by) found in a picture, and from them where the pockets are: the line the holes lie on, the
//! pitch between them against the pitch of the tape, how far each hole is from the line and how big it is against its size. The place where a part is picked is a pocket, and the pocket
//! is found from the holes because the holes are the part of a tape that is cut to a norm and easy to see.
//!
//! Nothing here moves a feeder: it says where the holes and the pockets are, how well the tape advanced (the pitch measured against the pitch of the tape) and why a picture is not to be
//! trusted (too few holes, a hole missing, holes off the line, a pitch that is not the pitch, a hole of the wrong size). Every limit is a setting (the `tape_*` keys), every reason has a
//! code that does not change, and the work is bounded by the number of pixels and of holes. It is tried on pictures drawn by the tests; no tape has been photographed.

use crate::fiducial::{find, Polarity, Settings};
use crate::image::Gray;

/// The code of the refusal of a picture with more pixels than the setting allows.
pub const TOO_BIG: &str = "tape_too_big";
/// The code of the refusal of a picture with more holes than the setting allows.
pub const TOO_MANY: &str = "tape_too_many_holes";
/// The code of the refusal of the size of a pixel that is not known.
pub const NO_SCALE: &str = "tape_no_scale";

/// Reasons a picture is not to be trusted, in the order they are listed.
pub const FEW_HOLES: &str = "tape_few_holes";
pub const HOLE_MISSING: &str = "tape_hole_missing";
pub const OFF_LINE: &str = "tape_off_line";
pub const PITCH: &str = "tape_pitch";
pub const HOLE_SIZE: &str = "tape_hole_size";

/// Every reason, in order.
pub const REASONS: [&str; 5] = [FEW_HOLES, HOLE_MISSING, OFF_LINE, PITCH, HOLE_SIZE];

/// What is asked of the search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TapeSettings {
    /// The diameter of the holes and the pitch between them, in millimetres (the norm of a tape of 8 mm has holes of 1.5 mm at 4 mm).
    pub hole_mm: f64,
    pub pitch_mm: f64,
    /// How far the pitch measured may be from the pitch of the tape, and how far a hole may lie from the line of the holes, in millimetres.
    pub tolerance_mm: f64,
    pub line_tolerance_mm: f64,
    /// How far the size of a hole may be from its size, as a share (0 to 1).
    pub size_tolerance: f64,
    /// The least holes that must be found, and the most that are accepted.
    pub min_holes: usize,
    pub max_holes: usize,
    /// The pitch of the pockets, and where the first is from a hole: along the tape and across it (positive to the right of the way the tape goes, the picture having y down), in millimetres.
    pub pocket_pitch_mm: f64,
    pub pocket_along_mm: f64,
    pub pocket_across_mm: f64,
    /// Whether the holes are darker than the tape, and the most pixels of a picture.
    pub polarity: Polarity,
    pub max_pixels: usize,
}

impl Default for TapeSettings {
    fn default() -> Self {
        TapeSettings {
            hole_mm: 1.5,
            pitch_mm: 4.0,
            tolerance_mm: 0.1,
            line_tolerance_mm: 0.15,
            size_tolerance: 0.25,
            min_holes: 4,
            max_holes: 400,
            pocket_pitch_mm: 4.0,
            pocket_along_mm: 2.0,
            pocket_across_mm: 3.5,
            polarity: Polarity::Dark,
            max_pixels: 25_000_000,
        }
    }
}

/// Whether the settings make sense; the reason when they do not.
pub fn check(s: &TapeSettings) -> Result<(), String> {
    let range = |v: f64, lo: f64, hi: f64| v.is_finite() && v >= lo && v <= hi;
    if !range(s.hole_mm, 0.2, 20.0) || !range(s.pitch_mm, 0.5, 50.0) {
        return Err("the hole must be from 0.2 to 20 mm and the pitch from 0.5 to 50 mm".into());
    }
    if s.hole_mm >= s.pitch_mm {
        return Err("the hole of a tape must be smaller than the pitch".into());
    }
    if !range(s.tolerance_mm, 0.001, 5.0) || !range(s.line_tolerance_mm, 0.001, 5.0) {
        return Err("the tolerances of the tape must be from 0.001 to 5 mm".into());
    }
    if !range(s.size_tolerance, 0.0, 1.0) {
        return Err("the tolerance of the size of a hole must be from 0 to 1".into());
    }
    if !(2..=s.max_holes).contains(&s.min_holes) || s.max_holes > 10_000 {
        return Err(
            "the holes of the tape must be at least 2 and at most 10000, with the least not above the most"
                .into(),
        );
    }
    if !range(s.pocket_pitch_mm, 0.5, 100.0)
        || !range(s.pocket_along_mm, -100.0, 100.0)
        || !range(s.pocket_across_mm, -100.0, 100.0)
    {
        return Err("the pockets of the tape are out of range".into());
    }
    if !(64..=25_000_000).contains(&s.max_pixels) {
        return Err("the most pixels of the tape search must be from 64 to 25000000".into());
    }
    Ok(())
}

/// A hole found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hole {
    /// The centre in pixels.
    pub x: f64,
    pub y: f64,
    /// The diameter in millimetres.
    pub diameter_mm: f64,
}

/// What was found in a picture of a tape.
#[derive(Debug, Clone, PartialEq)]
pub struct TapeReport {
    /// The holes, in the order along the tape.
    pub holes: Vec<Hole>,
    /// The way the tape goes in the picture, in degrees from the x axis turning toward y (down).
    pub angle_deg: f64,
    /// The pitch measured (the mean distance between neighbouring holes, counting a gap of several pitches as that many) and how far it is from the pitch of the tape.
    pub pitch_mm: f64,
    pub pitch_error_mm: f64,
    /// The farthest a hole lies from the line of the holes.
    pub line_error_mm: f64,
    /// The pockets in pixels, row after row along the tape, and the index of the one nearest to the centre of the picture, where a part would be picked.
    pub pockets: Vec<(f64, f64)>,
    pub pick: Option<usize>,
    /// The codes of what is wrong; empty when the picture can be trusted.
    pub reasons: Vec<&'static str>,
    /// The size of a pixel that was used, in millimetres.
    pub mm_per_px: f64,
}

impl TapeReport {
    /// Whether the picture can be trusted.
    pub fn good(&self) -> bool {
        self.reasons.is_empty()
    }
}

/// Finds the holes of a tape in a picture and the pockets that go with them.
pub fn locate(img: &Gray, mm_per_px: f64, s: &TapeSettings) -> Result<TapeReport, String> {
    check(s)?;
    if !(mm_per_px.is_finite() && mm_per_px > 0.0 && mm_per_px < 100.0) {
        return Err(format!(
            "{NO_SCALE}: the size of a pixel in millimetres is needed (--mm-per-px or mm_per_px)"
        ));
    }
    if img.data.len() != img.w * img.h || (img.w as u64) * (img.h as u64) > s.max_pixels as u64 {
        return Err(format!(
            "{TOO_BIG}: {} by {} pixels is more than the {} allowed",
            img.w, img.h, s.max_pixels
        ));
    }
    let d = s.hole_mm / mm_per_px;
    let search = Settings {
        polarity: s.polarity,
        min_diameter: (d * 0.5).max(2.0),
        max_diameter: (d * 1.6).max(3.0),
        min_fill: 0.75,
        max_elongation: 1.4,
    };
    let found = find(img, &search)?;
    if found.len() > s.max_holes {
        return Err(format!(
            "{TOO_MANY}: {} holes found, more than the {} allowed",
            found.len(),
            s.max_holes
        ));
    }
    let n = found.len();
    let mut reasons: Vec<&'static str> = Vec::new();
    if n < s.min_holes {
        reasons.push(FEW_HOLES);
        return Ok(TapeReport {
            holes: found
                .iter()
                .map(|f| Hole {
                    x: f.x,
                    y: f.y,
                    diameter_mm: f.diameter * mm_per_px,
                })
                .collect(),
            angle_deg: 0.0,
            pitch_mm: 0.0,
            pitch_error_mm: 0.0,
            line_error_mm: 0.0,
            pockets: Vec::new(),
            pick: None,
            reasons,
            mm_per_px,
        });
    }
    // the line through the holes (total least squares)
    let (mx, my) = (
        found.iter().map(|f| f.x).sum::<f64>() / n as f64,
        found.iter().map(|f| f.y).sum::<f64>() / n as f64,
    );
    let (mut sxx, mut sxy, mut syy) = (0.0, 0.0, 0.0);
    for f in &found {
        let (dx, dy) = (f.x - mx, f.y - my);
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
    }
    let theta = 0.5 * (2.0 * sxy).atan2(sxx - syy);
    let (mut ux, mut uy) = (theta.cos(), theta.sin());
    if ux < -1e-9 || (ux.abs() <= 1e-9 && uy < 0.0) {
        ux = -ux;
        uy = -uy;
    }
    let mut along: Vec<(f64, f64, usize)> = found
        .iter()
        .enumerate()
        .map(|(i, f)| {
            (
                (f.x - mx) * ux + (f.y - my) * uy,
                (f.y - my) * ux - (f.x - mx) * uy,
                i,
            )
        })
        .collect();
    along.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let line_error = along
        .iter()
        .map(|a| a.1.abs() * mm_per_px)
        .fold(0.0f64, f64::max);
    // the gaps: a gap of k pitches means k - 1 holes were not found
    let (mut steps, mut missing, mut worst_gap) = (0.0f64, false, 0.0f64);
    for pair in along.windows(2) {
        let gap = (pair[1].0 - pair[0].0) * mm_per_px;
        let k = (gap / s.pitch_mm).round().max(1.0);
        if k > 1.0 {
            missing = true;
        }
        steps += k;
        worst_gap = worst_gap.max((gap - k * s.pitch_mm).abs());
    }
    let span = (along[n - 1].0 - along[0].0) * mm_per_px;
    let pitch = span / steps;
    let pitch_error = pitch - s.pitch_mm;
    if missing {
        reasons.push(HOLE_MISSING);
    }
    if line_error > s.line_tolerance_mm {
        reasons.push(OFF_LINE);
    }
    if pitch_error.abs() > s.tolerance_mm || worst_gap > s.tolerance_mm * 2.0 {
        reasons.push(PITCH);
    }
    let holes: Vec<Hole> = along
        .iter()
        .map(|a| {
            let f = &found[a.2];
            Hole {
                x: f.x,
                y: f.y,
                diameter_mm: f.diameter * mm_per_px,
            }
        })
        .collect();
    if holes
        .iter()
        .any(|h| (h.diameter_mm - s.hole_mm).abs() > s.hole_mm * s.size_tolerance)
    {
        reasons.push(HOLE_SIZE);
    }
    // the pockets: from the first hole, along the tape by the pitch of the pockets and across it by the offset; the normal points to the right of the way the tape goes
    let (nx, ny) = (-uy, ux);
    let first = &along[0];
    let (hx, hy) = (found[first.2].x, found[first.2].y);
    let reach = (along[n - 1].0 - first.0) * mm_per_px;
    let count = if reach >= s.pocket_along_mm {
        ((reach - s.pocket_along_mm) / s.pocket_pitch_mm).floor() as usize + 1
    } else {
        0
    };
    let count = count.min(s.max_holes);
    let mut pockets = Vec::with_capacity(count);
    for k in 0..count {
        let t = (s.pocket_along_mm + k as f64 * s.pocket_pitch_mm) / mm_per_px;
        let c = s.pocket_across_mm / mm_per_px;
        pockets.push((hx + t * ux + c * nx, hy + t * uy + c * ny));
    }
    let centre = (img.w as f64 / 2.0, img.h as f64 / 2.0);
    let pick = pockets
        .iter()
        .enumerate()
        .min_by(|a, b| {
            let da = (a.1 .0 - centre.0).hypot(a.1 .1 - centre.1);
            let db = (b.1 .0 - centre.0).hypot(b.1 .1 - centre.1);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i);
    Ok(TapeReport {
        holes,
        angle_deg: uy.atan2(ux).to_degrees(),
        pitch_mm: pitch,
        pitch_error_mm: pitch_error,
        line_error_mm: line_error,
        pockets,
        pick,
        reasons,
        mm_per_px,
    })
}

/// The report as JSON with keys that do not change.
pub fn to_json(r: &TapeReport) -> String {
    let holes: Vec<String> = r
        .holes
        .iter()
        .map(|h| {
            format!(
                "{{\"x\":{:.3},\"y\":{:.3},\"diameter_mm\":{:.4}}}",
                h.x, h.y, h.diameter_mm
            )
        })
        .collect();
    let pockets: Vec<String> = r
        .pockets
        .iter()
        .map(|p| format!("{{\"x\":{:.3},\"y\":{:.3}}}", p.0, p.1))
        .collect();
    let reasons: Vec<String> = r.reasons.iter().map(|c| format!("\"{c}\"")).collect();
    let pick = r.pick.map_or("null".to_string(), |i| i.to_string());
    format!(
        "{{\"command\":\"tape-locate\",\"mm_per_px\":{:.6},\"angle_deg\":{:.3},\"pitch_mm\":{:.4},\"pitch_error_mm\":{:.4},\"line_error_mm\":{:.4},\"holes\":[{}],\"pockets\":[{}],\"pick\":{},\"good\":{},\"reasons\":[{}]}}\n",
        r.mm_per_px,
        r.angle_deg,
        r.pitch_mm,
        r.pitch_error_mm,
        r.line_error_mm,
        holes.join(","),
        pockets.join(","),
        pick,
        r.good(),
        reasons.join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tape along x at `y` with holes of 1.5 mm every `pitch` mm at 0.05 mm a pixel, the first at `x0`; the ones in `skip` are not drawn.
    fn tape(pitch: f64, tilt: f64, skip: &[usize], count: usize) -> Gray {
        let mut g = Gray::blank(480, 200, 190).unwrap();
        let mmpp = 0.05;
        for k in 0..count {
            if skip.contains(&k) {
                continue;
            }
            let t = 30.0 + k as f64 * pitch / mmpp;
            g.disc(t, 60.0 + t * tilt, 0.75 / mmpp, 20);
        }
        g
    }

    #[test]
    fn the_holes_pitch_and_pockets_of_a_tape_are_found() {
        let r = locate(&tape(4.0, 0.0, &[], 6), 0.05, &TapeSettings::default()).unwrap();
        assert!(r.good(), "{:?}", r.reasons);
        assert_eq!(r.holes.len(), 6);
        assert!(r.pitch_error_mm.abs() < 0.01, "{}", r.pitch_error_mm);
        assert!(r.angle_deg.abs() < 0.1);
        assert!(r.line_error_mm < 0.03);
        // the first pocket is 2 mm along and 3.5 mm to the right (down in the picture)
        let (x, y) = r.pockets[0];
        assert!(
            (x - (30.0 + 40.0)).abs() < 0.6 && (y - (60.0 + 70.0)).abs() < 0.6,
            "{x} {y}"
        );
        assert!(r.pick.is_some());
    }

    #[test]
    fn a_tilted_tape_gives_its_angle() {
        let r = locate(&tape(4.0, 0.1, &[], 6), 0.05, &TapeSettings::default()).unwrap();
        assert!(
            (r.angle_deg - 0.1f64.atan().to_degrees()).abs() < 0.2,
            "{}",
            r.angle_deg
        );
        assert!(r.good(), "{:?}", r.reasons);
    }

    #[test]
    fn a_wrong_pitch_a_missing_hole_and_few_holes_are_told_apart() {
        let s = TapeSettings::default();
        let wide = locate(&tape(4.3, 0.0, &[], 6), 0.05, &s).unwrap();
        assert_eq!(wide.reasons, [PITCH]);
        let gap = locate(&tape(4.0, 0.0, &[2], 6), 0.05, &s).unwrap();
        assert_eq!(gap.reasons, [HOLE_MISSING]);
        assert!(gap.pitch_error_mm.abs() < 0.01);
        let few = locate(&tape(4.0, 0.0, &[], 3), 0.05, &s).unwrap();
        assert_eq!(few.reasons, [FEW_HOLES]);
        assert!(few.pockets.is_empty() && few.pick.is_none());
    }

    #[test]
    fn a_hole_off_the_line_is_told() {
        let mut g = tape(4.0, 0.0, &[2], 6);
        g.disc(30.0 + 2.0 * 80.0, 60.0 + 6.0, 15.0, 20);
        let r = locate(&g, 0.05, &TapeSettings::default()).unwrap();
        assert!(r.reasons.contains(&OFF_LINE), "{:?}", r.reasons);
    }

    #[test]
    fn a_picture_without_a_scale_or_too_big_is_refused_with_its_code() {
        let g = tape(4.0, 0.0, &[], 6);
        let s = TapeSettings::default();
        assert!(locate(&g, 0.0, &s).unwrap_err().starts_with(NO_SCALE));
        assert!(locate(&g, f64::NAN, &s).unwrap_err().starts_with(NO_SCALE));
        let small = TapeSettings {
            max_pixels: 64,
            ..s
        };
        assert!(locate(&g, 0.05, &small).unwrap_err().starts_with(TOO_BIG));
        let few = TapeSettings {
            min_holes: 2,
            max_holes: 3,
            ..s
        };
        assert!(locate(&g, 0.05, &few).unwrap_err().starts_with(TOO_MANY));
    }

    #[test]
    fn settings_that_make_no_sense_are_refused() {
        let d = TapeSettings::default();
        for bad in [
            TapeSettings { hole_mm: 5.0, ..d },
            TapeSettings {
                pitch_mm: f64::NAN,
                ..d
            },
            TapeSettings {
                tolerance_mm: 0.0,
                ..d
            },
            TapeSettings {
                size_tolerance: 2.0,
                ..d
            },
            TapeSettings { min_holes: 1, ..d },
            TapeSettings {
                max_holes: 20_000,
                ..d
            },
            TapeSettings {
                pocket_pitch_mm: 0.0,
                ..d
            },
            TapeSettings { max_pixels: 1, ..d },
        ] {
            assert!(check(&bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_json_carries_the_verdict_and_the_pick() {
        let r = locate(&tape(4.0, 0.0, &[], 6), 0.05, &TapeSettings::default()).unwrap();
        let j = to_json(&r);
        assert!(j.contains("\"good\":true,\"reasons\":[]"), "{j}");
        assert!(j.contains("\"pick\":"));
    }
}
