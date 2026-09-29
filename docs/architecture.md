# BENDR Desktop Architecture

## How it Builds on the Baseline

The core of `bendr` is designed around a strict **zero-dependency philosophy**. It is a single-file HTML app that can run fully offline, ensuring maximum portability, immunity to "npm rot", and long-term stability. It runs entirely inside the browser's sandbox.

However, professional live visual environments demand capabilities that browsers explicitly block:
- **Zero-copy GPU texture sharing** (Syphon on macOS, Spout on Windows) to feed other VJ software like Resolume or TouchDesigner.
- **Network protocols** (OSC, RTP-MIDI, NDI) to communicate with external hardware and software.

`bendr-desktop` bridges this gap using **Tauri v2**. It wraps the pristine `bendr` core in a native desktop shell, injecting capabilities from the outside in.

### The Integration Pattern

We do not modify the core `index.html`. Instead, `bendr-desktop` uses a **Git Submodule** to pull in the upstream `bendr` repository. During the build process (`build.js`), we inject a thin Javascript shim (`bridge.js`) just before the closing `</body>` tag.

```mermaid
flowchart TD
    subgraph "bendr-desktop (Tauri)"
        subgraph "Frontend (Webview)"
            CORE["bendr core (index.html)"]
            BRIDGE["Tauri JS Bridge Layer"]
        end
        subgraph "Backend (Rust)"
            CMD["Tauri IPC Commands"]
            MIDI["Network MIDI Service"]
            SYPHON["Syphon / Spout Server"]
        end
    end
    
    CORE <-->|"Hooks & Events"| BRIDGE
    BRIDGE <-->|"IPC"| CMD
    CMD --> MIDI
    CMD --> SYPHON
    
    SYPHON <-->|"GPU Textures"| EXT_SYPHON["Resolume / VDMX"]
```

This pattern means:
1. **Upstream remains clean:** The core `bendr` repo never learns about Tauri, Rust, or npm.
2. **Effortless updates:** Pulling in new `bendr` features is as simple as updating the git submodule.
3. **Graceful degradation:** If `bendr-desktop`'s bridge fails, the core app continues to function exactly as it would in a normal browser.

### Bridging the Gap: Syphon & Spout

In the browser, extracting video frames requires `readPixels()` or `canvas.toDataURL()`, which incurs a massive CPU roundtrip penalty. In `bendr-desktop`, we bypass WKWebView's internal rendering isolation using **local WebRTC loopback** and optimized IPC to stream textures into a native Rust process.

For Syphon/Spout specifically:
- **macOS (Syphon):** We use `syphon-rs` to publish the WebGL output frame as an IOSurface.
- **Windows (Spout):** We use `spout2-rs` to share DirectX/OpenGL textures.

By keeping these complex FFI bindings and GPU memory interactions in Rust, we maintain the safety and performance required for live VJ setups, while the UI and video synthesis logic remain happily sandboxed in JS/WebGL.
