//! NDI (Network Device Interface) module for BENDR Desktop.
//!
//! Provides a dynamically loaded NDI publisher to broadcast
//! video frames over the local network without requiring static
//! linkage to proprietary NDI SDKs.

use libloading::{Library, Symbol};
use ndi_sdk::sys::*;
use std::ffi::CString;

pub struct NdiService {
    pub active: bool,
    lib: Option<Library>,
    v5: Option<*const NDIlib_v5>,
    sender: Option<NDIlib_send_instance_t>,
}

unsafe impl Send for NdiService {}
unsafe impl Sync for NdiService {}

impl NdiService {
    pub fn new() -> Self {
        Self {
            active: false,
            lib: None,
            v5: None,
            sender: None,
        }
    }

    /// Attempt to load the NDI dynamic library from standard OS paths.
    unsafe fn load_lib() -> Result<Library, String> {
        let lib_name = if cfg!(target_os = "macos") {
            "libndi.dylib"
        } else if cfg!(windows) {
            "Processing.NDI.Lib.x64.dll"
        } else {
            "libndi.so.6"
        };

        // Try standard linker path first
        if let Ok(lib) = Library::new(lib_name) {
            return Ok(lib);
        }

        // Try common macOS installation path
        if cfg!(target_os = "macos") {
            if let Ok(lib) = Library::new("/usr/local/lib/libndi.dylib") {
                return Ok(lib);
            }
        }

        Err("NDI runtime not found. Please install NDI Tools.".to_string())
    }

    /// Dynamically load NDI and create a sender instance if it doesn't exist.
    /// Fails gracefully if NDI is not installed on the host system.
    fn ensure_initialized(&mut self) -> Result<(), String> {
        if self.lib.is_none() {
            log::info!("[BENDR NDI] Attempting to dynamically load NDI runtime...");
            unsafe {
                let lib = Self::load_lib()?;
                let load_fn: Symbol<unsafe extern "C" fn() -> *const NDIlib_v5> =
                    lib.get(b"NDIlib_v5_load\0").map_err(|e| e.to_string())?;

                let v5 = load_fn();
                if v5.is_null() {
                    return Err("NDIlib_v5_load returned null".to_string());
                }

                let init_fn = (*v5).initialize.ok_or("Missing NDI initialize")?;
                if !init_fn() {
                    return Err("NDI initialize returned false".to_string());
                }

                self.v5 = Some(v5);
                self.lib = Some(lib);
                log::info!("[BENDR NDI] Successfully loaded NDI runtime");
            }
        }

        if self.sender.is_none() {
            if let Some(v5) = self.v5 {
                log::info!("[BENDR NDI] Creating NDI Sender 'BENDR Desktop'");
                unsafe {
                    let create_fn = (*v5).send_create.ok_or("Missing NDI send_create")?;
                    let name = CString::new("BENDR Desktop").unwrap();
                    let desc = NDIlib_send_create_t {
                        p_ndi_name: name.as_ptr(),
                        p_groups: std::ptr::null(),
                        clock_video: true,
                        clock_audio: false,
                    };
                    let sender = create_fn(&desc);
                    if sender.is_null() {
                        return Err("Failed to create NDI sender (returned null)".to_string());
                    }
                    self.sender = Some(sender);
                }
            }
        }

        Ok(())
    }

    /// Publish a single RGBA frame over NDI.
    pub fn publish_frame(&mut self, width: u32, height: u32, pixels: &[u8]) {
        if !self.active {
            return;
        }

        if let Err(e) = self.ensure_initialized() {
            log::warn!("[BENDR NDI] Disabled: {}", e);
            self.active = false;
            return;
        }

        if let (Some(v5), Some(sender)) = (self.v5, self.sender) {
            let expected_len = (width * height * 4) as usize;
            if pixels.len() < expected_len {
                return;
            }

            unsafe {
                let send_video_fn = (*v5).send_send_video_v2;
                if let Some(send_video) = send_video_fn {
                    let frame = NDIlib_video_frame_v2_t {
                        xres: width as i32,
                        yres: height as i32,
                        FourCC: NDIlib_FourCC_video_type_e::NDIlib_FourCC_video_type_RGBA,
                        frame_rate_N: 60,
                        frame_rate_D: 1,
                        picture_aspect_ratio: (width as f32) / (height as f32),
                        frame_format_type: NDIlib_frame_format_type_e::NDIlib_frame_format_type_progressive,
                        timecode: !0, // NDIlib_send_timecode_synthesize
                        p_data: pixels.as_ptr() as *mut u8,
                        __bindgen_anon_1: NDIlib_video_frame_v2_t__bindgen_ty_1 {
                            line_stride_in_bytes: (width * 4) as i32,
                        },
                        p_metadata: std::ptr::null(),
                        timestamp: 0,
                    };

                    send_video(sender, &frame);
                }
            }
        }
    }

    pub fn stop(&mut self) {
        log::info!("[BENDR NDI] Stopping NDI sender");
        if let (Some(v5), Some(sender)) = (self.v5, self.sender) {
            unsafe {
                if let Some(destroy_fn) = (*v5).send_destroy {
                    destroy_fn(sender);
                }
            }
        }
        self.sender = None;
        self.active = false;
    }
}

impl Drop for NdiService {
    fn drop(&mut self) {
        self.stop();
        if let Some(v5) = self.v5 {
            unsafe {
                if let Some(destroy_fn) = (*v5).destroy {
                    destroy_fn();
                }
            }
        }
    }
}

/// Managed Tauri state wrapping the NDI publisher.
pub struct NdiState(pub std::sync::Mutex<NdiService>);
