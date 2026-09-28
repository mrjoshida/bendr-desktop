pub mod commands;
pub mod display;
pub mod midi;
pub mod ndi;
pub mod osc;
pub mod syphon;

use tauri::Manager;

pub fn run() {
    // Initialize logging if configured
    let _ = env_logger::try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::midi_scan_ports,
            commands::midi_connect,
            commands::midi_disconnect,
            commands::get_displays,
            commands::list_displays,
            commands::create_output_window,
            commands::close_output_window,
            commands::is_output_open,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Initialize MIDI service
            let midi_service = midi::init(app_handle.clone())
                .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error>)?;

            // Register MIDI service in managed state for command handlers
            app.manage(tokio::sync::Mutex::new(midi_service));

            // Spawn background task for MIDI monitoring / network discovery
            tauri::async_runtime::spawn(async move {
                log::info!("[BENDR] MIDI background task spawned");
                // Phase 1b: Background mDNS / Bonjour network MIDI service discovery
            });

            log::info!("[BENDR] Setup complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
