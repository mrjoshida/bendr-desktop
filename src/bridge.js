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
      
      const syntheticEvent = {
        data: new Uint8Array([status, data1, data2]),
        timeStamp: performance.now()
      };
      
      if (typeof window.onMidi === 'function') {
        window.onMidi(syntheticEvent);
      }
    });

    // ── OSC Event Listener ─────────────────────────────────
    // Bridged from the Rust UDP server.
    // Translates specific OSC paths directly into MIDI events,
    // allowing users to use bendr's native MIDI Learn with OSC.
    listen('osc:message', (event) => {
      const { path, args } = event.payload;
      if (!args || args.length === 0) return;

      // Extract numeric value (Float or Int)
      const rawVal = args[0];
      let val = 0;
      if (typeof rawVal === 'number') {
        val = rawVal;
      } else if (rawVal && typeof rawVal === 'object' && rawVal.Float !== undefined) {
        val = rawVal.Float;
      } else if (rawVal && typeof rawVal === 'object' && rawVal.Int !== undefined) {
        val = rawVal.Int;
      }

      // 1. /bendr/cc/<number> (val: 0.0 - 1.0) -> mapped to CC (0xB0)
      const ccMatch = path.match(/^\/bendr\/cc\/(\d+)$/);
      if (ccMatch) {
        const ccNum = parseInt(ccMatch[1], 10) & 0x7F;
        const ccVal = Math.max(0, Math.min(127, Math.round(val * 127)));
        if (typeof window.onMidi === 'function') {
          window.onMidi({ data: new Uint8Array([0xB0, ccNum, ccVal]), timeStamp: performance.now() });
        }
        return;
      }

      // 2. /bendr/note/<number> (val: 0.0 - 1.0) -> mapped to Note On/Off
      const noteMatch = path.match(/^\/bendr\/note\/(\d+)$/);
      if (noteMatch) {
        const noteNum = parseInt(noteMatch[1], 10) & 0x7F;
        const vel = Math.max(0, Math.min(127, Math.round(val * 127)));
        const status = vel > 0 ? 0x90 : 0x80;
        if (typeof window.onMidi === 'function') {
          window.onMidi({ data: new Uint8Array([status, noteNum, vel]), timeStamp: performance.now() });
        }
        return;
      }
    });
    
    console.log('[BENDR Desktop] MIDI & OSC event bridges active');
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

// ── Syphon & Spout Publisher Bridge ──────────────────────
// Transmits the WebGL frame to Rust for native sharing.

BendrDesktop.publisher = {
  _active: false,
  _canvas: null,
  _ctx: null,

  async toggle(active) {
    this._active = active;
    try {
      await invoke('toggle_publisher', { active });
      if (active) this._startCapture();
    } catch (e) {
      console.error('[BENDR Desktop] Failed to toggle publisher:', e);
    }
  },

  _startCapture() {
    if (!this._active) return;
    
    // Attempt to grab pixels.
    // In bendr, the main output is rendered to the global `canvas`.
    // However, gl.preserveDrawingBuffer is false. 
    // We can hook into window.__tick or grab after renderFrame.
    // To do this reliably, we can create a secondary canvas that
    // captures the stream and copies frames.
    
    if (!this._canvas) {
      this._canvas = document.createElement('canvas');
      this._ctx = this._canvas.getContext('2d', { willReadFrequently: true });
      this._video = document.createElement('video');
      this._video.autoplay = true;
      this._video.muted = true;
      this._video.srcObject = window.__getOutputStream();
    }

    const captureLoop = async () => {
      if (!this._active) return;
      
      if (this._video.videoWidth > 0 && this._video.videoHeight > 0) {
        if (this._canvas.width !== this._video.videoWidth) {
          this._canvas.width = this._video.videoWidth;
          this._canvas.height = this._video.videoHeight;
        }
        this._ctx.drawImage(this._video, 0, 0);
        const imgData = this._ctx.getImageData(0, 0, this._canvas.width, this._canvas.height);
        
        try {
          // Send pixel buffer over IPC
          await invoke('publish_frame', {
            width: this._canvas.width,
            height: this._canvas.height,
            pixels: new Uint8Array(imgData.data.buffer) // Fast zero-copy IPC array transfer
          });
        } catch (e) {
          console.error('[BENDR Publisher IPC Error]', e);
        }
      }
      
      // Throttle to roughly 30-60fps
      requestAnimationFrame(captureLoop);
    };
    
    requestAnimationFrame(captureLoop);
  }
};
