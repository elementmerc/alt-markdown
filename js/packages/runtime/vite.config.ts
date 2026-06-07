import { defineConfig } from "vite";

// Library build for the runtime bundle. Phase 4 wires the WASM core load and the
// component upgrade pipeline; this config keeps the build real from Phase 0.
export default defineConfig({
  build: {
    lib: {
      entry: "src/index.ts",
      name: "AltMarkdown",
      fileName: "index",
      formats: ["es"],
    },
  },
});
