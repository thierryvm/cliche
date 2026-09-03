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

    rollupOptions: {
      // TWO entry points, and the separation is the point.
      //
      // `veil.html` is the full-screen overlay that shows the frozen screen. It
      // must not load React or the design system: that bundle would be parsed
      // and executed between the shortcut and the image, inside the very budget
      // lot 1 measures, and it would stay there for ever afterwards. Keeping it
      // out is worth these six lines of configuration.
      //
      // Paths are relative to Vite's `root`, which is the project directory both
      // `pnpm dev` and `pnpm build` run from. No `__dirname` - this file is an
      // ES module and resolving one would mean adding `@types/node` for two
      // strings.
      input: {
        main: 'index.html',
        veil: 'veil.html',
      },
    },
  },
});
