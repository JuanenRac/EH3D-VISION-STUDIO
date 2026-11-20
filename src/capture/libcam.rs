//! The backend for the CSI cameras of the Raspberry Pi and the CM5 (libcamera), as a skeleton. libcamera has a C++ interface and no stable C one, so this program talks to a small C shim
//! (`libeh3d_libcamera`, to be written against the libcamera of the system) through the functions declared below; the cameras are listed through it, and opening one for frames says
//! `backend_unsupported` until the shim has it. Only built with the feature `libcamera` on Linux, which also needs the shim to be there at link time, and nothing has been run on a real camera.
//!
//! The only unsafe code is at the edge, in this file: the calls to the shim, each with a pointer to a buffer of this file that lives for the whole call and a length that is its size.
#![allow(unsafe_code)]

use super::fourcc;
use super::{Camera, CameraId, CameraInfo, CapError, CaptureBackend, Code, FormatDesc};
use std::ffi::{c_char, c_int, c_void};

#[link(name = "eh3d_libcamera")]
extern "C" {
    /// Starts the camera manager; null when libcamera is not there.
    fn eh3d_lc_open() -> *mut c_void;
    fn eh3d_lc_close(handle: *mut c_void);
    /// How many cameras there are.
    fn eh3d_lc_count(handle: *mut c_void) -> c_int;
    /// The name of the camera `index` written to `buf` of `len` bytes, ending in a zero; the length written or -1.
    fn eh3d_lc_name(handle: *mut c_void, index: c_int, buf: *mut c_char, len: c_int) -> c_int;
    /// The formats of the camera `index` as `width height fps fourcc` quadruples written to `out` (room for `max` quadruples); how many were written or -1.
    fn eh3d_lc_formats(handle: *mut c_void, index: c_int, out: *mut u32, max: c_int) -> c_int;
}

/// The camera manager of libcamera, closed when it goes out of scope.
struct Manager(*mut c_void);

impl Manager {
    fn start() -> Result<Manager, CapError> {
        // SAFETY: takes no arguments; returns a handle or null.
        let h = unsafe { eh3d_lc_open() };
        if h.is_null() {
            return Err(CapError::new(
                Code::BackendUnavailable,
                "libcamera: the camera manager did not start",
            ));
        }
        Ok(Manager(h))
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        // SAFETY: the handle came from `eh3d_lc_open` and is closed once, here.
        unsafe { eh3d_lc_close(self.0) }
    }
}

/// The backend of libcamera.
#[derive(Debug, Default)]
pub struct LibcameraBackend;

impl CaptureBackend for LibcameraBackend {
    fn name(&self) -> &'static str {
        "libcamera"
    }

    fn enumerate(&mut self) -> Result<Vec<CameraInfo>, CapError> {
        let m = Manager::start()?;
        // SAFETY: `m.0` is the live handle of `m`.
        let count = unsafe { eh3d_lc_count(m.0) };
        let mut out = Vec::new();
        for index in 0..count.clamp(0, 16) {
            let mut name = [0 as c_char; 128];
            // SAFETY: `name` is a buffer of 128 bytes and the length passed is 128.
            let n = unsafe { eh3d_lc_name(m.0, index, name.as_mut_ptr(), 128) };
            if n < 0 {
                continue;
            }
            let bytes: Vec<u8> = name
                .iter()
                .take_while(|&&c| c != 0)
                .map(|&c| c as u8)
                .collect();
            let mut quads = [[0u32; 4]; 64];
            // SAFETY: `quads` has room for 64 quadruples and the maximum passed is 64.
            let got = unsafe { eh3d_lc_formats(m.0, index, quads.as_mut_ptr().cast::<u32>(), 64) };
            let formats: Vec<FormatDesc> = quads
                .iter()
                .take(got.clamp(0, 64) as usize)
                .filter_map(|q| {
                    Some(FormatDesc {
                        width: q[0],
                        height: q[1],
                        fps: q[2],
                        pixel: fourcc::format_from_code(q[3])?,
                    })
                })
                .collect();
            let formats = fourcc::clean(&formats);
            if formats.is_empty() {
                continue;
            }
            out.push(CameraInfo {
                name: String::from_utf8_lossy(&bytes).into_owned(),
                id: CameraId {
                    vendor: 0,
                    product: 0,
                    serial: String::new(),
                    port: format!("csi{index}"),
                },
                formats,
            });
        }
        Ok(out)
    }

    fn open(&mut self, _id: &CameraId, _format: FormatDesc) -> Result<Box<dyn Camera>, CapError> {
        Err(CapError::new(
            Code::BackendUnsupported,
            "libcamera: opening a camera for frames (a request and buffer loop in the shim) is not written yet",
        ))
    }
}
