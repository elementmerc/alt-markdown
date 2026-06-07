import { expect, test } from "@playwright/test";

// End-to-end proof of the v0.1 thesis in a real browser: the zero-build demo
// loads the wasm core + the runtime as plain ES modules, renders the sample
// document, and the components upgrade their static fallbacks in place.
//
// Prerequisites (run before this spec):
//   npm run build   -> packages/*/dist
//   npm run wasm    -> js/wasm/web bindings
// The demo is served by the configured webServer (python3 http.server).

test.describe("kitchen-sink demo", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/demo/");
    // The runtime upgrades elements in connectedCallback once the module runs.
    await page.waitForSelector("alt-callout[data-altmd-upgraded]");
  });

  test("components upgrade in place", async ({ page }) => {
    // Callout upgraded.
    await expect(page.locator("alt-callout")).toHaveAttribute(
      "data-altmd-upgraded",
      "",
    );
  });

  test("graceful degradation: the static fallback is present", async ({
    page,
  }) => {
    // The callout's static fallback is a semantic aside, present in the DOM
    // regardless of enhancement, so the document reads without the runtime.
    await expect(page.locator("alt-callout aside[role='note']")).toBeVisible();
  });

  test("tabs are interactive", async ({ page }) => {
    const tablist = page.locator("alt-tabs .alt-tablist");
    await expect(tablist).toBeVisible();

    const buttons = tablist.locator("button");
    await expect(buttons).toHaveCount(2);

    const tabs = page.locator("alt-tabs alt-tab");
    // First tab visible, second hidden after initial select(0).
    await expect(tabs.nth(0)).toBeVisible();
    await expect(tabs.nth(1)).toBeHidden();

    // Clicking the second button reveals the second tab.
    await buttons.nth(1).click();
    await expect(tabs.nth(0)).toBeHidden();
    await expect(tabs.nth(1)).toBeVisible();
  });

  test("diagram renders inside a sandbox without same-origin", async ({
    page,
  }) => {
    const iframe = page.locator("alt-diagram iframe");
    await expect(iframe).toHaveCount(1);

    // Live origin-isolation proof: the sandbox token list must grant scripts
    // but never allow-same-origin, so the frame runs in an opaque origin with
    // no access to the host page.
    const sandbox = await iframe.getAttribute("sandbox");
    expect(sandbox).toBeTruthy();
    expect(sandbox).toContain("allow-scripts");
    expect(sandbox).not.toContain("allow-same-origin");
  });
});
