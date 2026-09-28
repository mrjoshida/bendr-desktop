#!/usr/bin/env node

/**
 * BENDR Desktop - Frontend Build Script
 *
 * Injects the Tauri desktop bridge (`src/bridge.js`) and UI extensions
 * (`src/desktop-ui.js`) into the core BENDR single-file application (`bendr/index.html`),
 * outputting the unified app to `dist/index.html`.
 */

const fs = require('fs');
const path = require('path');

const ROOT_DIR = __dirname;
const BENDR_HTML_PATH = path.join(ROOT_DIR, 'bendr', 'index.html');
const BRIDGE_JS_PATH = path.join(ROOT_DIR, 'src', 'bridge.js');
const DESKTOP_UI_JS_PATH = path.join(ROOT_DIR, 'src', 'desktop-ui.js');
const DIST_DIR = path.join(ROOT_DIR, 'dist');
const OUTPUT_HTML_PATH = path.join(DIST_DIR, 'index.html');

function build() {
  console.log('[build] Starting BENDR Desktop frontend build...');

  // 1. Verify bendr git submodule exists
  if (!fs.existsSync(BENDR_HTML_PATH)) {
    console.error('\n[build] ERROR: bendr/index.html not found!');
    console.error('[build] The bendr submodule appears to be uninitialized or missing.');
    console.error('[build] Please initialize git submodules by running:');
    console.error('        git submodule update --init --recursive\n');
    process.exit(1);
  }

  // 2. Read core bendr index.html
  console.log(`[build] Reading core app: ${path.relative(ROOT_DIR, BENDR_HTML_PATH)}`);
  const bendrHtml = fs.readFileSync(BENDR_HTML_PATH, 'utf-8');

  // 3. Read bridge.js
  let bridgeJs = '';
  if (fs.existsSync(BRIDGE_JS_PATH)) {
    console.log(`[build] Reading bridge script: ${path.relative(ROOT_DIR, BRIDGE_JS_PATH)}`);
    bridgeJs = fs.readFileSync(BRIDGE_JS_PATH, 'utf-8');
  } else {
    console.warn(`[build] Warning: ${path.relative(ROOT_DIR, BRIDGE_JS_PATH)} not found, omitting.`);
  }

  // 4. Read desktop-ui.js
  let desktopUiJs = '';
  if (fs.existsSync(DESKTOP_UI_JS_PATH)) {
    console.log(`[build] Reading desktop UI extensions: ${path.relative(ROOT_DIR, DESKTOP_UI_JS_PATH)}`);
    desktopUiJs = fs.readFileSync(DESKTOP_UI_JS_PATH, 'utf-8');
  } else {
    console.warn(`[build] Warning: ${path.relative(ROOT_DIR, DESKTOP_UI_JS_PATH)} not found, omitting.`);
  }

  // 5. Build injection payload
  const injectedCode = [
    '<!-- BENDR Desktop Injections -->',
    '<script id="bendr-desktop-bridge">',
    bridgeJs,
    '</script>',
    '<script id="bendr-desktop-ui">',
    desktopUiJs,
    '</script>'
  ].join('\n');

  // 6. Inject before closing </body> tag
  const bodyCloseIndex = bendrHtml.lastIndexOf('</body>');
  if (bodyCloseIndex === -1) {
    console.error('[build] ERROR: Closing </body> tag not found in bendr/index.html!');
    process.exit(1);
  }

  const combinedHtml =
    bendrHtml.slice(0, bodyCloseIndex) +
    injectedCode +
    '\n' +
    bendrHtml.slice(bodyCloseIndex);

  // 7. Ensure dist/ directory exists
  if (!fs.existsSync(DIST_DIR)) {
    fs.mkdirSync(DIST_DIR, { recursive: true });
    console.log(`[build] Created output directory: ${path.relative(ROOT_DIR, DIST_DIR)}/`);
  }

  // 8. Write output file
  fs.writeFileSync(OUTPUT_HTML_PATH, combinedHtml, 'utf-8');

  const inSizeKb = (Buffer.byteLength(bendrHtml, 'utf-8') / 1024).toFixed(1);
  const outSizeKb = (Buffer.byteLength(combinedHtml, 'utf-8') / 1024).toFixed(1);
  console.log(`[build] Written to ${path.relative(ROOT_DIR, OUTPUT_HTML_PATH)}`);
  console.log(`[build] Source size: ${inSizeKb} KB -> Output size: ${outSizeKb} KB`);
  console.log('[build] Build completed successfully.');
}

build();
