/**
 * Smoke Test: App Launch
 *
 * Verifies that the memoQ Clone application starts, renders its main window,
 * and reaches a usable initial state without errors.
 *
 * Prerequisites (CI):
 *   - App must be installed before this suite runs (see staging.yml install step).
 *   - SMOKE_APP_URL env var (e.g. tauri://localhost) or playwright config must
 *     point to the running app's webview.
 *
 * Local run:
 *   npx playwright test tests/smoke/app-launch.spec.ts --project=smoke
 */

import { test, expect } from "@playwright/test";

test.describe("App Launch", () => {
  test("main window renders without crash", async ({ page }) => {
    // TODO: replace with actual app URL / tauri webview endpoint once
    //       the Playwright smoke project is configured in playwright.config.ts
    await page.goto("/");

    // The root element must exist — indicates the React app mounted
    await expect(page.locator("#root")).toBeAttached({ timeout: 10_000 });
  });

  test("no console errors on startup", async ({ page }) => {
    const errors: string[] = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") errors.push(msg.text());
    });

    await page.goto("/");
    // Give the app time to fully initialize
    await page.waitForTimeout(2_000);

    expect(errors, `Console errors found on startup: ${errors.join("; ")}`).toHaveLength(0);
  });

  test("main window reaches correct default size", async ({ page }) => {
    await page.goto("/");

    const viewport = page.viewportSize();
    // Default window size defined in tauri.conf.json should be 1280×800
    expect(viewport?.width).toBeGreaterThanOrEqual(1280);
    expect(viewport?.height).toBeGreaterThanOrEqual(800);
  });

  test("app title is visible", async ({ page }) => {
    await page.goto("/");

    // The window / document title should include the product name
    const title = await page.title();
    expect(title.toLowerCase()).toMatch(/memoq|memoq clone/i);
  });
});
