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

  // Ensure emit and listen are available on __TAURI_INTERNALS__
  if (!window.__TAURI_INTERNALS__.emit) {
    window.__TAURI_INTERNALS__.emit = (event, payload) => invoke('plugin:event|emit', { event, payload });
  }
  if (!window.__TAURI_INTERNALS__.listen && window.__TAURI_INTERNALS__.transformCallback) {
    window.__TAURI_INTERNALS__.listen = (event, handler) => invoke('plugin:event|listen', {
      event,
      target: { kind: 'Any' },
      handler: window.__TAURI_INTERNALS__.transformCallback(handler)
    });
  }

  const emit = (event, payload) => {
    if (window.__TAURI_INTERNALS__.emit) return window.__TAURI_INTERNALS__.emit(event, payload);
    return invoke('plugin:event|emit', { event, payload });
  };

  const listen = (event, handler) => {
    if (window.__TAURI_INTERNALS__.listen) return window.__TAURI_INTERNALS__.listen(event, handler);
    if (window.__TAURI_INTERNALS__.transformCallback) {
      return invoke('plugin:event|listen', {
        event,
        target: { kind: 'Any' },
        handler: window.__TAURI_INTERNALS__.transformCallback(handler)
      });
    }
    return Promise.resolve(() => {});
  };

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

  // ── Display & Output Window Bridge ──────────────────────
  // Multi-display output: send clean program feed to a 
  // secondary display (HDMI out, projector, capture card)
  // via local WebRTC loopback with Tauri IPC signaling.

  BendrDesktop.output = {
    /** @type {RTCPeerConnection|null} */
    _pc: null,
    /** @type {boolean} */
    _open: false,

    /**
     * List connected displays
     * @returns {Promise<Array<{name:string, width:number, height:number, x:number, y:number, scale_factor:number, is_primary:boolean}>>}
     */
    async listDisplays() {
      try {
        return await invoke('list_displays');
      } catch (e) {
        console.error('[BENDR Desktop] Display enumeration failed:', e);
        return [];
      }
    },

    /**
     * Open the output window on a specific display and start streaming
     * @param {number} displayIndex - Index of the target display
     */
    async open(displayIndex) {
      try {
        // Close any existing output first
        if (this._open) await this.close();

        // Create the native output window on the target display
        await invoke('create_output_window', { displayIndex });
        this._open = true;

        // Get bendr's program output stream
        const stream = window.__getOutputStream();
        if (!stream) {
          throw new Error('No output stream available from bendr core');
        }

        // Set up WebRTC sender
        this._pc = new RTCPeerConnection({ iceServers: [] });

        // Add the video track from bendr's captureStream
        for (const track of stream.getTracks()) {
          this._pc.addTrack(track, stream);
        }

        // Handle ICE candidates — send to output window
        this._pc.onicecandidate = (event) => {
          if (event.candidate) {
            // Emit to the output window via Tauri
            const { emit: internalEmit } = window.__TAURI_INTERNALS__;
            const sendEmit = internalEmit || emit;
            sendEmit('output:ice-candidate', {
              candidate: event.candidate.candidate,
              sdpMid: event.candidate.sdpMid,
              sdpMLineIndex: event.candidate.sdpMLineIndex,
            });
          }
        };

        // Listen for answer from output window
        listen('output:webrtc-answer', async (event) => {
          try {
            if (this._pc && this._pc.signalingState !== 'stable') {
              await this._pc.setRemoteDescription(new RTCSessionDescription(event.payload));
              console.log('[BENDR Desktop] WebRTC answer received, connection established');
            }
          } catch (e) {
            console.error('[BENDR Desktop] Failed to set WebRTC answer:', e);
          }
        });

        // Listen for ICE candidates from output window
        listen('output:ice-candidate-reply', async (event) => {
          try {
            if (this._pc) {
              await this._pc.addIceCandidate(new RTCIceCandidate(event.payload));
            }
          } catch (e) {
            console.error('[BENDR Desktop] Failed to add ICE candidate:', e);
          }
        });

        // Listen for tick requests from output window to drive render loop
        listen('output:request-tick', () => {
          if (typeof window.__tick === 'function') {
            try { window.__tick(); } catch (e) {}
          }
        });

        // Create and send offer
        const offer = await this._pc.createOffer();
        await this._pc.setLocalDescription(offer);

        // Small delay to let output window initialize
        setTimeout(() => {
          const { emit: internalEmit } = window.__TAURI_INTERNALS__;
          const sendEmit = internalEmit || emit;
          sendEmit('output:webrtc-offer', {
            sdp: this._pc.localDescription.sdp,
            type: this._pc.localDescription.type,
          });
          console.log('[BENDR Desktop] WebRTC offer sent to output window');
        }, 500);

        console.log(`[BENDR Desktop] Output window opened on display ${displayIndex}`);
      } catch (e) {
        console.error('[BENDR Desktop] Failed to open output:', e);
        this._open = false;
      }
    },

    /**
     * Close the output window and clean up WebRTC
     */
    async close() {
      if (this._pc) {
        this._pc.close();
        this._pc = null;
      }
      this._open = false;
      try {
        await invoke('close_output_window');
        console.log('[BENDR Desktop] Output window closed');
      } catch (e) {
        console.error('[BENDR Desktop] Failed to close output:', e);
      }
    },

    /** Check if output is currently streaming */
    isOpen() {
      return this._open;
    }
  };

  console.log('[BENDR Desktop] Multi-display output bridge loaded');

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
