//! The simulated camera. It draws what a camera over a board would see (fiducial marks that a known turn and shift put in place, a calibration picture, or a flat field), packs the frame in the
//! format that was asked for (YUYV, motion JPEG in a stored form, RGB or grey) and reads it back through the same readers a real camera goes through. It fails on purpose: the camera that is
//! unplugged, black frames, the same frame again and again, a frame that is cut or has the wrong size, frames that are lost, and a delay of some frames between a change and the picture
//! that shows it. It reads no clock and draws its noise from a seed, so a run repeats exactly. What it proves is how the program behaves; it says nothing about a real camera.

use super::log::{Level, Log};
use super::pixels;
use super::{
    Camera, CameraId, CameraInfo, CapError, CaptureBackend, Code, FormatDesc, Frame, PixelFormat,
    Property, PropertyRange, PropertyValue,
};
use crate::align::Transform;
use crate::image::Gray;
use crate::sim::SimRng;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};

/// What the camera looks at.
#[derive(Debug, Clone)]
pub enum Scene {
    /// One grey level.
    Flat(u8),
    /// Dark discs on a light ground at the places of the design, which the pose of the board moves.
    Fiducials {
        design: Vec<(f64, f64)>,
        mm_per_px: f64,
        radius_px: f64,
    },
    /// A picture that is shown as it is (a calibration board drawn by `charuco`); what is outside it is grey.
    Picture(Gray),
}

/// What the camera sees at one moment: where the board is and how the sensor is set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    pub pose: Transform,
    pub exposure: f64,
    pub gain: f64,
}

/// The world that the cameras look at and the stage moves: a scene and the pose of the board in it.
#[derive(Debug, Clone)]
pub struct World {
    pub scene: Scene,
    pub pose: Transform,
}

/// The pose of a board that is where its design puts it: no shift and no turn.
pub fn home() -> Transform {
    Transform {
        dx: 0.0,
        dy: 0.0,
        theta_deg: 0.0,
        scale: 1.0,
        rms_mm: 0.0,
    }
}

/// A world that the cameras and the stage share.
pub type Shared = Arc<Mutex<World>>;

impl World {
    /// A world with the board at the origin, not turned.
    pub fn new(scene: Scene) -> World {
        World {
            scene,
            pose: home(),
        }
    }

    /// The four marks of a board 80 by 50 millimetres at a quarter of a millimetre for each pixel, with the board moved by `pose`.
    pub fn board(pose: Transform) -> World {
        let mut w = World::new(Scene::Fiducials {
            design: vec![(-40.0, -25.0), (40.0, -25.0), (40.0, 25.0), (-40.0, 25.0)],
            mm_per_px: 0.25,
            radius_px: 10.0,
        });
        w.pose = pose;
        w
    }

    /// The world to share between the cameras and the stage.
    pub fn shared(self) -> Shared {
        Arc::new(Mutex::new(self))
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Draws a frame of the scene as the look says. The exposure is a percentage of the normal one and the gain adds to it.
pub fn render(scene: &Scene, look: &Look, w: usize, h: usize) -> Gray {
    let k = (look.exposure / 100.0) * (1.0 + look.gain / 100.0);
    let mut frame = match scene {
        Scene::Flat(v) => Gray::blank(w, h, *v).expect("a frame of a legal size"),
        Scene::Picture(p) => {
            let mut g = Gray::blank(w, h, 128).expect("a frame of a legal size");
            for y in 0..h.min(p.h) {
                for x in 0..w.min(p.w) {
                    g.data[y * w + x] = p.data[y * p.w + x];
                }
            }
            g
        }
        Scene::Fiducials {
            design,
            mm_per_px,
            radius_px,
        } => {
            let mut g = Gray::blank(w, h, 220).expect("a frame of a legal size");
            for &mark in design {
                let (x, y) = look.pose.apply(mark);
                g.disc(
                    w as f64 / 2.0 + x / mm_per_px,
                    h as f64 / 2.0 + y / mm_per_px,
                    *radius_px,
                    30,
                );
            }
            g
        }
    };
    if (k - 1.0).abs() > 1e-9 {
        for v in &mut frame.data {
            *v = (f64::from(*v) * k).round().clamp(0.0, 255.0) as u8;
        }
    }
    frame
}

/// What goes wrong with the frames of a camera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// The camera is not there: asking for a frame is `camera_disconnected`.
    Disconnect,
    /// Every pixel is 0.
    Black,
    /// The frame of before comes again.
    Frozen,
    /// The frame is cut in half.
    Truncated,
    /// The frame has one byte more than its format says.
    BadSize,
    /// This many frames are lost before this one (the counter jumps).
    Skip(u32),
}

/// The frames, counted from 1, that a fault covers: `from` to `to` both included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub from: u64,
    pub to: u64,
    pub fault: Fault,
}

/// A camera of the simulation.
#[derive(Debug, Clone)]
pub struct Spec {
    pub name: String,
    pub id: CameraId,
    pub formats: Vec<FormatDesc>,
    /// Frames between a change and the picture that shows it.
    pub latency_frames: usize,
    pub windows: Vec<Window>,
}

impl Spec {
    /// A camera with the formats most cameras list: 640 by 480 at 30 and 480 by 320 at 30 in every pixel format, and one that is listed with no size, as a CSI camera is by V4L2.
    pub fn standard(n: u16, port: &str) -> Spec {
        let mut formats = vec![FormatDesc {
            width: 0,
            height: 0,
            fps: 0,
            pixel: PixelFormat::Yuyv,
        }];
        for pixel in PixelFormat::ALL {
            for (width, height) in [(640, 480), (480, 320)] {
                formats.push(FormatDesc {
                    width,
                    height,
                    fps: 30,
                    pixel,
                });
            }
        }
        Spec {
            name: format!("Simulated camera {n}"),
            id: CameraId {
                vendor: 0x1234,
                product: 0x5678,
                serial: format!("SIM{n:04}"),
                port: port.to_string(),
            },
            formats,
            latency_frames: 0,
            windows: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct Entry {
    spec: Spec,
    connected: bool,
    in_use: bool,
}

#[derive(Debug, Default)]
struct State {
    entries: Vec<Entry>,
}

/// The backend of the simulation.
#[derive(Debug, Clone)]
pub struct SimBackend {
    state: Arc<Mutex<State>>,
    world: Shared,
    seed: u64,
    log: Log,
}

impl SimBackend {
    /// A backend with no cameras looking at `world`.
    pub fn new(seed: u64, world: Shared) -> SimBackend {
        SimBackend {
            state: Arc::new(Mutex::new(State::default())),
            world,
            seed,
            log: Log::new(Level::Warn),
        }
    }

    /// The same backend writing to `log`.
    pub fn with_log(mut self, log: Log) -> SimBackend {
        self.log = log;
        self
    }

    /// Adds a camera.
    pub fn add(&self, spec: Spec) {
        lock(&self.state).entries.push(Entry {
            spec,
            connected: true,
            in_use: false,
        });
    }

    /// Unplugs the camera with this key: it is not listed and its open handle gives `camera_disconnected`.
    pub fn unplug(&self, key: &str) {
        for e in &mut lock(&self.state).entries {
            if e.spec.id.key() == key {
                e.connected = false;
            }
        }
    }

    /// Plugs it again, on another port when `port` is given.
    pub fn plug(&self, key: &str, port: Option<&str>) {
        for e in &mut lock(&self.state).entries {
            if e.spec.id.key() == key {
                e.connected = true;
                if let Some(p) = port {
                    e.spec.id.port = p.to_string();
                }
            }
        }
    }

    /// Sets the frames between a change and the picture that shows it, for the camera with this key.
    pub fn set_latency(&self, key: &str, frames: usize) {
        for e in &mut lock(&self.state).entries {
            if e.spec.id.key() == key {
                e.spec.latency_frames = frames;
            }
        }
    }

    /// Puts a fault on the frames of a camera.
    pub fn inject(&self, key: &str, window: Window) {
        for e in &mut lock(&self.state).entries {
            if e.spec.id.key() == key {
                e.spec.windows.push(window);
            }
        }
    }
}

impl CaptureBackend for SimBackend {
    fn name(&self) -> &'static str {
        "sim"
    }

    fn evidence(&self) -> &'static str {
        "simulator"
    }

    fn enumerate(&mut self) -> Result<Vec<CameraInfo>, CapError> {
        let st = lock(&self.state);
        Ok(st
            .entries
            .iter()
            .filter(|e| e.connected)
            .map(|e| CameraInfo {
                name: e.spec.name.clone(),
                id: e.spec.id.clone(),
                formats: e.spec.formats.clone(),
            })
            .filter(|c| !c.usable_formats().is_empty())
            .collect())
    }

    fn open(&mut self, id: &CameraId, format: FormatDesc) -> Result<Box<dyn Camera>, CapError> {
        if !format.is_usable() {
            return Err(CapError::new(
                Code::FormatInvalid,
                format!("{}x{} at {}", format.width, format.height, format.fps),
            ));
        }
        let mut st = lock(&self.state);
        let at = st
            .entries
            .iter()
            .position(|e| e.connected && e.spec.id == *id)
            .ok_or_else(|| CapError::new(Code::CameraNotFound, id.key()))?;
        let entry = &mut st.entries[at];
        if !entry.spec.formats.contains(&format) {
            return Err(CapError::new(
                Code::FormatUnsupported,
                format!(
                    "{}x{} at {} as {}",
                    format.width,
                    format.height,
                    format.fps,
                    format.pixel.name()
                ),
            ));
        }
        if entry.in_use {
            return Err(CapError::new(Code::CameraBusy, id.key()));
        }
        entry.in_use = true;
        let info = CameraInfo {
            name: entry.spec.name.clone(),
            id: entry.spec.id.clone(),
            formats: entry.spec.formats.clone(),
        };
        self.log.write(
            Level::Info,
            "sim",
            None,
            format!("opened {} at {}", id.key(), format.pixel.name()),
        );
        Ok(Box::new(SimCamera {
            state: Arc::clone(&self.state),
            world: Arc::clone(&self.world),
            info,
            format,
            seed: self.seed,
            count: 0,
            props: HashMap::new(),
            looks: VecDeque::new(),
            last_raw: None,
            closed: false,
            log: self.log.clone(),
        }))
    }
}

/// The property table of the simulated camera; the other properties are not there.
pub fn range_of(p: Property) -> Option<PropertyRange> {
    let r = |min, max, step, default, auto_supported| {
        Some(PropertyRange {
            min,
            max,
            step,
            default,
            auto_supported,
        })
    };
    match p {
        Property::Exposure => r(1.0, 1000.0, 1.0, 100.0, true),
        Property::Gain => r(0.0, 100.0, 1.0, 0.0, false),
        Property::Focus => r(0.0, 255.0, 1.0, 128.0, true),
        Property::Brightness => r(0.0, 255.0, 1.0, 128.0, false),
        Property::Contrast => r(0.0, 255.0, 1.0, 32.0, false),
        Property::WhiteBalance => r(2800.0, 6500.0, 100.0, 4600.0, true),
        Property::Zoom => r(100.0, 500.0, 1.0, 100.0, false),
        Property::PowerlineHz => r(50.0, 60.0, 10.0, 50.0, false),
        _ => None,
    }
}

#[derive(Debug)]
struct SimCamera {
    state: Arc<Mutex<State>>,
    world: Shared,
    info: CameraInfo,
    format: FormatDesc,
    seed: u64,
    count: u64,
    props: HashMap<Property, PropertyValue>,
    looks: VecDeque<Look>,
    last_raw: Option<Vec<u8>>,
    closed: bool,
    log: Log,
}

impl SimCamera {
    fn index(&self) -> Option<usize> {
        lock(&self.state).entries.iter().position(|e| {
            e.spec.id.vendor == self.info.id.vendor && e.spec.id.serial == self.info.id.serial
        })
    }

    fn connected(&self) -> bool {
        self.index()
            .is_some_and(|i| lock(&self.state).entries[i].connected)
    }

    fn value(&self, p: Property) -> PropertyValue {
        self.props
            .get(&p)
            .copied()
            .unwrap_or_else(|| PropertyValue::fixed(range_of(p).map_or(0.0, |r| r.default)))
    }

    fn pack(&self, luma: &Gray) -> Vec<u8> {
        match self.format.pixel {
            PixelFormat::Yuyv => luma.data.iter().flat_map(|&y| [y, 128]).collect(),
            PixelFormat::Rgb24 => luma.data.iter().flat_map(|&y| [y, y, y]).collect(),
            PixelFormat::Gray8 => luma.data.clone(),
            PixelFormat::Mjpeg => pixels::stored_jpeg(luma),
        }
    }

    fn unpack(&self, raw: &[u8]) -> Result<Gray, CapError> {
        let (w, h) = (self.format.width as usize, self.format.height as usize);
        match self.format.pixel {
            PixelFormat::Yuyv => pixels::yuyv_to_luma(raw, w, h),
            PixelFormat::Rgb24 => pixels::rgb24_to_luma(raw, w, h),
            PixelFormat::Gray8 => pixels::gray8_to_luma(raw, w, h),
            PixelFormat::Mjpeg => pixels::mjpeg_to_luma(raw),
        }
    }
}

impl Camera for SimCamera {
    fn info(&self) -> &CameraInfo {
        &self.info
    }

    fn evidence(&self) -> &'static str {
        "simulator"
    }

    fn format(&self) -> FormatDesc {
        self.format
    }

    fn property_range(&self, p: Property) -> Option<PropertyRange> {
        range_of(p)
    }

    fn get(&self, p: Property) -> Result<PropertyValue, CapError> {
        if range_of(p).is_none() {
            return Err(CapError::new(Code::PropertyUnsupported, p.name()));
        }
        Ok(self.value(p))
    }

    fn set(&mut self, p: Property, v: PropertyValue) -> Result<(), CapError> {
        let Some(r) = range_of(p) else {
            return Err(CapError::new(Code::PropertyUnsupported, p.name()));
        };
        if v.auto {
            if !r.auto_supported {
                return Err(CapError::new(Code::PropertyAutoUnsupported, p.name()));
            }
        } else if !r.allows(v.value) {
            return Err(CapError::new(
                Code::PropertyOutOfRange,
                format!(
                    "{} {} is not in {} to {} by {}",
                    p.name(),
                    v.value,
                    r.min,
                    r.max,
                    r.step
                ),
            ));
        }
        self.props.insert(p, v);
        Ok(())
    }

    fn has_new_frame(&self) -> bool {
        !self.closed && self.connected()
    }

    fn frame_count(&self) -> u64 {
        self.count
    }

    fn grab(&mut self) -> Result<Frame, CapError> {
        if self.closed || !self.connected() {
            return Err(CapError::new(Code::CameraDisconnected, self.info.id.key()));
        }
        self.count += 1;
        let window = self.index().and_then(|i| {
            lock(&self.state).entries[i]
                .spec
                .windows
                .iter()
                .find(|w| (w.from..=w.to).contains(&self.count))
                .copied()
        });
        let fault = window.map(|w| w.fault);
        if let Some(Fault::Skip(n)) = fault {
            self.count += u64::from(n);
        }
        if fault == Some(Fault::Disconnect) {
            self.log.write(
                Level::Warn,
                "sim",
                Some(Code::CameraDisconnected),
                format!("frame {} was not made", self.count),
            );
            return Err(CapError::new(Code::CameraDisconnected, self.info.id.key()));
        }
        let (w, h) = (self.format.width as usize, self.format.height as usize);
        let exposure = self.value(Property::Exposure);
        let gain = self.value(Property::Gain).value;
        let latency = self
            .index()
            .map_or(0, |i| lock(&self.state).entries[i].spec.latency_frames);
        let look = {
            let world = lock(&self.world);
            Look {
                pose: world.pose,
                exposure: if exposure.auto { 100.0 } else { exposure.value },
                gain,
            }
        };
        self.looks.push_back(look);
        while self.looks.len() > latency + 1 {
            self.looks.pop_front();
        }
        let shown = self.looks.front().copied().unwrap_or(look);
        let mut luma = render(&lock(&self.world).scene, &shown, w, h);
        let mut rng = SimRng::new(self.seed ^ self.count.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        for v in &mut luma.data {
            let moved = i32::from(*v) + rng.below(3) as i32 - 1;
            *v = moved.clamp(0, 255) as u8;
        }
        if fault == Some(Fault::Black) {
            luma.data.iter_mut().for_each(|v| *v = 0);
        }
        let mut raw = match (&fault, &self.last_raw) {
            (Some(Fault::Frozen), Some(last)) => last.clone(),
            _ => self.pack(&luma),
        };
        if fault != Some(Fault::Frozen) {
            self.last_raw = Some(raw.clone());
        }
        match fault {
            Some(Fault::Truncated) => raw.truncate(raw.len() / 2),
            Some(Fault::BadSize) => raw.push(0),
            _ => {}
        }
        let luma = self.unpack(&raw).inspect_err(|e| {
            self.log
                .write(Level::Warn, "sim", Some(e.code), e.detail.clone());
        })?;
        Ok(Frame {
            seq: self.count,
            time_us: self.count * 1_000_000 / u64::from(self.format.fps.max(1)),
            luma,
            raw,
            raw_format: self.format.pixel,
        })
    }

    fn close(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        if let Some(i) = self.index() {
            lock(&self.state).entries[i].in_use = false;
        }
    }
}

impl Drop for SimCamera {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::fourcc;

    fn backend() -> (SimBackend, CameraId) {
        let b = SimBackend::new(7, World::board(home()).shared());
        let spec = Spec::standard(1, "1-2");
        let id = spec.id.clone();
        b.add(spec);
        (b, id)
    }

    fn fmt(pixel: PixelFormat) -> FormatDesc {
        FormatDesc {
            width: 480,
            height: 320,
            fps: 30,
            pixel,
        }
    }

    fn open(b: &mut SimBackend, id: &CameraId, pixel: PixelFormat) -> Box<dyn Camera> {
        b.open(id, fmt(pixel)).unwrap()
    }

    #[test]
    fn the_listing_has_the_cameras_by_name_with_only_formats_that_can_be_used() {
        let (mut b, id) = backend();
        let list = b.enumerate().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, id);
        assert_eq!(list[0].name, "Simulated camera 1");
        assert_eq!(list[0].formats.len(), 9);
        assert_eq!(list[0].usable_formats().len(), 8);
        assert_eq!(fourcc::clean(&list[0].formats).len(), 8);
    }

    #[test]
    fn every_pixel_format_gives_the_same_picture_through_its_reader() {
        let mut lumas = Vec::new();
        for pixel in PixelFormat::ALL {
            let (mut b, id) = backend();
            let mut cam = open(&mut b, &id, pixel);
            let f = cam.grab().unwrap();
            assert_eq!(f.raw_format, pixel);
            assert_eq!((f.luma.w, f.luma.h), (480, 320));
            lumas.push(f.luma);
        }
        assert!(lumas.windows(2).all(|p| p[0] == p[1]));
    }

    #[test]
    fn the_same_seed_gives_the_same_frames_and_another_seed_another_noise() {
        let grab = |seed: u64| {
            let b = SimBackend::new(seed, World::board(home()).shared());
            let spec = Spec::standard(1, "1-2");
            let id = spec.id.clone();
            b.add(spec);
            let mut b = b;
            let mut cam = open(&mut b, &id, PixelFormat::Gray8);
            (cam.grab().unwrap().luma, cam.grab().unwrap().luma)
        };
        assert_eq!(grab(3), grab(3));
        let (a, c) = grab(3);
        assert_ne!(a, c, "two frames of one run differ by their noise");
        assert_ne!(grab(3).0, grab(4).0);
    }

    #[test]
    fn the_frame_counter_counts_from_one_and_the_time_follows_the_rate() {
        let (mut b, id) = backend();
        let mut cam = open(&mut b, &id, PixelFormat::Yuyv);
        assert_eq!(cam.frame_count(), 0);
        assert!(cam.has_new_frame());
        let f1 = cam.grab().unwrap();
        let f2 = cam.grab().unwrap();
        assert_eq!((f1.seq, f2.seq, cam.frame_count()), (1, 2, 2));
        assert_eq!(f2.time_us - f1.time_us, 33_333);
    }

    #[test]
    fn a_camera_that_is_open_is_busy_until_it_is_closed_and_a_format_it_does_not_list_is_refused() {
        let (mut b, id) = backend();
        let mut cam = open(&mut b, &id, PixelFormat::Gray8);
        assert_eq!(
            b.open(&id, fmt(PixelFormat::Gray8)).err().unwrap().code,
            Code::CameraBusy
        );
        cam.close();
        assert!(b.open(&id, fmt(PixelFormat::Gray8)).is_ok());
        let odd = FormatDesc {
            width: 333,
            ..fmt(PixelFormat::Gray8)
        };
        assert_eq!(
            b.open(&id, odd).err().unwrap().code,
            Code::FormatUnsupported
        );
        let none = FormatDesc {
            width: 0,
            height: 0,
            fps: 0,
            pixel: PixelFormat::Yuyv,
        };
        assert_eq!(b.open(&id, none).err().unwrap().code, Code::FormatInvalid);
        let mut ghost = id.clone();
        ghost.serial = "NOPE".into();
        assert_eq!(
            b.open(&ghost, fmt(PixelFormat::Gray8)).err().unwrap().code,
            Code::CameraNotFound
        );
    }

    #[test]
    fn dropping_the_camera_frees_it() {
        let (mut b, id) = backend();
        drop(open(&mut b, &id, PixelFormat::Gray8));
        assert!(b.open(&id, fmt(PixelFormat::Gray8)).is_ok());
    }

    #[test]
    fn an_unplugged_camera_is_not_listed_and_its_handle_says_so_and_plugging_it_on_another_port_is_seen(
    ) {
        let (mut b, id) = backend();
        let mut cam = open(&mut b, &id, PixelFormat::Gray8);
        assert!(cam.grab().is_ok());
        b.unplug(&id.key());
        assert!(b.enumerate().unwrap().is_empty());
        assert!(!cam.has_new_frame());
        assert_eq!(cam.grab().unwrap_err().code, Code::CameraDisconnected);
        b.plug(&id.key(), Some("3-1"));
        assert_eq!(b.enumerate().unwrap()[0].id.port, "3-1");
        assert!(cam.grab().is_ok());
    }

    #[test]
    fn faults_in_a_window_of_frames_hit_those_frames_and_no_others() {
        let (mut b, id) = backend();
        b.inject(
            &id.key(),
            Window {
                from: 2,
                to: 3,
                fault: Fault::Black,
            },
        );
        b.inject(
            &id.key(),
            Window {
                from: 4,
                to: 4,
                fault: Fault::Truncated,
            },
        );
        b.inject(
            &id.key(),
            Window {
                from: 5,
                to: 5,
                fault: Fault::BadSize,
            },
        );
        b.inject(
            &id.key(),
            Window {
                from: 6,
                to: 6,
                fault: Fault::Disconnect,
            },
        );
        let mut cam = open(&mut b, &id, PixelFormat::Yuyv);
        let mean =
            |g: &Gray| g.data.iter().map(|&v| f64::from(v)).sum::<f64>() / g.data.len() as f64;
        assert!(mean(&cam.grab().unwrap().luma) > 100.0);
        assert_eq!(mean(&cam.grab().unwrap().luma), 0.0);
        assert_eq!(mean(&cam.grab().unwrap().luma), 0.0);
        assert_eq!(cam.grab().unwrap_err().code, Code::FrameTruncated);
        assert_eq!(cam.grab().unwrap_err().code, Code::FrameBadSize);
        assert_eq!(cam.grab().unwrap_err().code, Code::CameraDisconnected);
        assert!(mean(&cam.grab().unwrap().luma) > 100.0);
    }

    #[test]
    fn a_frozen_stream_repeats_the_bytes_of_the_last_good_frame() {
        let (mut b, id) = backend();
        b.inject(
            &id.key(),
            Window {
                from: 3,
                to: 5,
                fault: Fault::Frozen,
            },
        );
        let mut cam = open(&mut b, &id, PixelFormat::Gray8);
        let f2 = {
            cam.grab().unwrap();
            cam.grab().unwrap()
        };
        let f3 = cam.grab().unwrap();
        let f5 = {
            cam.grab().unwrap();
            cam.grab().unwrap()
        };
        assert_eq!(f2.raw, f3.raw);
        assert_eq!(f2.raw, f5.raw);
        assert_eq!((f3.seq, f5.seq), (3, 5));
        assert_ne!(f2.raw, cam.grab().unwrap().raw);
    }

    #[test]
    fn lost_frames_make_the_counter_jump() {
        let (mut b, id) = backend();
        b.inject(
            &id.key(),
            Window {
                from: 3,
                to: 3,
                fault: Fault::Skip(4),
            },
        );
        let mut cam = open(&mut b, &id, PixelFormat::Gray8);
        let seqs: Vec<u64> = (0..4).map(|_| cam.grab().unwrap().seq).collect();
        assert_eq!(seqs, vec![1, 2, 7, 8]);
        assert_eq!(cam.frame_count(), 8);
    }

    #[test]
    fn a_change_shows_in_the_picture_only_after_the_latency() {
        let (mut b, id) = backend();
        {
            let mut st = lock(&b.state);
            st.entries[0].spec.latency_frames = 2;
        }
        let world = Arc::clone(&b.world);
        let mut cam = open(&mut b, &id, PixelFormat::Gray8);
        let shifted = |g: &Gray| g.data.iter().position(|&v| v < 100).unwrap_or(0) % 480;
        let first = shifted(&cam.grab().unwrap().luma);
        lock(&world).pose.dx = 20.0;
        let seen: Vec<usize> = (0..4).map(|_| shifted(&cam.grab().unwrap().luma)).collect();
        assert_eq!(seen[0], first);
        assert_eq!(seen[1], first);
        assert_ne!(seen[2], first);
        assert_eq!(seen[2], seen[3]);
    }

    #[test]
    fn properties_have_ranges_and_the_ones_the_camera_lacks_are_said_not_to_exist() {
        let (mut b, id) = backend();
        let mut cam = open(&mut b, &id, PixelFormat::Gray8);
        assert!(cam.property_range(Property::Exposure).is_some());
        assert!(cam.property_range(Property::Hue).is_none());
        assert_eq!(
            cam.get(Property::Hue).unwrap_err().code,
            Code::PropertyUnsupported
        );
        assert_eq!(cam.get(Property::Gain).unwrap().value, 0.0);
        cam.set(Property::Gain, PropertyValue::fixed(40.0)).unwrap();
        assert_eq!(cam.get(Property::Gain).unwrap().value, 40.0);
        assert_eq!(
            cam.set(Property::Gain, PropertyValue::fixed(101.0))
                .unwrap_err()
                .code,
            Code::PropertyOutOfRange
        );
        assert_eq!(
            cam.set(Property::PowerlineHz, PropertyValue::fixed(55.0))
                .unwrap_err()
                .code,
            Code::PropertyOutOfRange
        );
        assert_eq!(
            cam.set(Property::Gain, PropertyValue::auto())
                .unwrap_err()
                .code,
            Code::PropertyAutoUnsupported
        );
        assert!(cam.set(Property::Focus, PropertyValue::auto()).is_ok());
        assert_eq!(
            cam.set(Property::Hue, PropertyValue::fixed(1.0))
                .unwrap_err()
                .code,
            Code::PropertyUnsupported
        );
    }

    #[test]
    fn the_exposure_changes_the_brightness_of_the_picture() {
        let (mut b, id) = backend();
        let mut cam = open(&mut b, &id, PixelFormat::Gray8);
        let mean =
            |g: &Gray| g.data.iter().map(|&v| f64::from(v)).sum::<f64>() / g.data.len() as f64;
        let normal = mean(&cam.grab().unwrap().luma);
        cam.set(Property::Exposure, PropertyValue::fixed(50.0))
            .unwrap();
        let dim = mean(&cam.grab().unwrap().luma);
        assert!((dim / normal - 0.5).abs() < 0.03, "{dim} {normal}");
        cam.set(Property::Exposure, PropertyValue::auto()).unwrap();
        assert!((mean(&cam.grab().unwrap().luma) / normal - 1.0).abs() < 0.03);
    }

    #[test]
    fn the_backend_and_its_cameras_say_that_they_are_simulated() {
        let (mut b, id) = backend();
        assert_eq!(b.evidence(), "simulator");
        assert_eq!(
            open(&mut b, &id, PixelFormat::Gray8).evidence(),
            "simulator"
        );
        assert!(crate::convention::EVIDENCE.contains(&b.evidence()));
    }

    #[test]
    fn the_log_hears_of_the_faults() {
        let log = Log::new(Level::Warn);
        let (b, id) = backend();
        let mut b = b.with_log(log.clone());
        b.inject(
            &id.key(),
            Window {
                from: 1,
                to: 1,
                fault: Fault::Truncated,
            },
        );
        let mut cam = open(&mut b, &id, PixelFormat::Yuyv);
        assert!(cam.grab().is_err());
        let r = log.records();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].code, Some(Code::FrameTruncated));
    }
}
