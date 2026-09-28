//! MIDI input and connection service for BENDR Desktop.
//!
//! Provides unified access to local USB MIDI controllers and network
//! MIDI endpoints (Apple MIDI / RTP-MIDI), bridging incoming events
//! into Tauri webview event dispatches.

use midir::{MidiInput, MidiInputConnection};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Payload emitted with the `midi:message` event.
#[derive(Debug, Clone, Serialize)]
pub struct MidiMessagePayload {
    /// MIDI channel (0-15) derived from lower 4 bits of the status byte.
    pub channel: u8,
    /// Raw status byte (e.g. 0x90 for Note On, 0xB0 for CC).
    pub status: u8,
    /// First data byte (note number, controller index, etc.).
    pub data1: u8,
    /// Second data byte (velocity, value, etc.).
    pub data2: u8,
}

/// Service managing MIDI inputs and connection state.
pub struct MidiService {
    /// Tauri application handle for emitting events.
    app_handle: AppHandle,
    /// Active MIDI input connection, if any.
    connection: Option<MidiInputConnection<()>>,
    /// Discovered network MIDI session endpoints (Phase 1b).
    pub network_endpoints: Vec<String>,
}

// =========================================================================
// TODO: Phase 1b — RTP-MIDI / mDNS Discovery
// -------------------------------------------------------------------------
// 1. Use the `mdns-sd` crate to query and advertise Bonjour MIDI sessions
//    under `_apple-midi._udp.local.`.
// 2. Discover iOS / iPad / Mac network MIDI sessions across the local WiFi.
// 3. Implement RTP-MIDI packet parser/streamer or connect via native CoreMIDI
//    network session endpoint on macOS.
// 4. Expose detected endpoints in `network_endpoints` and automatically
//    populate them in `scan_ports()`.
// =========================================================================

/// Initializes the MIDI service and inspects currently available input ports.
///
/// # Arguments
/// * `app_handle` - The Tauri application handle used to dispatch events.
pub fn init(app_handle: AppHandle) -> Result<MidiService, String> {
    log::info!("[BENDR MIDI] Initializing MIDI subsystem...");

    let midi_in = MidiInput::new("bendr-midi-init")
        .map_err(|e| format!("Failed to create MIDI input: {e}"))?;

    let ports = midi_in.ports();
    log::info!("[BENDR MIDI] Found {} available MIDI input port(s)", ports.len());
    for (idx, port) in ports.iter().enumerate() {
        if let Ok(name) = midi_in.port_name(port) {
            log::info!("[BENDR MIDI]   [{idx}] {name}");
        }
    }

    Ok(MidiService {
        app_handle,
        connection: None,
        network_endpoints: Vec::new(),
    })
}

impl MidiService {
    /// Scans for available MIDI input ports (both hardware USB and network endpoints).
    ///
    /// # Returns
    /// A vector of port name strings.
    pub fn scan_ports(&self) -> Result<Vec<String>, String> {
        let midi_in = MidiInput::new("bendr-midi-scan")
            .map_err(|e| format!("Failed to create MIDI input scanner: {e}"))?;

        let mut port_names = Vec::new();
        for port in midi_in.ports() {
            if let Ok(name) = midi_in.port_name(&port) {
                port_names.push(name);
            }
        }

        // Include any network endpoints discovered via mDNS (Phase 1b)
        for endpoint in &self.network_endpoints {
            port_names.push(format!("[Network] {endpoint}"));
        }

        log::info!("[BENDR MIDI] Scanned {} port(s)", port_names.len());
        Ok(port_names)
    }

    /// Connects to a named MIDI input port and starts streaming events.
    ///
    /// # Arguments
    /// * `port_name` - The name of the port to connect to.
    pub fn connect_port(&mut self, port_name: &str) -> Result<(), String> {
        // Disconnect any existing connection first
        self.disconnect()?;

        log::info!("[BENDR MIDI] Connecting to port: {port_name}");

        let midi_in = MidiInput::new("bendr-midi-listener")
            .map_err(|e| format!("Failed to create MIDI input: {e}"))?;

        let ports = midi_in.ports();
        let target_port = ports
            .into_iter()
            .find(|p| midi_in.port_name(p).map(|n| n == port_name).unwrap_or(false))
            .ok_or_else(|| format!("MIDI port not found: {port_name}"))?;

        let app_handle = self.app_handle.clone();
        let connection = midi_in
            .connect(
                &target_port,
                port_name,
                move |_timestamp, message, _| {
                    if message.is_empty() {
                        return;
                    }
                    let status = message[0];
                    let channel = status & 0x0F;
                    let data1 = message.get(1).copied().unwrap_or(0);
                    let data2 = message.get(2).copied().unwrap_or(0);

                    let payload = MidiMessagePayload {
                        channel,
                        status,
                        data1,
                        data2,
                    };

                    if let Err(e) = app_handle.emit("midi:message", payload) {
                        log::error!("[BENDR MIDI] Error emitting midi:message event: {e}");
                    }
                },
                (),
            )
            .map_err(|e| format!("Failed to connect to MIDI port {port_name}: {e}"))?;

        self.connection = Some(connection);
        log::info!("[BENDR MIDI] Successfully connected to {port_name}");
        Ok(())
    }

    /// Disconnects from the currently active MIDI input port.
    pub fn disconnect(&mut self) -> Result<(), String> {
        if let Some(conn) = self.connection.take() {
            log::info!("[BENDR MIDI] Closing active MIDI connection");
            drop(conn);
        }
        Ok(())
    }
}
