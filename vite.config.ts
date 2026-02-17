import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [
    svelte({
      compilerOptions: {
        // Force client-side code generation — this is a Tauri desktop app,
        // there is no server. Without this, Svelte 5 may produce an SSR
        // bundle where mount() / onMount() are unavailable.
        generate: "client",
      },
    }),
  ],

  // Ensure the bundler resolves package "exports" conditions for the
  // browser.  Svelte 5's package.json uses a "browser" condition to
  // expose client-side APIs (mount, onMount, etc.).  Without this, the
  // production build can fall through to the default/server entry where
  // mount() does not exist.
  resolve: {
    conditions: ["browser"],
  },

  build: {
    // WebView2 (Windows), WebKitGTK (Linux), WKWebView (macOS) all
    // support modern JS.  Targeting a recent baseline avoids unnecessary
    // transpilation and keeps the bundle small.
    target: ["es2021", "chrome100", "safari15"],
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },

  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
