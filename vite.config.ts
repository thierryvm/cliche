import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Tauri drives this dev server through `beforeDevCommand` and then points the
// webview at `devUrl`. Both sides must agree on the port, hence `strictPort`:
// failing loudly is better than Tauri loading a blank page from a port that
// silently moved.
export default defineConfig({
  plugins: [react()],

  // Tauri's own output (Rust compilation progress, our stdout logs) must stay
  // readable in the terminal.
  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
  },

  build: {
    // WebView2 on Windows 11 is evergreen Chromium; no legacy transpiling needed.
    target: 'chrome110',
    sourcemap: true,
  },
});
