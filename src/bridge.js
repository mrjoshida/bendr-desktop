/**
 * BENDR Desktop Bridge
 * 
 * Thin shim that connects bendr's existing extension points
 * to the Tauri Rust backend. Injected alongside index.html,
 * never modifying the core source.
 */

// Only activate when running inside Tauri
if (window.__TAURI_INTERNALS__) {
  const { invoke } = window.__TAURI_INTERNALS__;
  
  console.log('[BENDR Desktop] Bridge initializing...');

  // ── MIDI Bridge ────────────────────────────────────────
  // On macOS WKWebView, Web MIDI API is unavailable.
  // We route ALL MIDI through the Rust backend via midir,
  // giving us unified USB + Network MIDI.
  
  const BendrDesktop = {
    midi: {
      /**
       * Scan for available MIDI input ports (USB + Network)
       * @returns {Promise<string[]>} Array of port names
       */
      async scanPorts() {
        try {
          return await invoke('midi_scan_ports');
        } catch (e) {
          console.error('[BENDR Desktop] MIDI scan failed:', e);
          return [];
        }
      },

      /**
       * Connect to a MIDI port by name
       * @param {string} portName 
       */
      async connect(portName) {
        try {
          await invoke('midi_connect', { portName });
          console.log(`[BENDR Desktop] Connected to MIDI port: ${portName}`);
        } catch (e) {
          console.error('[BENDR Desktop] MIDI connect failed:', e);
        }
      },

      /**
       * Disconnect from current MIDI port
       */
      async disconnect() {
        try {
          await invoke('midi_disconnect');
          console.log('[BENDR Desktop] MIDI disconnected');
        } catch (e) {
          console.error('[BENDR Desktop] MIDI disconnect failed:', e);
        }
      }
    },

    /** Desktop display management */
    displays: {
      async getAll() {
        try {
          return await invoke('get_displays');
        } catch (e) {
          console.error('[BENDR Desktop] Display query failed:', e);
          return [];
        }
      }
    }
  };

  // Expose to global scope for bendr core to detect
  window.BendrDesktop = BendrDesktop;

  // ── MIDI Event Listener ────────────────────────────────
  // Listen for MIDI messages from the Rust backend and
  // inject them into bendr's existing MIDI handler.
  //
  // The Rust backend emits 'midi:message' events with:
  //   { status: number, data1: number, data2: number }
  //
  // bendr's handler is `window.onMidi(e)` (p3_mod_ui.js L4143)
  // and expects: { data: Uint8Array, timeStamp: number }
  // timeStamp is critical for MIDI clock (0xF8) BPM calculation.
  
  // Use Tauri's event listener API
  const { listen } = window.__TAURI_INTERNALS__;
  if (listen) {
    listen('midi:message', (event) => {
      const { status, data1, data2 } = event.payload;
      
      // Create a synthetic MIDIMessageEvent-like object
      // matching what bendr's onMidi() expects:
      //   e.data[0] = status byte
      //   e.data[1] = data1 (note/CC number)
      //   e.data[2] = data2 (velocity/CC value)
      //   e.timeStamp = DOMHighResTimeStamp (required for clock EMA)
      const syntheticEvent = {
        data: new Uint8Array([status, data1, data2]),
        timeStamp: performance.now()
      };
      
      // Call bendr's global MIDI handler directly.
      // onMidi() is a top-level function in p3_mod_ui.js,
      // bound to window scope via the concatenated script block.
      if (typeof window.onMidi === 'function') {
        window.onMidi(syntheticEvent);
      }
    });
    
    console.log('[BENDR Desktop] MIDI event bridge active');
  }

  // ── Auto-scan on startup ───────────────────────────────
  setTimeout(async () => {
    const ports = await BendrDesktop.midi.scanPorts();
    if (ports.length > 0) {
      console.log(`[BENDR Desktop] Found ${ports.length} MIDI port(s):`, ports);
    }
  }, 1000);

  console.log('[BENDR Desktop] Bridge ready');
}
