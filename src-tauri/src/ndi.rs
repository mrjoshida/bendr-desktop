//! NDI video input/output module for BENDR Desktop.
//!
//! TODO: Phase 3 — NDI video I/O via ndi-sdk bindings

/// Placeholder service for NDI video input/output.
pub struct NdiService;

/// Stub initialization for the NDI service.
pub fn init() -> Result<NdiService, String> {
    log::info!("[BENDR NDI] NDI service not yet implemented");
    Ok(NdiService)
}
