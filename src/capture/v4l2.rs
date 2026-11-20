//! The backend of Linux (V4L2), as a skeleton. It lists the cameras by asking the kernel (`QUERYCAP`, `ENUM_FMT`, `ENUM_FRAMESIZES`, `ENUM_FRAMEINTERVALS`) and cleans the lists the way the
//! other backends are cleaned; opening a camera for frames is not written yet and says so with `backend_unsupported`. Only built with the feature `v4l2` on Linux, and nothing has been run
//! on a real camera.
//!
//! The only unsafe code is at the edge, in this file: four calls to the C library (`open`, `close`, `ioctl`) and the plain structures the kernel fills in. Each structure has the size the
//! kernel header gives, checked when the program is built, and each call is made with a pointer to a structure of this file that lives for the whole call.
#![allow(unsafe_code)]

use super::fourcc;
use super::{Camera, CameraId, CameraInfo, CapError, CaptureBackend, Code, FormatDesc};
use std::ffi::{c_char, c_int, c_ulong, c_void, CString};

extern "C" {
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn ioctl(fd: c_int, request: c_ulong, arg: *mut c_void) -> c_int;
}

const O_RDWR: c_int = 2;
const O_CLOEXEC: c_int = 0o2_000_000;

#[repr(C)]
#[derive(Clone, Copy)]
struct Capability {
    driver: [u8; 16],
    card: [u8; 32],
    bus_info: [u8; 32],
    version: u32,
    capabilities: u32,
    device_caps: u32,
    reserved: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FmtDesc {
    index: u32,
    kind: u32,
    flags: u32,
    description: [u8; 32],
    pixelformat: u32,
    mbus_code: u32,
    reserved: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FrmSizeEnum {
    index: u32,
    pixel_format: u32,
    kind: u32,
    width: u32,
    height: u32,
    rest: [u32; 4],
    reserved: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FrmIvalEnum {
    index: u32,
    pixel_format: u32,
    width: u32,
    height: u32,
    kind: u32,
    numerator: u32,
    denominator: u32,
    rest: [u32; 4],
    reserved: [u32; 2],
}

const _: () = assert!(std::mem::size_of::<Capability>() == 104);
const _: () = assert!(std::mem::size_of::<FmtDesc>() == 64);
const _: () = assert!(std::mem::size_of::<FrmSizeEnum>() == 44);
const _: () = assert!(std::mem::size_of::<FrmIvalEnum>() == 52);

/// The number of an `ioctl` request on the usual layout of Linux (x86, ARM): direction in the top two bits (1 write, 2 read), the size of the argument, the type and the number.
const fn ioc(dir: c_ulong, nr: c_ulong, size: usize) -> c_ulong {
    (dir << 30) | ((size as c_ulong) << 16) | ((b'V' as c_ulong) << 8) | nr
}

const QUERYCAP: c_ulong = ioc(2, 0, std::mem::size_of::<Capability>());
const ENUM_FMT: c_ulong = ioc(3, 2, std::mem::size_of::<FmtDesc>());
const ENUM_FRAMESIZES: c_ulong = ioc(3, 74, std::mem::size_of::<FrmSizeEnum>());
const ENUM_FRAMEINTERVALS: c_ulong = ioc(3, 75, std::mem::size_of::<FrmIvalEnum>());
const BUF_TYPE_VIDEO_CAPTURE: u32 = 1;
const CAP_VIDEO_CAPTURE: u32 = 0x1;
const CAP_DEVICE_CAPS: u32 = 0x8000_0000;
const FRMSIZE_DISCRETE: u32 = 1;
const FRMIVAL_DISCRETE: u32 = 1;

fn text(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// A device file that is closed when it goes out of scope.
struct Device(c_int);

impl Device {
    fn open(path: &str) -> Option<Device> {
        let c = CString::new(path).ok()?;
        // SAFETY: `c` is a NUL-terminated string that lives through the call; `open` reads it and returns a descriptor or -1.
        let fd = unsafe { open(c.as_ptr(), O_RDWR | O_CLOEXEC) };
        (fd >= 0).then_some(Device(fd))
    }

    fn query<T>(&self, request: c_ulong, arg: &mut T) -> bool {
        // SAFETY: `arg` points to a structure of this file whose size is the one the request number was made from; it is valid and exclusively borrowed for the call.
        unsafe { ioctl(self.0, request, (arg as *mut T).cast::<c_void>()) == 0 }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        // SAFETY: the descriptor was returned by `open` and is closed once, here.
        unsafe {
            close(self.0);
        }
    }
}

fn zeroed<T: Copy>() -> T {
    // SAFETY: every structure of this file is plain numbers and arrays of numbers, for which all-zero bytes are a value.
    unsafe { std::mem::zeroed() }
}

fn formats_of(dev: &Device) -> Vec<FormatDesc> {
    let mut out = Vec::new();
    for fi in 0..64 {
        let mut f: FmtDesc = zeroed();
        f.index = fi;
        f.kind = BUF_TYPE_VIDEO_CAPTURE;
        if !dev.query(ENUM_FMT, &mut f) {
            break;
        }
        let Some(pixel) = fourcc::format_from_code(f.pixelformat) else {
            continue;
        };
        for si in 0..64 {
            let mut s: FrmSizeEnum = zeroed();
            s.index = si;
            s.pixel_format = f.pixelformat;
            if !dev.query(ENUM_FRAMESIZES, &mut s) {
                break;
            }
            if s.kind != FRMSIZE_DISCRETE {
                break;
            }
            let mut listed = 0;
            for ii in 0..64 {
                let mut v: FrmIvalEnum = zeroed();
                v.index = ii;
                v.pixel_format = f.pixelformat;
                v.width = s.width;
                v.height = s.height;
                if !dev.query(ENUM_FRAMEINTERVALS, &mut v) || v.kind != FRMIVAL_DISCRETE {
                    break;
                }
                listed += 1;
                out.push(FormatDesc {
                    width: s.width,
                    height: s.height,
                    fps: v.denominator / v.numerator.max(1),
                    pixel,
                });
            }
            if listed == 0 {
                // the camera did not say a rate: it is listed so that the clean-up drops it and the log can say why
                out.push(FormatDesc {
                    width: s.width,
                    height: s.height,
                    fps: 0,
                    pixel,
                });
            }
        }
    }
    out
}

/// The backend of V4L2.
#[derive(Debug, Default)]
pub struct V4l2Backend;

impl CaptureBackend for V4l2Backend {
    fn name(&self) -> &'static str {
        "v4l2"
    }

    fn enumerate(&mut self) -> Result<Vec<CameraInfo>, CapError> {
        let mut out = Vec::new();
        for n in 0..64 {
            let Some(dev) = Device::open(&format!("/dev/video{n}")) else {
                continue;
            };
            let mut cap: Capability = zeroed();
            if !dev.query(QUERYCAP, &mut cap) {
                continue;
            }
            let caps = if cap.capabilities & CAP_DEVICE_CAPS != 0 {
                cap.device_caps
            } else {
                cap.capabilities
            };
            if caps & CAP_VIDEO_CAPTURE == 0 {
                continue;
            }
            let formats = fourcc::clean(&formats_of(&dev));
            if formats.is_empty() {
                continue;
            }
            out.push(CameraInfo {
                name: text(&cap.card),
                id: CameraId {
                    vendor: 0,
                    product: 0,
                    serial: String::new(),
                    port: fourcc::port_from_bus_info(&text(&cap.bus_info)),
                },
                formats,
            });
        }
        Ok(out)
    }

    fn open(&mut self, _id: &CameraId, _format: FormatDesc) -> Result<Box<dyn Camera>, CapError> {
        Err(CapError::new(
            Code::BackendUnsupported,
            "v4l2: opening a camera for frames (buffers and streaming) is not written yet",
        ))
    }
}
