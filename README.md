# BENDR Desktop

Desktop wrapper for [BENDR](https://github.com/mrjoshida/bendr) — circuit-bent video processor with native WiFi MIDI, OSC, Syphon, and NDI capabilities.

Powered by [Tauri v2](https://v2.tauri.app/).

---

## Overview

**BENDR Desktop** embeds the core BENDR single-file web application within a high-performance native desktop shell. It unlocks capabilities that are restricted or unavailable in standard browser contexts (such as WKWebView Web MIDI limitations on macOS), providing:

- Unified local USB and network (Apple MIDI / RTP-MIDI) input.
- Native performance with minimal memory and binary footprint.
- Future routing for OSC control, Syphon/Spout GPU video textures, and NDI video streaming.

## Architecture

The project consists of three coordinated layers:

1. **BENDR Core Submodule (`bendr/`)**: The upstream circuit-bending web engine maintained as a git submodule.
2. **Desktop Bridge (`src/bridge.js` & `src/desktop-ui.js`)**: A thin JavaScript shim injected at build time into `dist/index.html`. It exposes native APIs under `window.BendrDesktop` and translates native IPC events (such as `midi:message`) into standard MIDI message events for BENDR's input handlers.
3. **Native Rust Backend (`src-tauri/`)**: Tauri v2 application managing OS-level hardware interfaces (CoreMIDI / ALSA / WinMM via `midir`), network discovery (`mdns-sd`), and window management.

```
┌────────────────────────────────────────────────────────┐
│                   BENDR Webview UI                     │
│  (Canvas, WebGL Shaders, Audio Modulators, Patch Bay)  │
└───────────────────────────▲────────────────────────────┘
                            │ synthetic events / JS API
┌───────────────────────────▼────────────────────────────┐
│          Desktop Bridge Shim (src/bridge.js)           │
└───────────────────────────▲────────────────────────────┘
                            │ Tauri IPC (invoke / emit)
┌───────────────────────────▼────────────────────────────┐
│             Tauri v2 Rust Backend (src-tauri)          │
│  ├── MIDI Service (midir + Apple MIDI / RTP-MIDI)      │
│  ├── Display Management                                │
│  ├── [Phase 2] OSC Service (rosc)                      │
│  ├── [Phase 2] Syphon / Spout Output                   │
│  └── [Phase 3] NDI Video I/O                           │
└────────────────────────────────────────────────────────┘
```

## Prerequisites

Before building or running BENDR Desktop, ensure you have:

- **Node.js**: v18 or later
- **Rust & Cargo**: Latest stable toolchain (`rustup update`)
- **Tauri CLI v2**: `cargo install tauri-cli --version "^2.0.0"` or via `npm run tauri`
- **Platform Dependencies**:
  - **macOS**: Xcode Command Line Tools (`xcode-select --install`)
  - **Linux**: `libwebkit2gtk-4.1-dev`, `build-essential`, `curl`, `wget`, `file`, `libssl-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libasound2-dev`
  - **Windows**: Microsoft Visual Studio C++ Build Tools and WebView2 runtime

## Getting Started

### 1. Clone the Repository

Clone recursively to fetch the `bendr` core engine submodule:

```bash
git clone --recurse-submodules https://github.com/mrjoshida/bendr-desktop.git
cd bendr-desktop
```

If you already cloned without `--recurse-submodules`:

```bash
git submodule update --init --recursive
```

### 2. Install Node Dependencies

```bash
npm install
```

### 3. Run Development Mode

Build the frontend bundle and launch the Tauri dev window:

```bash
npm run dev
```

This automatically runs `build.js` before launching Tauri, injecting `src/bridge.js` and `src/desktop-ui.js` into `bendr/index.html` to generate `dist/index.html`.

### 4. Build Production Bundle

```bash
npm run build
```

The compiled native executable and installer bundles will be generated under `src-tauri/target/release/bundle/`.

---

## Updating BENDR Core

To pull the latest updates from the upstream BENDR repository:

```bash
cd bendr
git checkout main
git pull
cd ..
git commit -am "chore: update bendr core submodule"
```

Then rebuild the frontend:

```bash
npm run build:frontend
```

---

## Implementation Roadmap

- [x] **Phase 1a: Core Desktop Shell** — Tauri v2 architecture, `dist` compilation pipeline, and bridge injection.
- [x] **Phase 1b: Hardware & WiFi MIDI** — `midir` integration, port scanning, event forwarding, and mDNS discovery stubs.
- [ ] **Phase 2a: OSC Control** — Bidirectional Open Sound Control via `rosc` for parameter mapping and hardware surface control.
- [ ] **Phase 2b: Syphon / Spout Output** — Zero-latency GPU frame sharing with Resolume, VDMX, TouchDesigner, and OBS.
- [ ] **Phase 3: NDI Video I/O** — Networked video streaming and camera capture over gigabit Ethernet.

---

## Related Repositories

- [BENDR Core Engine](https://github.com/mrjoshida/bendr) — WebGL circuit-bent video synthesizer.
- [BENDR Final Cut Pro Plugin](https://github.com/mrjoshida/bendr-fcp) — FxPlug4 plugin integration.

---

## License

MIT © Josh Caldwell
