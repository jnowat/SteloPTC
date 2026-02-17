import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  compilerOptions: {
    // Redundant with vite.config.ts but acts as a safety net:
    // always generate client-side code (no SSR).
    generate: "client",
  },
};
