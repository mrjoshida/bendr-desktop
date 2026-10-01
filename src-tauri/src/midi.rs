//! MIDI input and connection service for BENDR Desktop.
//!
//! Provides unified access to local USB MIDI controllers and network
//! MIDI endpoints (Apple MIDI / RTP-MIDI), bridging incoming events
//! into Tauri webview event dispatches.

use midir::{MidiInput, MidiInputConnection};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use std::error::Error;
use rtpmidi::sessions::{
    events::event_handling::MidiMessageEvent,
    invite_responder::InviteResponder,
    rtp_midi_session::RtpMidiSession,
};
use midi_types::MidiMessage;

/// Payload emitted with the `midi:message` event.
#[derive(Debug, Clone, Serialize)]
pub struct MidiMessagePayload {
    /// Raw status byte (e.g. 0x90 for Note On, 0xB0 for CC).
    pub status: u8,
    /// First data byte (note number, controller index, etc.).
    pub data1: u8,
    /// Second data byte (velocity, value, etc.).
    pub data2: u8,
}

/// Service managing local USB MIDI inputs and connection state.
pub struct MidiService {
    /// Tauri application handle for emitting events.
    app_handle: AppHandle,
    /// Active MIDI input connection, if any.
    connection: Option<MidiInputConnection<()>>,
}

/// Helper to decompose `midi_types::MidiMessage` from RTP-MIDI into raw MIDI bytes (status, data1, data2).
fn extract_raw_midi(msg: &MidiMessage) -> (u8, u8, u8) {
    match *msg {
        MidiMessage::NoteOn(ch, note, vel) => (0x90 | u8::from(ch), u8::from(note), u8::from(vel)),
        MidiMessage::NoteOff(ch, note, vel) => (0x80 | u8::from(ch), u8::from(note), u8::from(vel)),
        MidiMessage::ControlChange(ch, ctrl, val) => (0xB0 | u8::from(ch), u8::from(ctrl), u8::from(val)),
        MidiMessage::PitchBendChange(ch, val) => {
            let (msb, lsb): (u8, u8) = val.into();
            (0xE0 | u8::from(ch), msb, lsb)
        }
        MidiMessage::ProgramChange(ch, prog) => (0xC0 | u8::from(ch), u8::from(prog), 0),
        MidiMessage::ChannelPressure(ch, val) => (0xD0 | u8::from(ch), u8::from(val), 0),
        MidiMessage::KeyPressure(ch, note, val) => (0xA0 | u8::from(ch), u8::from(note), u8::from(val)),
        MidiMessage::SongPositionPointer(val) => {
            let (msb, lsb): (u8, u8) = val.into();
            (0xF2, msb, lsb)
        }
        MidiMessage::SongSelect(s) => (0xF3, u8::from(s), 0),
        MidiMessage::QuarterFrame(q) => (0xF1, u8::from(q), 0),
        MidiMessage::TimingClock => (0xF8, 0, 0),
        MidiMessage::Start => (0xFA, 0, 0),
        MidiMessage::Continue => (0xFB, 0, 0),
        MidiMessage::Stop => (0xFC, 0, 0),
        MidiMessage::ActiveSensing => (0xFE, 0, 0),
        MidiMessage::Reset => (0xFF, 0, 0),
        MidiMessage::TuneRequest => (0xF6, 0, 0),
    }
}

/// Spawns the background RTP-MIDI (mDNS) server session.
/// Automatically accepts connections from iPads/macs and bridges data to the UI.
async fn start_rtp_midi_server(app_handle: AppHandle) -> Result<(), Box<dyn Error>> {
    let control_port = 5004; // Standard AppleMIDI port. Will use 5005 for stream.
    let session_name = "BENDR Desktop";
    let ssrc = 0x42454E44; // "BEND" in hex

    log::info!("[BENDR RTP-MIDI] Starting mDNS broadcast as '{session_name}' on port {control_port}");

    // The session advertises via mDNS automatically because we enabled the `mdns` feature.
    let session = RtpMidiSession::start(
        control_port,
        session_name,
        ssrc,
        InviteResponder::Accept, // Auto-accept all iOS/macOS incoming connections
    )
    .await?;

    // Add listener for incoming MIDI messages
    session
        .add_listener(MidiMessageEvent, move |(msg, _timestamp)| {
            let (status, data1, data2) = extract_raw_midi(&msg);
            
            let payload = MidiMessagePayload { status, data1, data2 };

            if let Err(e) = app_handle.emit("midi:message", payload) {
                log::error!("[BENDR RTP-MIDI] Error emitting midi:message event: {e}");
            }
        })
        .await;

    // We leave the session running indefinitely in this async task
    log::info!("[BENDR RTP-MIDI] Session active and ready for connections");
    
    // Hold the task open so the session doesn't drop
    std::future::pending::<()>().await;
    
    Ok(())
}

/// Initializes the MIDI service and inspects currently available input ports.
/// Also spawns the RTP-MIDI background server.
///
/// # Arguments
/// * `app_handle` - The Tauri application handle used to dispatch events.
pub fn init(app_handle: AppHandle) -> Result<MidiService, String> {
    log::info!("[BENDR MIDI] Initializing MIDI subsystem...");

    // 1. Setup local USB MIDI (midir)
    let midi_in = MidiInput::new("bendr-midi-init")
        .map_err(|e| format!("Failed to create MIDI input: {e}"))?;

    let ports = midi_in.ports();
    log::info!("[BENDR MIDI] Found {} available local MIDI input port(s)", ports.len());
    for (idx, port) in ports.iter().enumerate() {
        if let Ok(name) = midi_in.port_name(port) {
            log::info!("[BENDR MIDI]   [{idx}] {name}");
        }
    }

    // 2. Spawn RTP-MIDI (WiFi MIDI) background server
    let rtp_app_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = start_rtp_midi_server(rtp_app_handle).await {
            log::error!("[BENDR RTP-MIDI] Server failed to start: {e}");
        }
    });

    Ok(MidiService {
        app_handle,
        connection: None,
    })
}

impl MidiService {
    /// Scans for available local USB MIDI input ports.
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

        // We no longer inject network endpoints here because the RTP-MIDI server
        // is auto-accepting and globally routed, removing the need for the user
        // to explicitly select it from the UI dropdown.
        port_names.push("[Network] RTP-MIDI (Auto-Accepting)".to_string());

        log::info!("[BENDR MIDI] Scanned {} port(s)", port_names.len());
        Ok(port_names)
    }

    /// Connects to a named local MIDI input port and starts streaming events.
    /// Note: Does not apply to RTP-MIDI which auto-streams.
    ///
    /// # Arguments
    /// * `port_name` - The name of the port to connect to.
    pub fn connect_port(&mut self, port_name: &str) -> Result<(), String> {
        // Disconnect any existing connection first
        self.disconnect()?;

        if port_name.contains("RTP-MIDI") {
            log::info!("[BENDR MIDI] Selected Network MIDI (already active in background)");
            return Ok(());
        }

        log::info!("[BENDR MIDI] Connecting to local port: {port_name}");

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
                    let data1 = message.get(1).copied().unwrap_or(0);
                    let data2 = message.get(2).copied().unwrap_or(0);

                    let payload = MidiMessagePayload {
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

    /// Disconnects from the currently active local MIDI input port.
    pub fn disconnect(&mut self) -> Result<(), String> {
        if let Some(conn) = self.connection.take() {
            log::info!("[BENDR MIDI] Closing active local MIDI connection");
            drop(conn);
        }
        Ok(())
    }
}
