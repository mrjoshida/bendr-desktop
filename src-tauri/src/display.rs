//! Display enumeration and output window management for BENDR Desktop.
//!
//! Manages native multi-monitor detection and creation/fullscreening of
//! dedicated secondary output windows for projectors, capture cards,
//! and external monitors.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// Information about a connected display.
#[derive(Debug, Clone, Serialize)]
pub struct DisplayInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub scale_factor: f64,
    pub is_primary: bool,
}

static OUTPUT_WINDOW_OPEN: AtomicBool = AtomicBool::new(false);

/// List all connected displays with their geometry and attributes.
pub fn list_displays(app: &AppHandle) -> Result<Vec<DisplayInfo>, String> {
    let monitors = app
        .available_monitors()
        .map_err(|e| format!("Failed to enumerate displays: {e}"))?;

    let primary = app
        .primary_monitor()
        .map_err(|e| format!("Failed to get primary monitor: {e}"))?
        .map(|m| m.position().clone());

    let mut displays = Vec::new();
    for (idx, monitor) in monitors.iter().enumerate() {
        let pos = monitor.position();
        let size = monitor.size();
        let is_primary = primary
            .as_ref()
            .map(|p| p.x == pos.x && p.y == pos.y)
            .unwrap_or(idx == 0);

        displays.push(DisplayInfo {
            name: monitor
                .name()
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("Display {}", idx + 1)),
            width: size.width,
            height: size.height,
            x: pos.x,
            y: pos.y,
            scale_factor: monitor.scale_factor(),
            is_primary,
        });
    }
    Ok(displays)
}

/// Create the output window on a specific display.
pub fn create_output_window(app: &AppHandle, display_index: usize) -> Result<(), String> {
    if OUTPUT_WINDOW_OPEN.load(Ordering::SeqCst) {
        // Close existing first
        close_output_window(app)?;
    }

    let monitors = app
        .available_monitors()
        .map_err(|e| format!("Failed to enumerate displays: {e}"))?;

    let monitor = monitors.get(display_index).ok_or_else(|| {
        format!(
            "Display index {display_index} out of range (found {} displays)",
            monitors.len()
        )
    })?;

    let pos = monitor.position();
    let size = monitor.size();

    log::info!(
        "[BENDR Display] Creating output window on display {display_index} at ({}, {}) size {}x{}",
        pos.x,
        pos.y,
        size.width,
        size.height
    );

    let output_window = WebviewWindowBuilder::new(
        app,
        "output",
        WebviewUrl::App("output.html".into()),
    )
    .title("BENDR Output")
    .inner_size(size.width as f64, size.height as f64)
    .position(pos.x as f64, pos.y as f64)
    .decorations(false)
    .always_on_top(true)
    .focused(false) // Keep focus on controls window
    .build()
    .map_err(|e| format!("Failed to create output window: {e}"))?;

    // Go fullscreen after creation
    output_window
        .set_fullscreen(true)
        .map_err(|e| format!("Failed to set fullscreen: {e}"))?;

    OUTPUT_WINDOW_OPEN.store(true, Ordering::SeqCst);
    log::info!("[BENDR Display] Output window created and fullscreen on display {display_index}");
    Ok(())
}

/// Close the output window.
pub fn close_output_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("output") {
        // Notify output window to clean up WebRTC
        let _ = window.emit("output:close", ());
        window
            .close()
            .map_err(|e| format!("Failed to close output window: {e}"))?;
    }
    OUTPUT_WINDOW_OPEN.store(false, Ordering::SeqCst);
    log::info!("[BENDR Display] Output window closed");
    Ok(())
}

/// Check if the output window is currently active.
pub fn is_output_open() -> bool {
    OUTPUT_WINDOW_OPEN.load(Ordering::SeqCst)
}
