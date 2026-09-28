/**
 * BENDR Desktop UI Extensions
 * 
 * Desktop-specific UI panels and controls.
 * Only activates when running inside Tauri.
 * 
 * TODO Phase 1:
 * - MIDI device connection panel
 * - Display selection for fullscreen output
 * 
 * TODO Phase 2:
 * - OSC routing configuration
 * - Syphon/Spout status indicators
 */

if (window.__TAURI_INTERNALS__) {
  console.log('[BENDR Desktop] Desktop UI extensions loaded');
  // UI extensions will be added here in future phases
}
