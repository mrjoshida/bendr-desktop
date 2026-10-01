//! Syphon (macOS) and Spout (Windows) frame publishing service.
//!
//! Receives raw RGBA pixel buffers from the JS bridge and publishes
//! them to other applications via native GPU texture sharing protocols.
//!
//! - macOS: Syphon framework via `syphon-core` + `syphon-metal`
//! - Windows: Spout2 via `spout2-rs`

#[cfg(target_os = "macos")]
pub mod macos {
    use objc2::runtime::AnyObject;
    use objc2_metal::{
        MTLCommandBuffer, MTLCommandQueue, MTLCreateSystemDefaultDevice, MTLDevice,
        MTLOrigin, MTLPixelFormat, MTLRegion, MTLSize, MTLStorageMode, MTLTexture,
        MTLTextureDescriptor, MTLTextureUsage,
    };
    use syphon_core::SyphonServer;
    use std::ptr::NonNull;

    /// macOS Syphon frame publisher.
    ///
    /// Publishes BENDR's program output as a Syphon source that any
    /// Syphon-aware app (Resolume, VDMX, MadMapper, etc.) can receive.
    pub struct SyphonService {
        server: Option<SyphonServer>,
        device: Option<objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn MTLDevice>>>,
        queue: Option<objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn MTLCommandQueue>>>,
        texture: Option<objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn MTLTexture>>>,
        tex_width: u32,
        tex_height: u32,
        pub active: bool,
    }

    impl SyphonService {
        pub fn new() -> Self {
            Self {
                server: None,
                device: None,
                queue: None,
                texture: None,
                tex_width: 0,
                tex_height: 0,
                active: false,
            }
        }

        /// Initialize Metal device and Syphon server.
        /// Called once when the publisher is first activated.
        fn ensure_initialized(&mut self, width: u32, height: u32) -> Result<(), String> {
            if self.server.is_some() && self.tex_width == width && self.tex_height == height {
                return Ok(());
            }

            // Stop any existing server
            if let Some(ref server) = self.server {
                server.stop();
            }

            log::info!("[BENDR Syphon] Initializing Metal device and Syphon server ({}x{})", width, height);

            unsafe {
                let device = MTLCreateSystemDefaultDevice()
                    .ok_or("Metal is not available on this system")?;
                let queue = device
                    .newCommandQueue()
                    .ok_or("Failed to create Metal command queue")?;

                // Create a managed Metal texture for CPU -> GPU upload (BGRA8Unorm)
                let desc = MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                    MTLPixelFormat::BGRA8Unorm,
                    width as usize,
                    height as usize,
                    false,
                );
                desc.setStorageMode(MTLStorageMode::Managed);
                desc.setUsage(MTLTextureUsage::ShaderRead);
                let texture = device
                    .newTextureWithDescriptor(&desc)
                    .ok_or("Failed to create Metal texture")?;

                // Create Syphon server
                use objc2::runtime::ProtocolObject;
                let device_ptr = (&*device as *const ProtocolObject<dyn MTLDevice>) as *mut AnyObject;
                let server = SyphonServer::new_with_name_and_device(
                    "BENDR",
                    device_ptr,
                    width,
                    height,
                )
                .map_err(|e| format!("Failed to create Syphon server: {e:?}"))?;

                log::info!("[BENDR Syphon] Server '{}' created successfully", server.name());

                self.device = Some(device);
                self.queue = Some(queue);
                self.texture = Some(texture);
                self.server = Some(server);
                self.tex_width = width;
                self.tex_height = height;

                Ok(())
            }
        }

        /// Publish a single RGBA frame to connected Syphon clients.
        ///
        /// Converts RGBA -> BGRA, uploads to a Metal texture, and publishes
        /// via the Syphon server. Skips work if no clients are connected.
        ///
        /// # Arguments
        /// * `width` - Frame width in pixels
        /// * `height` - Frame height in pixels
        /// * `pixels` - Raw RGBA pixel data (4 bytes per pixel)
        pub fn publish_frame(&mut self, width: u32, height: u32, pixels: &[u8]) {
            if !self.active {
                return;
            }

            // Lazy-initialize on first frame (or resolution change)
            if let Err(e) = self.ensure_initialized(width, height) {
                log::error!("[BENDR Syphon] Init failed: {e}");
                self.active = false;
                return;
            }

            let server = self.server.as_ref().unwrap();

            // Skip GPU work if nobody is listening
            if !server.has_clients() {
                return;
            }

            let queue = self.queue.as_ref().unwrap();
            let texture = self.texture.as_ref().unwrap();

            // Convert RGBA -> BGRA in-place (Syphon's native format)
            let expected_len = (width * height * 4) as usize;
            if pixels.len() < expected_len {
                return;
            }
            let mut bgra = pixels[..expected_len].to_vec();
            for chunk in bgra.chunks_exact_mut(4) {
                chunk.swap(0, 2); // R <-> B
            }

            // Upload pixel buffer to Metal texture and publish
            unsafe {
                texture.replaceRegion_mipmapLevel_withBytes_bytesPerRow(
                    MTLRegion {
                        origin: MTLOrigin { x: 0, y: 0, z: 0 },
                        size: MTLSize {
                            width: width as usize,
                            height: height as usize,
                            depth: 1,
                        },
                    },
                    0,
                    NonNull::new_unchecked(bgra.as_ptr() as *mut _),
                    (width * 4) as usize,
                );

                // Create command buffer and publish
                if let Some(cmd_buf) = queue.commandBuffer() {
                    use objc2::runtime::ProtocolObject;
                    let tex_ptr =
                        (&**texture as *const ProtocolObject<dyn MTLTexture>) as *mut AnyObject;
                    let cmd_ptr = (&*cmd_buf as *const ProtocolObject<dyn MTLCommandBuffer>)
                        as *mut AnyObject;
                    server.publish_metal_texture(tex_ptr, cmd_ptr);
                    cmd_buf.commit();
                }
            }
        }

        /// Stop the Syphon server and release resources.
        pub fn stop(&mut self) {
            if let Some(ref server) = self.server {
                log::info!("[BENDR Syphon] Stopping server");
                server.stop();
            }
            self.server = None;
            self.texture = None;
            self.queue = None;
            self.device = None;
            self.active = false;
        }
    }

    impl Drop for SyphonService {
        fn drop(&mut self) {
            self.stop();
        }
    }

    // Safety: SyphonService wraps Metal and Syphon objects which are thread-safe
    // in macOS and protected behind a Mutex in PublisherState.
    unsafe impl Send for SyphonService {}
    unsafe impl Sync for SyphonService {}
}

#[cfg(target_os = "windows")]
pub mod windows {
    /// Windows Spout2 frame publisher.
    pub struct SpoutService {
        pub active: bool,
    }

    impl SpoutService {
        pub fn new() -> Self {
            Self { active: false }
        }

        pub fn publish_frame(&mut self, _width: u32, _height: u32, _pixels: &[u8]) {
            if !self.active {
                return;
            }
            // TODO: Phase 2 — Spout2 sender via spout2-rs
        }
    }
}

#[cfg(target_os = "macos")]
pub use macos::SyphonService as FramePublisher;

#[cfg(target_os = "windows")]
pub use windows::SpoutService as FramePublisher;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub struct FramePublisher {
    pub active: bool,
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl FramePublisher {
    pub fn new() -> Self {
        Self { active: false }
    }
    pub fn publish_frame(&mut self, _width: u32, _height: u32, _pixels: &[u8]) {}
}

/// Managed Tauri state wrapping the platform-specific frame publisher.
///
/// Uses `std::sync::Mutex` to hold the publisher briefly during
/// Metal texture upload and Syphon publishing calls.
pub struct PublisherState(pub std::sync::Mutex<FramePublisher>);
