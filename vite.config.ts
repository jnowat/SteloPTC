import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { VitePWA } from "vite-plugin-pwa";

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
      // Suppress Svelte's built-in a11y lint warnings.  This is a
      // closed-network desktop app; the a11y rules add noise without value.
      onwarn: (warning, defaultHandler) => {
        if (warning.code.startsWith("a11y")) return;
        defaultHandler(warning);
      },
    }),
    // WP-62: PWA support for the web-export build target only. This plugin
    // always emits a manifest + service worker into `dist/`, since Tauri and
    // the PWA build share the same `npm run build` output — but the actual
    // `navigator.serviceWorker.register(...)` call (in `src/main.ts`) is
    // gated behind a runtime Tauri-detection check, so the service worker
    // never activates inside the desktop webview and can never intercept
    // Tauri's `ipc://` calls. `injectRegister: false` disables the plugin's
    // own auto-injected registration script so that gate is the only place
    // registration happens.
    VitePWA({
      registerType: "autoUpdate",
      injectRegister: false,
      manifest: {
        name: "Stelo Lab Suite",
        short_name: "Stelo",
        description: "Plant Tissue Culture / Cell Culture / Mycology specimen tracking",
        display: "standalone",
        orientation: "portrait",
        theme_color: "#0f172a",
        background_color: "#0f172a",
        icons: [
          { src: "pwa-128x128.png", sizes: "128x128", type: "image/png" },
          { src: "pwa-256x256.png", sizes: "256x256", type: "image/png", purpose: "any maskable" },
        ],
      },
      workbox: {
        // Precache the built static assets; use a network-first strategy
        // for navigation so the PWA always prefers fresh content when
        // online and falls back to the cached shell when offline.
        globPatterns: ["**/*.{js,css,html,svg,png,ico}"],
        navigateFallback: "index.html",
        runtimeCaching: [
          {
            urlPattern: ({ request }) => request.mode === "navigate",
            handler: "NetworkFirst",
            options: { cacheName: "stelo-pages" },
          },
        ],
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
