import { expect, test } from "@playwright/test";

// Loads each gallery article in a real browser and proves the headline claim:
// the documents render fully and nothing in them executes, even though the
// cybersecurity article is deliberately full of attack payloads.

const ARTICLES = ["cybersecurity", "cs-lewis", "paradox-of-genius"];

for (const doc of ARTICLES) {
  test(`article "${doc}" renders and runs no script`, async ({ page }) => {
    let dialogFired = false;
    page.on("dialog", (d) => {
      dialogFired = true;
      void d.dismiss();
    });
    await page.goto(`/demo/article.html?doc=${doc}`);
    await page.waitForSelector("#doc h1");

    // No payload became a live <script> in the rendered content.
    expect(await page.locator("#doc script").count()).toBe(0);
    // No alert/confirm/prompt fired from any payload.
    await page.waitForTimeout(400);
    expect(dialogFired).toBe(false);
  });
}

test("the cybersecurity article renders its chart and a contained diagram", async ({
  page,
}) => {
  await page.goto("/demo/article.html?doc=cybersecurity");
  await page.waitForSelector("alt-chart .alt-chart-canvas canvas");

  const iframe = page.locator("alt-diagram iframe");
  await expect(iframe).toHaveCount(1);
  const sandbox = (await iframe.getAttribute("sandbox")) ?? "";
  expect(sandbox).not.toContain("allow-scripts");
  expect(sandbox).not.toContain("allow-same-origin");

  // The hostile callout attribute became an inert data attribute.
  await expect(page.locator("alt-callout[data-onclick]")).toHaveCount(1);
});
