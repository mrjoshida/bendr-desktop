//! Tauri command handlers for BENDR Desktop.
//!
//! Exposes IPC commands to the frontend webview for MIDI scanning,
//! connection management, and display/window queries.

use tauri::State;
use tokio::sync::Mutex;
use crate::midi::MidiService;

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

/// Information describing an attached display monitor.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DisplayInfo {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

/// Retrieves available display information for multi-display / fullscreen output.
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
