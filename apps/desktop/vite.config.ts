/// <reference types="node" />
/// <reference types="vitest" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: Boolean(process.env.TAURI_ENV_DEBUG),
    // Matches the three.js vendor budget in scripts/check-bundle-budget.mjs;
    // every other chunk is held to 500 kB there.
    chunkSizeWarningLimit: 700,
    rollupOptions: {
      output: {
        // Vendor code in stable chunks so app changes do not re-download them.
        manualChunks(id) {
          if (id.includes("/node_modules/@pixiv/three-vrm")) return "vrm";
          if (id.includes("/node_modules/three/")) return "three";
          if (/\/node_modules\/(react|react-dom|scheduler)\//.test(id)) return "react";
          return undefined;
        },
      },
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
  },
});

