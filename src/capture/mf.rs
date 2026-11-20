//! The backend of Windows (Media Foundation), as a skeleton. The library is started and stopped here; listing the cameras and opening one need the COM interfaces of Media Foundation
//! (`IMFActivate`, `IMFSourceReader`), which are not written yet, and say so with `backend_unsupported`. Only built with the feature `mfoundation` on Windows, and nothing has been run on a real
//! camera.
//!
//! The only unsafe code is at the edge, in this file: two calls to `mfplat.dll` that take numbers and no pointers.
#![allow(unsafe_code)]

use super::{Camera, CameraId, CameraInfo, CapError, CaptureBackend, Code, FormatDesc};

#[link(name = "mfplat")]
extern "system" {
    fn MFStartup(version: u32, flags: u32) -> i32;
    fn MFShutdown() -> i32;
}

/// `MF_VERSION`: the version of the SDK in the high word and the version of the API in the low one.
const MF_VERSION: u32 = 0x0002_0070;
/// `MFSTARTUP_FULL`: start every part of the platform.
const MFSTARTUP_FULL: u32 = 0;

/// Media Foundation, started; stopped again when it goes out of scope.
struct Platform;

impl Platform {
    fn start() -> Result<Platform, CapError> {
        // SAFETY: `MFStartup` takes two numbers and returns an HRESULT; it is balanced by the `MFShutdown` of `Drop`.
        let hr = unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL) };
        if hr < 0 {
            return Err(CapError::new(
                Code::BackendUnavailable,
                format!("mfoundation: MFStartup failed with 0x{hr:08X}"),
            ));
        }
        Ok(Platform)
    }
}

impl Drop for Platform {
    fn drop(&mut self) {
        // SAFETY: balances the `MFStartup` that made this value; takes no arguments.
        unsafe {
            MFShutdown();
        }
    }
}

/// The backend of Media Foundation.
#[derive(Debug, Default)]
pub struct MfBackend;

impl CaptureBackend for MfBackend {
    fn name(&self) -> &'static str {
        "mfoundation"
    }

    fn enumerate(&mut self) -> Result<Vec<CameraInfo>, CapError> {
        let _platform = Platform::start()?;
        Err(CapError::new(
            Code::BackendUnsupported,
            "mfoundation: listing the cameras (MFEnumDeviceSources) is not written yet",
        ))
    }

    fn open(&mut self, _id: &CameraId, _format: FormatDesc) -> Result<Box<dyn Camera>, CapError> {
        Err(CapError::new(
            Code::BackendUnsupported,
            "mfoundation: opening a camera for frames (IMFSourceReader) is not written yet",
        ))
    }
}
