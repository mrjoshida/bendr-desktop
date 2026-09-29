use std::sync::Mutex;
use tauri::State;

#[cfg(target_os = "macos")]
pub mod macos {
    use std::sync::Mutex;
    // use syphon_rs::{SyphonServer, SyphonServerOptions};
    
    pub struct SyphonService {
        // server: Option<SyphonServer>,
        pub active: bool,
    }
    
    impl SyphonService {
        pub fn new() -> Self {
            Self {
                // server: None,
                active: false,
            }
        }
        
        pub fn publish_frame(&mut self, width: u32, height: u32, pixels: &[u8]) {
            if !self.active { return; }
            // TODO: implement actual syphon frame push once we have the metal context initialized
            // syphon-rs requires an active Metal Device/Command Queue or OpenGL context.
            // For now, this acts as the bridging stub.
        }
    }
}

#[cfg(target_os = "windows")]
pub mod windows {
    pub struct SpoutService {
        pub active: bool,
    }
    
    impl SpoutService {
        pub fn new() -> Self {
            Self { active: false }
        }
        
        pub fn publish_frame(&mut self, width: u32, height: u32, pixels: &[u8]) {
            if !self.active { return; }
            // TODO: spout2_rs push
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
    pub fn new() -> Self { Self { active: false } }
    pub fn publish_frame(&mut self, _width: u32, _height: u32, _pixels: &[u8]) {}
}

pub struct PublisherState(pub Mutex<FramePublisher>);
