import { defineConfig } from "vitest/config";

// The runtime is browser code, so tests run against a DOM by default.
// The Playwright e2e specs live under e2e/ and run via `npm run test:e2e`,
// not vitest, so they are excluded from the unit run here.
export default defineConfig({
  test: {
    environment: "jsdom",
    exclude: ["**/node_modules/**", "**/dist/**", "e2e/**"],
  },
});
