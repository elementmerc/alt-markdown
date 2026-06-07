import { defineConfig } from "vitest/config";

// The runtime is browser code, so tests run against a DOM by default.
export default defineConfig({
  test: {
    environment: "jsdom",
  },
});
