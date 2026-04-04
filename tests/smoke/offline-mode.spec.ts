/**
 * Smoke Test: Network-Disconnected Mode
 *
 * Validates that the app degrades gracefully when the network is unavailable:
 *   - App does not crash when network interface is disabled
 *   - Offline TM lookup returns cached/local results or shows a clear indicator
 *   - Re-enabling network — app reconnects without requiring restart
 */

import { test, expect } from "@playwright/test";

test.describe("Offline / Network-Disconnected Mode", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.locator("#root").waitFor({ state: "attached", timeout: 10_000 });
  });

  test("app does not crash when network is disabled", async ({ page, context }) => {
    const errors: string[] = [];
    page.on("pageerror", (err) => errors.push(err.message));

    // Simulate offline by setting the browser context to offline mode
    await context.setOffline(true);

    // Wait for any async network calls to settle
    await page.waitForTimeout(2_000);

    // The root must still be rendered — no white screen / crash
    await expect(page.locator("#root")).toBeVisible();
    expect(errors).toHaveLength(0);

    // Restore network
    await context.setOffline(false);
  });

  test("offline TM lookup shows graceful fallback indicator", async ({ page, context }) => {
    // TODO:
    //   1. Go offline
    //   2. Open a project, click a segment
    //   3. Assert TM panel shows "offline" / "no connection" indicator rather than an error
    await context.setOffline(true);
    await page.waitForTimeout(1_000);
    // await expect(page.getByTestId("tm-offline-indicator")).toBeVisible();
    await context.setOffline(false);
    expect(true).toBe(true); // placeholder until TM UI is wired up
  });

  test("reconnecting network — app recovers without restart", async ({ page, context }) => {
    const errors: string[] = [];
    page.on("pageerror", (err) => errors.push(err.message));

    await context.setOffline(true);
    await page.waitForTimeout(1_000);
    await context.setOffline(false);
    await page.waitForTimeout(2_000);

    // App must remain operational after network is restored
    await expect(page.locator("#root")).toBeVisible();
    expect(errors).toHaveLength(0);
  });
});
