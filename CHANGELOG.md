# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-10-02

### Added
- **Native WebRTC Display Selector UI**: Hijacked the core `#btnPop` button to open a custom dropdown menu that lists all physical displays natively. The borderless WebRTC output window can now be routed directly to any monitor.
- **RTP-MIDI (AppleMIDI) Auto-Discovery**: Integrated `rtpmidi` crate. `BENDR Desktop` now broadcasts itself via mDNS on UDP 5004 and auto-accepts incoming connections from iPads, TouchOSC, and macOS network sessions, injecting them seamlessly into the Tauri IPC frontend bridge.
- **OSC UDP Background Server**: Added native OSC support via `rosc` on UDP 8000. Incoming OSC messages (like `/bendr/cc/14 1.0` or `/bendr/note/60 1.0`) are bridged to the JS frontend and dynamically converted into synthetic MIDI events, allowing standard OSC apps to utilize `bendr`'s native MIDI Learn tool out of the box.
- **NDI Output Broadcasting**: Added zero-dependency dynamic NDI 6 runtime loading via `libloading` and `ndi-sdk` FFI structs. Passes raw RGBA frames natively over the local network to Resolume/OBS alongside Syphon without requiring static proprietary C++ linkage (gracefully disables if NDI tools are not installed).

## [Unreleased]

### Added
- Syphon (macOS) frame publishing via syphon-core + objc2-metal with full Metal pipeline (device, command queue, texture upload, RGBA to BGRA conversion)
- Spout2 (Windows) stub for future DirectX/OpenGL texture sharing
- Architecture documentation (docs/architecture.md)
- Gemini AI code review workflow (.github/workflows/gemini-review.yaml)
- Async-safe publisher commands using tokio::sync::Mutex

### Fixed
- IPC serialization: replaced Array.from() with raw Uint8Array for zero-copy frame transfer
- Thread safety: publisher uses tokio::sync::Mutex instead of std::sync::Mutex to avoid blocking the Tauri IPC thread during Metal FFI calls

## [0.1.0] - 2026-09-28

### Added
- Initial Tauri v2 project scaffold
- bendr core linked as git submodule (bendr/)
- MIDI bridge: Rust midir backend to window.onMidi() with synthetic MIDIMessageEvent objects including timeStamp
- Tauri commands: midi_scan_ports, midi_connect, midi_disconnect
- Multi-display output via local WebRTC loopback (display.rs, output.html)
- Display enumeration, borderless fullscreen output window on target display
- Render-drive loop: output window sends output:request-tick to prevent background throttling
- Build pipeline: build.js injects bridge.js + desktop-ui.js into core index.html
- Phase 2/3 stubs for OSC (osc.rs), NDI (ndi.rs)
