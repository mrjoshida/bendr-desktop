//! Syphon / Spout video output module for BENDR Desktop.
//!
//! TODO: Phase 2 — Syphon output (macOS) / Spout (Windows) via platform-specific bindings

#![cfg(target_os = "macos")]

/// Placeholder service for Syphon video output on macOS.
pub struct SyphonService;

/// Stub initialization for the Syphon service.
pub fn init() -> Result<SyphonService, String> {
    log::info!("[BENDR Syphon] Syphon service not yet implemented");
    Ok(SyphonService)
}
