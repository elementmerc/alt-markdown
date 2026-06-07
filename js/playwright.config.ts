import { defineConfig, devices } from "@playwright/test";

// The demo is a zero-build page served over plain HTTP. Its inputs (the runtime
// dist and the wasm bindings) must be built first; the e2e spec documents the
// prerequisite commands. The cached Chromium is launched directly; do not run
// the installer on this platform (it reports the OS as unsupported even though
// the cached binary works).
export default defineConfig({
  testDir: "e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  reporter: "list",
  use: {
    baseURL: "http://localhost:5173",
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    command: "python3 -m http.server 5173",
    url: "http://localhost:5173/demo/",
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
  },
});
