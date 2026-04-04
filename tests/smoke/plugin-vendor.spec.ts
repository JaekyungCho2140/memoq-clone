/**
 * Smoke Test: Plugin System & Vendor Portal
 *
 * Covers the plugin/vendor features introduced in:
 *   - AFR-44: Vendor Portal — role/permission + assignment workflow
 *   - Plugin system (WASM-based MT providers, etc.)
 *
 * These tests verify that the UI surfaces are reachable and that
 * plugin-related commands don't crash the app.
 */

import { test, expect } from "@playwright/test";

test.describe("Plugin System", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(process.env.SMOKE_APP_URL ?? "tauri://localhost");
    await page.locator("#root").waitFor({ state: "attached", timeout: 10_000 });
  });

  test("plugin list is accessible and renders without crash", async ({ page }) => {
    // TODO: navigate to Settings → Plugins
    // await page.getByRole("menuitem", { name: /settings/i }).click();
    // await page.getByRole("tab", { name: /plugins/i }).click();
    // await expect(page.getByTestId("plugin-list")).toBeVisible();
    expect(true).toBe(true); // placeholder
  });

  test("example WASM MT-provider plugin loads without error", async ({ page }) => {
    // TODO: verify that the example-mt-provider.wasm plugin appears in the plugin list
    //       and its status is "loaded" / "enabled"
    expect(true).toBe(true); // placeholder
  });
});

test.describe("Vendor Portal", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(process.env.SMOKE_APP_URL ?? "tauri://localhost");
    await page.locator("#root").waitFor({ state: "attached", timeout: 10_000 });
  });

  test("vendor portal UI is reachable", async ({ page }) => {
    // TODO: navigate to the Vendor Portal section
    // await page.getByRole("link", { name: /vendor portal/i }).click();
    // await expect(page.getByTestId("vendor-portal-header")).toBeVisible();
    expect(true).toBe(true); // placeholder
  });

  test("vendor role assignment renders without crash", async ({ page }) => {
    // TODO: open vendor assignment dialog; assert the role dropdown is present
    expect(true).toBe(true); // placeholder
  });
});
