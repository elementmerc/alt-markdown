import { expect, test } from "@playwright/test";

// Proves the graphics components actually light up in a real browser: the chart
// lazy-loads uPlot and draws a canvas, and the maths lazy-loads KaTeX and
// typesets the expression. Both read their data from the static fallback, which
// stays in the DOM (visually hidden) as the accessible source of truth.
test.describe("interactive graphics", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/demo/kitchensink.html");
    await page.waitForSelector("alt-chart[data-altmd-upgraded]");
  });

  test("chart renders an interactive uPlot canvas", async ({ page }) => {
    const canvas = page.locator("alt-chart .alt-chart-canvas canvas");
    await expect(canvas).toBeVisible();
    // uPlot sizes the canvas to real pixels once it has drawn.
    const box = await canvas.boundingBox();
    expect(box?.width ?? 0).toBeGreaterThan(100);

    // The fallback data table (a direct child; uPlot's own legend table lives
    // inside the canvas wrapper) is retained for accessibility but hidden.
    const fallback = page.locator("alt-chart > table");
    await expect(fallback).toHaveClass(/alt-visually-hidden/);
  });

  test("maths renders typeset KaTeX output", async ({ page }) => {
    const katex = page.locator("alt-math .alt-math-rendered .katex");
    await expect(katex).toBeVisible();
    // KaTeX builds the expression from <span> glyphs, not an image.
    await expect(page.locator("alt-math .katex")).not.toHaveCount(0);
  });

  test("diagram renders Mermaid SVG inside a locked sandbox", async ({ page }) => {
    const iframe = page.locator("alt-diagram iframe");
    await expect(iframe).toHaveCount(1);
    const sandbox = (await iframe.getAttribute("sandbox")) ?? "";
    // Script-disabled (no allow-scripts) and origin-isolated (no
    // allow-same-origin): a hostile SVG can neither execute nor reach the host.
    expect(sandbox).not.toContain("allow-scripts");
    expect(sandbox).not.toContain("allow-same-origin");
    // The Mermaid-rendered SVG is the iframe's content.
    const srcdoc = (await iframe.getAttribute("srcdoc")) ?? "";
    expect(srcdoc).toContain("<svg");
  });

  test("graphics degrade: the fallback source is present in the DOM", async ({
    page,
  }) => {
    // Even after enhancement the fallback data survives (hidden), so a no-JS
    // reader or a screen reader still gets the content.
    await expect(page.locator("alt-chart table th")).not.toHaveCount(0);
  });
});
