//! OSC (Open Sound Control) module for BENDR Desktop.
//!
//! Provides a background UDP server that receives OSC messages
//! from external software (TouchDesigner, Max, TouchOSC, etc.)
//! and bridges them to the frontend via Tauri events.

use rosc::{OscPacket, OscType};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A serialized version of an OSC argument that can be sent over IPC.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum OscArg {
    Int(i32),
    Float(f32),
    String(String),
    Bool(bool),
    Nil,
}

impl From<&OscType> for OscArg {
    fn from(arg: &OscType) -> Self {
        match arg {
            OscType::Int(v) => OscArg::Int(*v),
            OscType::Float(v) => OscArg::Float(*v),
            OscType::String(v) => OscArg::String(v.clone()),
            OscType::Bool(v) => OscArg::Bool(*v),
            OscType::Nil => OscArg::Nil,
            _ => OscArg::String(format!("{:?}", arg)), // Fallback for complex types
        }
    }
}

/// Payload emitted with the `osc:message` event.
#[derive(Debug, Clone, Serialize)]
pub struct OscMessagePayload {
    pub path: String,
    pub args: Vec<OscArg>,
}

#[allow(dead_code)]
pub struct OscService {
    app_handle: AppHandle,
    is_running: Arc<AtomicBool>,
}

/// Starts the background UDP server for receiving OSC messages.
async fn run_osc_server(app_handle: AppHandle, is_running: Arc<AtomicBool>, port: u16) {
    let addr = format!("0.0.0.0:{}", port);
    log::info!("[BENDR OSC] Binding OSC server to {}", addr);

    let socket = match UdpSocket::bind(&addr).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("[BENDR OSC] Failed to bind UDP socket: {}", e);
            return;
        }
    };

    let mut buf = [0u8; 8192]; // Max UDP packet size typically used for OSC is well under 8K

    while is_running.load(Ordering::Relaxed) {
        match socket.recv_from(&mut buf).await {
            Ok((size, _peer)) => {
                let packet_res = rosc::decoder::decode_udp(&buf[..size]);
                match packet_res {
                    Ok((_, packet)) => handle_packet(&app_handle, packet),
                    Err(e) => log::warn!("[BENDR OSC] Failed to decode packet: {}", e),
                }
            }
            Err(e) => {
                log::error!("[BENDR OSC] UDP receive error: {}", e);
            }
        }
    }
}

/// Recursively handles OSC packets (Messages or Bundles) and emits them.
fn handle_packet(app_handle: &AppHandle, packet: OscPacket) {
    match packet {
        OscPacket::Message(msg) => {
            let payload = OscMessagePayload {
                path: msg.addr,
                args: msg.args.iter().map(OscArg::from).collect(),
            };
            if let Err(e) = app_handle.emit("osc:message", payload) {
                log::error!("[BENDR OSC] Failed to emit osc:message: {}", e);
            }
        }
        OscPacket::Bundle(bundle) => {
            for packet in bundle.content {
                handle_packet(app_handle, packet);
            }
        }
    }
}

/// Initializes the OSC service, binding to UDP port 8000 by default.
pub fn init(app_handle: AppHandle) -> Result<OscService, String> {
    log::info!("[BENDR OSC] Initializing OSC subsystem...");

    let is_running = Arc::new(AtomicBool::new(true));
    let service = OscService {
        app_handle: app_handle.clone(),
        is_running: is_running.clone(),
    };

    let port = 8000; // Standard default OSC port

    tauri::async_runtime::spawn(async move {
        run_osc_server(app_handle, is_running, port).await;
    });

    Ok(service)
}

impl Drop for OscService {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
    }
}
