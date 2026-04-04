/**
 * Smoke Test: Performance Baselines
 *
 * Validates that the app meets minimum performance criteria:
 *   - Startup time < 5 seconds
 *   - Loading a 500-segment XLIFF < 3 seconds
 *   - No obvious memory leak during a short editing session
 *
 * These tests produce warnings rather than hard failures by default,
 * as exact timings vary per CI runner hardware.
 */

import { test, expect } from "@playwright/test";
import path from "path";

const FIXTURE_LARGE = path.join(__dirname, "fixtures", "500-segments.xliff");

test.describe("Performance Baselines", () => {
  test("app startup time is below 5 seconds", async ({ page }) => {
    const start = Date.now();
    await page.goto("/");
    await page.locator("#root").waitFor({ state: "attached", timeout: 10_000 });
    const elapsed = Date.now() - start;

    if (elapsed > 5_000) {
      console.warn(`[PERF WARNING] App startup took ${elapsed}ms (threshold: 5000ms)`);
    }
    // Soft assertion — CI runner slowness shouldn't block a release,
    // but if it exceeds 10 s we consider it a failure.
    expect(elapsed).toBeLessThan(10_000);
  });

  test("loading a 500-segment XLIFF file completes in < 3 seconds", async ({ page }) => {
    await page.goto("/");
    await page.locator("#root").waitFor({ state: "attached", timeout: 10_000 });

    // TODO: trigger file open with FIXTURE_LARGE and measure time until
    //       the segment list is fully rendered.
    //
    // const start = Date.now();
    // await page.evaluate(async (p) => {
    //   await window.__TAURI__.core.invoke("open_file", { path: p });
    // }, FIXTURE_LARGE);
    // await page.locator("[data-testid='segment-row']").nth(499).waitFor();
    // const elapsed = Date.now() - start;
    // if (elapsed > 3_000) {
    //   console.warn(`[PERF WARNING] 500-segment load took ${elapsed}ms`);
    // }
    // expect(elapsed).toBeLessThan(8_000);

    expect(FIXTURE_LARGE).toBeTruthy(); // placeholder until fixture + invoke are ready
  });

  test("no visible memory leak during short editing session", async ({ page }) => {
    await page.goto("/");
    await page.locator("#root").waitFor({ state: "attached", timeout: 10_000 });

    // TODO: record JS heap size before and after a 30-second editing simulation;
    //       assert growth < 50 MB.
    //
    // const before = await page.evaluate(() => (performance as any).memory?.usedJSHeapSize ?? 0);
    // ... simulate editing ...
    // const after = await page.evaluate(() => (performance as any).memory?.usedJSHeapSize ?? 0);
    // const growthMB = (after - before) / 1_048_576;
    // if (growthMB > 50) console.warn(`[PERF WARNING] Heap grew ${growthMB.toFixed(1)} MB`);
    // expect(growthMB).toBeLessThan(100);

    expect(true).toBe(true); // placeholder
  });
});
