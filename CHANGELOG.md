# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
