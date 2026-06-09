import { expect, test } from "@playwright/test";

// Guards the playground against the chart-overflow regression: a uPlot canvas
// sized once at mount used to stay wider than its column (a fallback width, or a
// stale measurement taken before a vertical scrollbar narrowed the pane), which
// left a permanent horizontal scrollbar on the preview. The runtime now reflows
// the chart to its container; this proves the preview never overflows sideways,
// at load and after a resize.

// A narrow, short viewport: two columns (above the 720px stack breakpoint) so the
// preview pane is well under the chart's fallback width, and short enough that the
// document needs a vertical scrollbar, reproducing the width-shrink that bit us.
const NARROW = { width: 760, height: 520 };

async function horizontalOverflow(page: import("@playwright/test").Page): Promise<number> {
  return page.locator("#out").evaluate((el) => el.scrollWidth - el.clientWidth);
}

test("the playground preview never overflows horizontally", async ({ page }) => {
  await page.setViewportSize(NARROW);
  await page.goto("/demo/playground.html");

  // Wait for the real render path: wasm init, then the starter's chart upgrades
  // into a uPlot canvas inside the preview pane.
  await page.waitForSelector("#out canvas");
  // Let the reflow (a ResizeObserver coalesced into a frame) settle.
  await page.waitForTimeout(250);

  expect(await horizontalOverflow(page)).toBeLessThanOrEqual(1);

  // Shrinking the window must not reintroduce the overflow.
  await page.setViewportSize({ width: 730, height: 520 });
  await page.waitForTimeout(250);
  expect(await horizontalOverflow(page)).toBeLessThanOrEqual(1);
});
