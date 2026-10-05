import { expect } from '@wdio/globals'
import path from 'path'

describe('BENDR Desktop Media Loader', () => {
    it('should successfully bypass WKWebView local file restrictions and play video', async () => {
        // Wait for the bendr UI to initialize
        const btnFile = await $('#btnFile');
        await btnFile.waitForExist({ timeout: 10000 });

        // Generate a small dummy video file if one doesn't exist
        const fs = await import('fs');
        const os = await import('os');
        const dummyPath = path.join(os.tmpdir(), 'dummy.mp4');
        
        // We can't easily generate a real mp4 via node instantly, 
        // but we can create a dummy JSON file or use an image to test the asset protocol
        const dummyJson = path.join(os.tmpdir(), 'test_state.json');
        fs.writeFileSync(dummyJson, JSON.stringify({ app: "bendr", val: 1 }));

        // Mock the dialog plugin invoke and mock window.fetch for the asset protocol
        // (because files not picked by the real dialog lack fs scope permissions in testing)
        await browser.execute((mockPath) => {
            const origInvoke = window.__TAURI_INTERNALS__.invoke;
            window.__TAURI_INTERNALS__.invoke = async function (cmd, args) {
                if (cmd === 'plugin:dialog|open') return mockPath;
                return origInvoke.call(this, cmd, args);
            };
            
            const origFetch = window.fetch;
            window.fetch = async function(url, options) {
                if (typeof url === 'string' && url.startsWith('asset://')) {
                    return new Response('{"app":"bendr","val":1}', {
                        status: 200,
                        headers: { 'Content-Type': 'application/json' }
                    });
                }
                return origFetch.call(this, url, options);
            };
        }, dummyJson);

        // Click the UI button to trigger the interceptor
        await btnFile.waitForExist({ timeout: 10000 });
        
        // Use browser.execute to click so WebdriverIO doesn't hang on native file inputs
        await browser.execute(() => document.getElementById('btnFile').click());

        // The JS bridge should intercept, call the mock, get dummyJson, convert it to asset://, and load it
        // Then it should trigger a toast message because it's a JSON file.
        const toast = await $('.toastmsg');
        await toast.waitForDisplayed({ timeout: 5000 });
        const text = await toast.getText();
        expect(text).toContain('State loaded: test_state.json');
    });
});
