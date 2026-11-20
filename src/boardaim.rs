//! Putting a board where its design says: look, move, look again. The camera looks at the fiducials of the board from a pose that does not change, the turn and the shift that the board is off
//! by are worked out, the stage is told to move the board by the opposite (never more than `align_max_step_mm` and `align_max_step_deg` at a time, never more than `align_max_total_mm` in all),
//! and the camera looks again at the same pose, after waiting for frames that were taken after the move. This goes on until the board is inside the tolerance or the steps are used up. Nothing
//! is added to the answer that the person did not write: the offset of the camera is the one of the configuration.
//!
//! When anything is wrong the program moves toward the safest state and never back toward the less safe one without a person: if the error grows after a move the step is halved, and if it grows
//! again, or the marks are not found, the camera gives black or repeated frames, a limit is reached or the stage fails, the board is not moved any more. The order of the states is
//! `Normal`, `Reduced`, `Hold`, `Abort`; `degrade` never returns a state below the one it was given.

use crate::align::{fit, pair_nearest, Transform};
use crate::capture::health::{fingerprint, mean, HealthSettings};
use crate::capture::log::{Level, Log};
use crate::capture::settle::{settle, SettleSettings};
use crate::capture::{Camera, CapError, Code};
use crate::charuco::solve::CameraOffset;
use crate::fiducial::{find_with, Settings as MarkSettings};

/// The keys of the configuration that the errors of this module name.
pub const KEYS_NAMED: &[&str] = &[
    "align_tolerance_mm",
    "align_tolerance_deg",
    "align_max_step_mm",
    "align_max_step_deg",
    "align_max_total_mm",
    "align_max_steps",
    "align_gain",
];

/// What is asked of the alignment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AimSettings {
    /// The board is aligned when it is off by no more than this in millimetres and in degrees.
    pub tolerance_mm: f64,
    pub tolerance_deg: f64,
    /// The most that one move may shift (millimetres) and turn (degrees).
    pub max_step_mm: f64,
    pub max_step_deg: f64,
    /// The most that all the moves together may shift, in millimetres.
    pub max_total_mm: f64,
    /// The most moves; there is one look more, to see the result of the last.
    pub max_steps: usize,
    /// The share of the correction that is asked of each move (0.1 to 1).
    pub gain: f64,
}

impl Default for AimSettings {
    fn default() -> Self {
        AimSettings {
            tolerance_mm: 0.05,
            tolerance_deg: 0.1,
            max_step_mm: 2.0,
            max_step_deg: 2.0,
            max_total_mm: 10.0,
            max_steps: 6,
            gain: 1.0,
        }
    }
}

/// Whether the values can be used.
pub fn check(s: &AimSettings) -> Result<(), CapError> {
    let bad =
        |key: &'static str, why: &str| Err(CapError::with_key(Code::AlignStepLimit, key, why));
    let pos = |v: f64, max: f64| v.is_finite() && v > 0.0 && v <= max;
    if !pos(s.tolerance_mm, 5.0) {
        return bad(
            "align_tolerance_mm",
            "it must be more than 0 and at most 5 millimetres",
        );
    }
    if !pos(s.tolerance_deg, 5.0) {
        return bad(
            "align_tolerance_deg",
            "it must be more than 0 and at most 5 degrees",
        );
    }
    if !pos(s.max_step_mm, 50.0) {
        return bad(
            "align_max_step_mm",
            "it must be more than 0 and at most 50 millimetres",
        );
    }
    if !pos(s.max_step_deg, 45.0) {
        return bad(
            "align_max_step_deg",
            "it must be more than 0 and at most 45 degrees",
        );
    }
    if !pos(s.max_total_mm, 500.0) {
        return bad(
            "align_max_total_mm",
            "it must be more than 0 and at most 500 millimetres",
        );
    }
    if !(1..=50).contains(&s.max_steps) {
        return bad("align_max_steps", "it must be from 1 to 50");
    }
    if !(s.gain.is_finite() && (0.1..=1.0).contains(&s.gain)) {
        return bad("align_gain", "it must be from 0.1 to 1");
    }
    Ok(())
}

/// What the program may do, from the freest to the safest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mode {
    /// Whole steps.
    Normal,
    /// Half steps.
    Reduced,
    /// No move: the board stays where it is and a person is told.
    Hold,
    /// No move and no more looks.
    Abort,
}

impl Mode {
    /// Every mode, from the freest to the safest.
    pub const ALL: [Mode; 4] = [Mode::Normal, Mode::Reduced, Mode::Hold, Mode::Abort];
}

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event {
    /// Nothing wrong.
    Fine,
    /// The error is bigger after the move.
    Diverging,
    /// The marks of the board were not found or do not fit the design.
    NoMarks,
    /// The camera gives frames that cannot be trusted.
    CameraBad,
    /// A limit of movement was reached.
    LimitReached,
    /// The stage refused or failed a move.
    StageFailed,
}

impl Event {
    /// Every event.
    pub const ALL: [Event; 6] = [
        Event::Fine,
        Event::Diverging,
        Event::NoMarks,
        Event::CameraBad,
        Event::LimitReached,
        Event::StageFailed,
    ];
}

/// The mode after an event. It is never below the mode before: nothing gets freer by itself.
pub fn degrade(mode: Mode, event: Event) -> Mode {
    let target = match event {
        Event::Fine => Mode::Normal,
        Event::Diverging => match mode {
            Mode::Normal => Mode::Reduced,
            _ => Mode::Hold,
        },
        Event::NoMarks | Event::LimitReached => Mode::Hold,
        Event::CameraBad | Event::StageFailed => Mode::Abort,
    };
    mode.max(target)
}

/// The most that a move may shift and turn in a mode; nothing when the mode does not move.
pub fn allowance(mode: Mode, s: &AimSettings) -> (f64, f64) {
    match mode {
        Mode::Normal => (s.max_step_mm, s.max_step_deg),
        Mode::Reduced => (s.max_step_mm / 2.0, s.max_step_deg / 2.0),
        Mode::Hold | Mode::Abort => (0.0, 0.0),
    }
}

/// Something that moves the board.
pub trait Stage {
    /// Moves the board by a turn of `dtheta_deg` about the origin of the machine and then a shift of `(dx, dy)` millimetres.
    fn move_by(&mut self, dx: f64, dy: f64, dtheta_deg: f64) -> Result<(), CapError>;
}

/// What is known about the board and the camera.
#[derive(Debug, Clone)]
pub struct Env {
    /// The fiducials of the board in the frame of the design, in millimetres.
    pub design: Vec<(f64, f64)>,
    /// Millimetres for a pixel, from the calibration.
    pub mm_per_px: f64,
    /// The offset of the camera, from the configuration.
    pub offset: CameraOffset,
    pub marks: MarkSettings,
    pub threshold: Option<u8>,
    /// The most that the marks may be off the design once it is fitted, in millimetres; more is a mark that is not the right one.
    pub max_fit_rms_mm: f64,
    pub settle: SettleSettings,
    pub health: HealthSettings,
}

impl Env {
    /// An environment for the design and the scale with the usual settings and no offset.
    pub fn new(design: Vec<(f64, f64)>, mm_per_px: f64) -> Env {
        Env {
            design,
            mm_per_px,
            offset: CameraOffset::default(),
            marks: MarkSettings::default(),
            threshold: None,
            max_fit_rms_mm: 0.2,
            settle: SettleSettings::default(),
            health: HealthSettings::default(),
        }
    }
}

/// One look and, when there was one, the move that followed.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub number: usize,
    /// How far the board was, in millimetres and degrees.
    pub error_mm: f64,
    pub error_deg: f64,
    /// The move that was asked, `(dx, dy, dtheta)`; `None` for the last look.
    pub command: Option<(f64, f64, f64)>,
    /// Whether the move was cut down to the limit of a step.
    pub clamped: bool,
    pub mode: Mode,
}

/// How the alignment ended.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub result: Result<(), CapError>,
    pub steps: Vec<Step>,
    /// Where the board was at the last look.
    pub last_pose: Option<Transform>,
    /// The sum of the lengths of the moves, in millimetres, and of the turns, in degrees.
    pub moved_mm: f64,
    pub moved_deg: f64,
    pub mode: Mode,
    /// What the looks prove: `simulator` when the camera is the simulated one.
    pub evidence: &'static str,
}

/// The pose of the board in the frame of the machine, from a frame: where the fiducials are, paired with the design.
pub fn locate(frame: &crate::image::Gray, env: &Env) -> Result<Transform, CapError> {
    let marks = find_with(frame, &env.marks, env.threshold)
        .map_err(|e| CapError::new(Code::AlignNoMarks, e))?;
    if marks.len() != env.design.len() {
        return Err(CapError::new(
            Code::AlignNoMarks,
            format!(
                "{} marks found, {} in the design",
                marks.len(),
                env.design.len()
            ),
        ));
    }
    let (cx, cy) = (frame.w as f64 / 2.0, frame.h as f64 / 2.0);
    let measured: Vec<(f64, f64)> = marks
        .iter()
        .map(|m| {
            env.offset
                .apply(((m.x - cx) * env.mm_per_px, (m.y - cy) * env.mm_per_px))
        })
        .collect();
    let paired =
        pair_nearest(&env.design, &measured).map_err(|e| CapError::new(Code::AlignNoMarks, e))?;
    let t = fit(&env.design, &paired, false).map_err(|e| CapError::new(Code::AlignNoMarks, e))?;
    if t.rms_mm > env.max_fit_rms_mm {
        return Err(CapError::new(
            Code::AlignNoMarks,
            format!(
                "the marks fit the design to {:.3} mm, {} allowed",
                t.rms_mm, env.max_fit_rms_mm
            ),
        ));
    }
    Ok(t)
}

/// The move that puts the board where the design says: the opposite of its pose, as a turn about the origin and then a shift.
pub fn correction(pose: &Transform) -> (f64, f64, f64) {
    let (s, c) = pose.theta_deg.to_radians().sin_cos();
    (
        -(c * pose.dx + s * pose.dy),
        -(-s * pose.dx + c * pose.dy),
        -pose.theta_deg,
    )
}

/// A move cut down to the limits, with whether it was.
pub fn clamp(command: (f64, f64, f64), mm: f64, deg: f64) -> ((f64, f64, f64), bool) {
    let len = command.0.hypot(command.1);
    let k = if len > mm && len > 0.0 { mm / len } else { 1.0 };
    let turn = command.2.clamp(-deg, deg);
    let clamped = k < 1.0 || turn != command.2;
    ((command.0 * k, command.1 * k, turn), clamped)
}

fn score(mm: f64, deg: f64, s: &AimSettings) -> f64 {
    (mm / s.tolerance_mm).max(deg / s.tolerance_deg)
}

/// Looks, moves and looks again until the board is inside the tolerance. The camera is never asked to move and the stage is never asked for more than the limits allow, in whatever mode.
pub fn align_board(
    cam: &mut dyn Camera,
    stage: &mut dyn Stage,
    env: &Env,
    aim: &AimSettings,
    log: &Log,
) -> Outcome {
    let mut out = Outcome {
        result: Ok(()),
        steps: Vec::new(),
        last_pose: None,
        moved_mm: 0.0,
        moved_deg: 0.0,
        mode: Mode::Normal,
        evidence: cam.evidence(),
    };
    if let Err(e) = check(aim) {
        out.result = Err(e);
        out.mode = Mode::Abort;
        return out;
    }
    let mut last_print: Option<u64> = None;
    let mut previous: Option<f64> = None;
    let mut moved_since_print = false;
    for number in 1..=aim.max_steps + 1 {
        // look
        let seen = settle(cam, &env.settle).and_then(|s| {
            if mean(&s.frame) < env.health.black_mean {
                return Err(CapError::new(Code::StreamBlack, "the frame is black"));
            }
            let print = fingerprint(&s.frame.luma.data);
            if moved_since_print && last_print == Some(print) {
                return Err(CapError::new(
                    Code::StreamFrozen,
                    "the frame did not change after a move",
                ));
            }
            last_print = Some(print);
            Ok(s.frame)
        });
        let frame = match seen {
            Ok(f) => f,
            Err(e) => {
                out.mode = degrade(out.mode, Event::CameraBad);
                log.write(
                    Level::Error,
                    "align",
                    Some(Code::AlignCameraUnsafe),
                    e.to_string(),
                );
                out.result = Err(CapError::new(Code::AlignCameraUnsafe, e.to_string()));
                return out;
            }
        };
        let pose = match locate(&frame.luma, env) {
            Ok(p) => p,
            Err(e) => {
                out.mode = degrade(out.mode, Event::NoMarks);
                log.write(Level::Warn, "align", Some(e.code), e.to_string());
                out.result = Err(e);
                return out;
            }
        };
        out.last_pose = Some(pose);
        let (error_mm, error_deg) = (pose.dx.hypot(pose.dy), pose.theta_deg.abs());
        let mut step = Step {
            number,
            error_mm,
            error_deg,
            command: None,
            clamped: false,
            mode: out.mode,
        };
        if error_mm <= aim.tolerance_mm && error_deg <= aim.tolerance_deg {
            out.steps.push(step);
            return out;
        }
        if number > aim.max_steps {
            out.steps.push(step);
            out.result = Err(CapError::with_key(
                Code::AlignNotConverged,
                "align_max_steps",
                format!(
                    "{error_mm:.3} mm and {error_deg:.3} deg off after {} moves",
                    aim.max_steps
                ),
            ));
            return out;
        }
        // the error must go down
        let now = score(error_mm, error_deg, aim);
        if previous.is_some_and(|p| now > p * 1.05) {
            out.mode = degrade(out.mode, Event::Diverging);
            step.mode = out.mode;
            log.write(
                Level::Warn,
                "align",
                Some(Code::AlignDiverging),
                format!("{now:.2} after {:.2}", previous.unwrap_or(0.0)),
            );
            if out.mode >= Mode::Hold {
                out.steps.push(step);
                out.result = Err(CapError::new(
                    Code::AlignDiverging,
                    format!("{error_mm:.3} mm off and growing after move {}", number - 1),
                ));
                return out;
            }
        }
        previous = Some(now);
        // move
        let (mm, deg) = allowance(out.mode, aim);
        let wanted = correction(&pose);
        let (cmd, clamped) = clamp(
            (
                wanted.0 * aim.gain,
                wanted.1 * aim.gain,
                wanted.2 * aim.gain,
            ),
            mm,
            deg,
        );
        let length = cmd.0.hypot(cmd.1);
        if out.moved_mm + length > aim.max_total_mm {
            out.mode = degrade(out.mode, Event::LimitReached);
            step.mode = out.mode;
            out.steps.push(step);
            out.result = Err(CapError::with_key(
                Code::AlignTotalLimit,
                "align_max_total_mm",
                format!(
                    "{:.2} mm moved and {length:.2} more asked, {} allowed",
                    out.moved_mm, aim.max_total_mm
                ),
            ));
            return out;
        }
        step.command = Some(cmd);
        step.clamped = clamped;
        step.mode = out.mode;
        if let Err(e) = stage.move_by(cmd.0, cmd.1, cmd.2) {
            out.mode = degrade(out.mode, Event::StageFailed);
            out.steps.push(step);
            log.write(
                Level::Error,
                "align",
                Some(Code::AlignStageError),
                e.to_string(),
            );
            out.result = Err(CapError::new(Code::AlignStageError, e.to_string()));
            return out;
        }
        out.moved_mm += length;
        out.moved_deg += cmd.2.abs();
        moved_since_print = true;
        out.steps.push(step);
    }
    out
}

/// A stage that moves the board of the simulated world, with the faults a real one has: it moves a share of what it is asked (`accuracy`: 1 is exact, 0.7 stops short, a negative number goes
/// the wrong way) and it can fail on a given move.
#[derive(Debug)]
pub struct SimStage {
    world: crate::capture::sim::Shared,
    /// The share of each move that is really made.
    pub accuracy: f64,
    /// The number of the move (from 1) that fails.
    pub fail_on: Option<usize>,
    /// The moves that were asked, in order.
    pub asked: Vec<(f64, f64, f64)>,
}

impl SimStage {
    /// A stage that moves the board of `world` exactly.
    pub fn new(world: crate::capture::sim::Shared) -> SimStage {
        SimStage {
            world,
            accuracy: 1.0,
            fail_on: None,
            asked: Vec::new(),
        }
    }
}

impl Stage for SimStage {
    fn move_by(&mut self, dx: f64, dy: f64, dtheta_deg: f64) -> Result<(), CapError> {
        self.asked.push((dx, dy, dtheta_deg));
        if self.fail_on == Some(self.asked.len()) {
            return Err(CapError::new(
                Code::AlignStageError,
                "the stage did not move",
            ));
        }
        let k = self.accuracy;
        let mut w = self.world.lock().unwrap_or_else(|e| e.into_inner());
        let (s, c) = (dtheta_deg * k).to_radians().sin_cos();
        let (tx, ty) = (w.pose.dx, w.pose.dy);
        w.pose.dx = c * tx - s * ty + dx * k;
        w.pose.dy = s * tx + c * ty + dy * k;
        w.pose.theta_deg += dtheta_deg * k;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::sim::{home, Fault, Scene, SimBackend, Spec, Window, World};
    use crate::capture::{CameraId, CaptureBackend, FormatDesc, PixelFormat};

    struct Rig {
        cam: Box<dyn Camera>,
        stage: SimStage,
        world: crate::capture::sim::Shared,
        backend: SimBackend,
        id: CameraId,
        env: Env,
    }

    fn rig(pose: Transform) -> Rig {
        let world = World::board(pose).shared();
        let mut backend = SimBackend::new(11, world.clone());
        let spec = Spec::standard(1, "1-1");
        let id = spec.id.clone();
        backend.add(spec);
        let cam = backend
            .open(
                &id,
                FormatDesc {
                    width: 480,
                    height: 320,
                    fps: 30,
                    pixel: PixelFormat::Gray8,
                },
            )
            .unwrap();
        let design = vec![(-40.0, -25.0), (40.0, -25.0), (40.0, 25.0), (-40.0, 25.0)];
        Rig {
            cam,
            stage: SimStage::new(world.clone()),
            world,
            backend,
            id,
            env: Env::new(design, 0.25),
        }
    }

    fn off(dx: f64, dy: f64, deg: f64) -> Transform {
        Transform {
            dx,
            dy,
            theta_deg: deg,
            scale: 1.0,
            rms_mm: 0.0,
        }
    }

    fn run(r: &mut Rig, aim: &AimSettings) -> Outcome {
        align_board(
            r.cam.as_mut(),
            &mut r.stage,
            &r.env,
            aim,
            &Log::new(Level::Debug),
        )
    }

    fn pose(r: &Rig) -> Transform {
        r.world.lock().unwrap().pose
    }

    #[test]
    fn a_board_off_by_a_little_is_put_where_the_design_says_in_one_move_and_checked_by_a_second_look(
    ) {
        let mut r = rig(off(1.2, -0.8, 0.9));
        let out = run(&mut r, &AimSettings::default());
        assert!(out.result.is_ok(), "{:?}", out.result);
        assert_eq!(out.steps.len(), 2, "look, move, look");
        assert_eq!(r.stage.asked.len(), 1);
        let p = pose(&r);
        assert!(p.dx.hypot(p.dy) < 0.05 && p.theta_deg.abs() < 0.1, "{p:?}");
        assert_eq!(out.mode, Mode::Normal);
        assert!(out.steps[1].error_mm < 0.05);
    }

    #[test]
    fn the_outcome_says_that_the_camera_was_simulated() {
        let mut r = rig(off(0.4, 0.0, 0.0));
        let out = run(&mut r, &AimSettings::default());
        assert_eq!(out.evidence, "simulator");
        assert!(crate::convention::EVIDENCE.contains(&out.evidence));
    }

    #[test]
    fn a_board_already_in_place_is_not_moved() {
        let mut r = rig(home());
        let out = run(&mut r, &AimSettings::default());
        assert!(out.result.is_ok());
        assert_eq!(out.steps.len(), 1);
        assert!(r.stage.asked.is_empty());
        assert_eq!(out.moved_mm, 0.0);
    }

    #[test]
    fn a_stage_that_stops_short_still_gets_there_in_a_few_moves() {
        let mut r = rig(off(3.0, 2.0, -1.5));
        r.stage.accuracy = 0.7;
        let aim = AimSettings {
            max_step_mm: 10.0,
            ..AimSettings::default()
        };
        let out = run(&mut r, &aim);
        assert!(out.result.is_ok(), "{:?}", out.result);
        assert!(
            r.stage.asked.len() >= 3 && r.stage.asked.len() <= 6,
            "{}",
            r.stage.asked.len()
        );
        let errors: Vec<f64> = out.steps.iter().map(|s| s.error_mm).collect();
        assert!(errors.windows(2).all(|w| w[1] < w[0]), "{errors:?}");
    }

    #[test]
    fn no_move_is_larger_than_the_step_limit_and_the_correction_is_made_in_several() {
        let mut r = rig(off(5.0, 0.0, 3.0));
        let aim = AimSettings {
            max_step_mm: 1.5,
            max_step_deg: 1.0,
            max_total_mm: 20.0,
            max_steps: 8,
            ..AimSettings::default()
        };
        let out = run(&mut r, &aim);
        assert!(out.result.is_ok(), "{:?}", out.result);
        for (dx, dy, dt) in &r.stage.asked {
            assert!(dx.hypot(*dy) <= 1.5 + 1e-9 && dt.abs() <= 1.0 + 1e-9);
        }
        assert!(out.steps.iter().filter(|s| s.clamped).count() >= 2);
        assert!(r.stage.asked.len() >= 4);
    }

    #[test]
    fn too_few_steps_for_the_error_is_not_converged_and_names_the_key() {
        let mut r = rig(off(8.0, 0.0, 0.0));
        let aim = AimSettings {
            max_step_mm: 1.0,
            max_steps: 2,
            ..AimSettings::default()
        };
        let out = run(&mut r, &aim);
        let e = out.result.unwrap_err();
        assert_eq!(e.code, Code::AlignNotConverged);
        assert_eq!(e.key, Some("align_max_steps"));
        assert_eq!(r.stage.asked.len(), 2);
        assert!(out.moved_mm <= 2.0 + 1e-9);
    }

    #[test]
    fn the_total_shift_is_limited_before_the_move_and_the_stage_is_not_asked() {
        let mut r = rig(off(6.0, 0.0, 0.0));
        let aim = AimSettings {
            max_step_mm: 5.0,
            max_total_mm: 4.0,
            ..AimSettings::default()
        };
        let out = run(&mut r, &aim);
        let e = out.result.unwrap_err();
        assert_eq!(e.code, Code::AlignTotalLimit);
        assert_eq!(e.key, Some("align_max_total_mm"));
        assert!(r.stage.asked.is_empty());
        assert_eq!(out.mode, Mode::Hold);
    }

    #[test]
    fn a_stage_that_moves_the_wrong_way_is_stopped_before_it_runs_away() {
        let mut r = rig(off(2.0, 0.0, 0.0));
        r.stage.accuracy = -0.8;
        let out = run(&mut r, &AimSettings::default());
        let e = out.result.unwrap_err();
        assert_eq!(e.code, Code::AlignDiverging);
        assert!(out.mode >= Mode::Hold);
        assert!(r.stage.asked.len() <= 3, "{}", r.stage.asked.len());
        let moved = pose(&r).dx.hypot(pose(&r).dy);
        assert!(moved < 10.0, "{moved}");
        assert!(
            out.steps.iter().any(|s| s.mode == Mode::Reduced),
            "the step is halved first"
        );
    }

    #[test]
    fn a_stage_that_fails_aborts_without_a_second_move() {
        let mut r = rig(off(2.0, 1.0, 0.0));
        r.stage.fail_on = Some(1);
        let out = run(&mut r, &AimSettings::default());
        assert_eq!(out.result.unwrap_err().code, Code::AlignStageError);
        assert_eq!(out.mode, Mode::Abort);
        assert_eq!(r.stage.asked.len(), 1);
    }

    #[test]
    fn a_camera_that_gives_black_frames_never_gets_the_stage_moved() {
        let mut r = rig(off(2.0, 1.0, 0.0));
        r.backend.inject(
            &r.id.key(),
            Window {
                from: 1,
                to: 100,
                fault: Fault::Black,
            },
        );
        let out = run(&mut r, &AimSettings::default());
        let e = out.result.unwrap_err();
        assert_eq!(e.code, Code::AlignCameraUnsafe);
        assert!(e.detail.contains("stream_black"), "{e}");
        assert!(r.stage.asked.is_empty());
        assert_eq!(out.mode, Mode::Abort);
    }

    #[test]
    fn a_camera_that_repeats_a_frame_after_a_move_stops_the_alignment() {
        let mut r = rig(off(4.0, 0.0, 0.0));
        r.backend.inject(
            &r.id.key(),
            Window {
                from: 3,
                to: 100,
                fault: Fault::Frozen,
            },
        );
        let aim = AimSettings {
            max_step_mm: 1.0,
            ..AimSettings::default()
        };
        r.env.settle = SettleSettings {
            frames: 0,
            tries: 5,
        };
        let out = run(&mut r, &aim);
        let e = out.result.unwrap_err();
        assert_eq!(e.code, Code::AlignCameraUnsafe);
        assert!(e.detail.contains("stream_frozen"), "{e}");
        assert_eq!(out.mode, Mode::Abort);
        assert_eq!(r.stage.asked.len(), 2);
    }

    #[test]
    fn marks_that_are_not_found_leave_the_board_where_it_is() {
        let mut r = rig(home());
        r.world.lock().unwrap().scene = Scene::Flat(120);
        let out = run(&mut r, &AimSettings::default());
        assert_eq!(out.result.unwrap_err().code, Code::AlignNoMarks);
        assert_eq!(out.mode, Mode::Hold);
        assert!(r.stage.asked.is_empty());
    }

    #[test]
    fn a_design_with_a_mark_too_many_is_no_marks() {
        let mut r = rig(home());
        r.env.design.push((0.0, 0.0));
        let out = run(&mut r, &AimSettings::default());
        let e = out.result.unwrap_err();
        assert_eq!(e.code, Code::AlignNoMarks);
        assert!(e.detail.contains("4 marks found, 5"), "{e}");
    }

    #[test]
    fn the_offset_of_the_camera_comes_from_the_configuration_and_moves_the_target_by_exactly_that()
    {
        let mut r = rig(off(0.5, 0.5, 0.0));
        r.env.offset = CameraOffset {
            x_mm: 1.0,
            y_mm: 0.5,
            deg: 0.0,
        };
        let out = run(&mut r, &AimSettings::default());
        assert!(out.result.is_ok(), "{:?}", out.result);
        let p = pose(&r);
        // the board ends where the camera sees it at minus the offset
        assert!(
            (p.dx + 1.0).abs() < 0.05 && (p.dy + 0.5).abs() < 0.05,
            "{p:?}"
        );
    }

    #[test]
    fn a_delayed_driver_is_waited_for_by_frames_before_the_second_look() {
        let mut r = rig(off(1.5, 0.0, 0.0));
        r.backend.set_latency(&r.id.key(), 2);
        r.env.settle = SettleSettings {
            frames: 3,
            tries: 20,
        };
        let out = run(&mut r, &AimSettings::default());
        assert!(out.result.is_ok(), "{:?}", out.result);
        assert_eq!(r.stage.asked.len(), 1);
    }

    #[test]
    fn the_correction_is_the_opposite_of_the_pose() {
        for (dx, dy, deg) in [
            (1.0, 2.0, 0.0),
            (-3.0, 0.5, 20.0),
            (0.0, 0.0, -7.0),
            (4.0, -4.0, 170.0),
        ] {
            let t = off(dx, dy, deg);
            let (cx, cy, ct) = correction(&t);
            // apply the correction to the pose: a turn about the origin, then a shift
            let (s, c) = ct.to_radians().sin_cos();
            let (nx, ny) = (c * dx - s * dy + cx, s * dx + c * dy + cy);
            assert!(nx.abs() < 1e-9 && ny.abs() < 1e-9 && (deg + ct).abs() < 1e-9);
        }
    }

    #[test]
    fn settings_that_make_no_sense_are_refused_and_the_stage_is_not_touched() {
        let base = AimSettings::default();
        let cases = [
            (
                AimSettings {
                    tolerance_mm: 0.0,
                    ..base
                },
                "align_tolerance_mm",
            ),
            (
                AimSettings {
                    tolerance_deg: f64::NAN,
                    ..base
                },
                "align_tolerance_deg",
            ),
            (
                AimSettings {
                    max_step_mm: 51.0,
                    ..base
                },
                "align_max_step_mm",
            ),
            (
                AimSettings {
                    max_step_deg: -1.0,
                    ..base
                },
                "align_max_step_deg",
            ),
            (
                AimSettings {
                    max_total_mm: 0.0,
                    ..base
                },
                "align_max_total_mm",
            ),
            (
                AimSettings {
                    max_steps: 0,
                    ..base
                },
                "align_max_steps",
            ),
            (AimSettings { gain: 1.5, ..base }, "align_gain"),
        ];
        for (aim, key) in cases {
            assert_eq!(check(&aim).unwrap_err().key, Some(key));
            let mut r = rig(off(1.0, 0.0, 0.0));
            let out = run(&mut r, &aim);
            assert!(out.result.is_err() && r.stage.asked.is_empty() && out.mode == Mode::Abort);
        }
    }

    // the rule of the safest state

    #[test]
    fn no_event_ever_gives_a_mode_freer_than_the_one_before() {
        for m in Mode::ALL {
            for e in Event::ALL {
                assert!(degrade(m, e) >= m, "{m:?} after {e:?}");
            }
        }
    }

    #[test]
    fn whatever_happens_in_whatever_order_the_mode_only_goes_up_and_a_stopped_mode_does_not_move() {
        // every sequence of up to six events
        let mut seqs: Vec<Vec<Event>> = vec![Vec::new()];
        for _ in 0..6 {
            let next: Vec<Vec<Event>> = seqs
                .iter()
                .flat_map(|s| {
                    Event::ALL.iter().map(move |&e| {
                        let mut t = s.clone();
                        t.push(e);
                        t
                    })
                })
                .collect();
            seqs = next;
            for s in seqs.iter().step_by(37) {
                let mut mode = Mode::Normal;
                for &e in s {
                    let next = degrade(mode, e);
                    assert!(next >= mode);
                    if mode >= Mode::Hold {
                        assert_eq!(allowance(next, &AimSettings::default()), (0.0, 0.0));
                    }
                    mode = next;
                }
            }
        }
    }

    #[test]
    fn the_modes_have_the_allowances_of_their_order_and_the_stopped_ones_have_none() {
        let s = AimSettings::default();
        assert_eq!(allowance(Mode::Normal, &s), (2.0, 2.0));
        assert_eq!(allowance(Mode::Reduced, &s), (1.0, 1.0));
        assert_eq!(allowance(Mode::Hold, &s), (0.0, 0.0));
        assert_eq!(allowance(Mode::Abort, &s), (0.0, 0.0));
        assert!(
            Mode::Normal < Mode::Reduced && Mode::Reduced < Mode::Hold && Mode::Hold < Mode::Abort
        );
        assert_eq!(degrade(Mode::Normal, Event::Diverging), Mode::Reduced);
        assert_eq!(degrade(Mode::Reduced, Event::Diverging), Mode::Hold);
        assert_eq!(degrade(Mode::Abort, Event::Fine), Mode::Abort);
        assert_eq!(degrade(Mode::Hold, Event::Fine), Mode::Hold);
    }

    #[test]
    fn a_move_is_cut_to_the_limits_and_the_cut_keeps_its_direction() {
        let ((dx, dy, dt), clamped) = clamp((3.0, 4.0, 5.0), 2.5, 2.0);
        assert!(clamped);
        assert!((dx.hypot(dy) - 2.5).abs() < 1e-9 && (dx / dy - 0.75).abs() < 1e-9 && dt == 2.0);
        assert_eq!(clamp((0.3, 0.4, -0.1), 2.5, 2.0), ((0.3, 0.4, -0.1), false));
        assert_eq!(clamp((0.0, 0.0, -9.0), 1.0, 2.0).0 .2, -2.0);
        assert_eq!(clamp((1.0, 0.0, 0.0), 0.0, 0.0), ((0.0, 0.0, 0.0), true));
    }
}
