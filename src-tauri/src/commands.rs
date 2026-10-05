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

/// Enable or disable the Syphon/Spout and NDI publishers
#[tauri::command]
pub fn toggle_publisher(
    syphon_state: tauri::State<'_, crate::syphon::PublisherState>,
    ndi_state: tauri::State<'_, crate::ndi::NdiState>,
    active: bool,
) -> Result<(), String> {
    if let Ok(mut pub_state) = syphon_state.0.lock() {
        pub_state.active = active;
    }
    if let Ok(mut n_state) = ndi_state.0.lock() {
        n_state.active = active;
    }
    log::info!("[BENDR Publisher] Active: {}", active);
    Ok(())
}

/// Receive a raw RGBA frame from JS and publish it via Syphon/Spout and NDI.
///
/// Uses std::sync::Mutex to hold the publishers briefly during transmission.
#[tauri::command]
pub fn publish_frame(
    syphon_state: tauri::State<'_, crate::syphon::PublisherState>,
    ndi_state: tauri::State<'_, crate::ndi::NdiState>,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
) -> Result<(), String> {
    if let Ok(mut pub_state) = syphon_state.0.lock() {
        if pub_state.active {
            pub_state.publish_frame(width, height, &pixels);
        }
    }
    if let Ok(mut n_state) = ndi_state.0.lock() {
        if n_state.active {
            n_state.publish_frame(width, height, &pixels);
        }
    }
    Ok(())
}

use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn pick_file(app: tauri::AppHandle, multiple: bool) -> Result<Vec<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    
    if multiple {
        app.dialog().file().pick_files(move |paths| {
            let res = match paths {
                Some(p) => p.into_iter().map(|f| f.into_path().unwrap().to_string_lossy().to_string()).collect(),
                None => vec![],
            };
            let _ = tx.send(res);
        });
    } else {
        app.dialog().file().pick_file(move |path| {
            let res = match path {
                Some(p) => vec![p.into_path().unwrap().to_string_lossy().to_string()],
                None => vec![],
            };
            let _ = tx.send(res);
        });
    }
    
    rx.await.map_err(|e| e.to_string())
}
