//! Tauri command handlers for BENDR Desktop.
//!
//! Exposes IPC commands to the frontend webview for MIDI scanning,
//! connection management, and display/window queries.

use tauri::State;
use tokio::sync::Mutex;
use crate::midi::MidiService;
use crate::display;

/// Scans for available MIDI input ports (both hardware USB and network endpoints).
///
/// Returns a list of human-readable port names.
#[tauri::command]
pub async fn midi_scan_ports(
    service: State<'_, Mutex<MidiService>>,
) -> Result<Vec<String>, String> {
    let svc = service.lock().await;
    svc.scan_ports()
}

/// Connects to a named MIDI input port and starts streaming MIDI messages.
///
/// # Arguments
/// * `port_name` - The exact name of the port to connect to.
#[tauri::command]
pub async fn midi_connect(
    port_name: String,
    service: State<'_, Mutex<MidiService>>,
) -> Result<(), String> {
    let mut svc = service.lock().await;
    svc.connect_port(&port_name)
}

/// Disconnects from the currently active MIDI input port.
#[tauri::command]
pub async fn midi_disconnect(
    service: State<'_, Mutex<MidiService>>,
) -> Result<(), String> {
    let mut svc = service.lock().await;
    svc.disconnect()
}

/// Information describing an attached display monitor (legacy / placeholder).
#[derive(Debug, Clone, serde::Serialize)]
pub struct DisplayInfo {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

/// Retrieves available display information for multi-display / fullscreen output (legacy endpoint).
#[tauri::command]
pub async fn get_displays() -> Result<Vec<DisplayInfo>, String> {
    // Placeholder for Phase 1/2 multi-monitor output routing
    Ok(vec![DisplayInfo {
        id: "display-0".to_string(),
        name: "Primary Display".to_string(),
        width: 1440,
        height: 900,
        is_primary: true,
    }])
}

/// List all connected displays with their properties.
#[tauri::command]
pub async fn list_displays(app: tauri::AppHandle) -> Result<Vec<display::DisplayInfo>, String> {
    display::list_displays(&app)
}

/// Create output window on a specific display.
#[tauri::command]
pub async fn create_output_window(app: tauri::AppHandle, display_index: usize) -> Result<(), String> {
    display::create_output_window(&app, display_index)
}

/// Close the output window.
#[tauri::command]
pub async fn close_output_window(app: tauri::AppHandle) -> Result<(), String> {
    display::close_output_window(&app)
}

/// Check if output window is currently open.
#[tauri::command]
pub async fn is_output_open() -> bool {
    display::is_output_open()
}

/// Enable or disable the Syphon/Spout publisher
#[tauri::command]
pub fn toggle_publisher(state: tauri::State<'_, crate::syphon::PublisherState>, active: bool) {
    if let Ok(mut pub_state) = state.0.lock() {
        pub_state.active = active;
        log::info!("[BENDR Publisher] Active: {}", active);
    }
}

/// Receive a raw RGBA frame from JS and publish it
#[tauri::command]
pub fn publish_frame(state: tauri::State<'_, crate::syphon::PublisherState>, width: u32, height: u32, pixels: Vec<u8>) {
    if let Ok(mut pub_state) = state.0.lock() {
        if pub_state.active {
            pub_state.publish_frame(width, height, &pixels);
        }
    }
}
