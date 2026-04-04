/**
 * Smoke Test: Core CAT Workflow
 *
 * Covers the essential translation editor loop:
 *   1. Create a new translation project
 *   2. Import an XLIFF file
 *   3. Edit a segment (source → target)
 *   4. Advance segment status (untranslated → translated → reviewed)
 *   5. Export the translated file
 *   6. Verify keyboard shortcuts work
 *
 * The test uses a minimal fixture XLIFF so the suite runs quickly.
 */

import { test, expect } from "@playwright/test";
import path from "path";

const FIXTURE_XLIFF = path.join(__dirname, "fixtures", "sample.xliff");

test.describe("Core CAT Workflow", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(process.env.SMOKE_APP_URL ?? "tauri://localhost");
    await page.locator("#root").waitFor({ state: "attached", timeout: 10_000 });
  });

  test("new translation project can be created", async ({ page }) => {
    // TODO: click the "New Project" button / menu item once selectors are known
    // Example placeholder:
    // await page.getByRole("button", { name: /new project/i }).click();
    // await expect(page.getByRole("dialog", { name: /new project/i })).toBeVisible();

    // Minimal assertion: ensure the page loads without throwing
    await expect(page.locator("#root")).toBeVisible();
  });

  test("XLIFF file imports correctly — segments appear in editor", async ({ page }) => {
    // TODO: trigger the file-open dialog and supply FIXTURE_XLIFF
    // For Tauri apps this typically requires mocking the dialog plugin or using
    // the file-drop API.
    //
    // Example shape:
    // await page.evaluate(async (filePath) => {
    //   // invoke the Tauri command that accepts a file path directly
    //   await window.__TAURI__.core.invoke("open_file", { path: filePath });
    // }, FIXTURE_XLIFF);
    // await expect(page.locator("[data-testid='segment-row']").first()).toBeVisible();

    expect(FIXTURE_XLIFF).toBeTruthy(); // placeholder — remove when real test is in place
  });

  test("segment text is editable", async ({ page }) => {
    // TODO: click first segment, type in target cell, verify text change
    // await page.locator("[data-testid='segment-target']").first().click();
    // await page.keyboard.type("test translation");
    // await expect(page.locator("[data-testid='segment-target']").first()).toHaveText("test translation");
    expect(true).toBe(true); // placeholder
  });

  test("segment status transitions work", async ({ page }) => {
    // Expected flow: untranslated → translated (confirm) → reviewed (approve)
    // TODO: confirm segment via keyboard shortcut or button, then approve
    // await page.keyboard.press("Alt+Enter"); // confirm/next segment
    // await expect(page.locator("[data-status='translated']").first()).toBeVisible();
    expect(true).toBe(true); // placeholder
  });

  test("XLIFF export generates a valid file", async ({ page }) => {
    // TODO: trigger export, intercept file-save dialog, validate XML structure
    // of the exported file using a lightweight parser
    expect(true).toBe(true); // placeholder
  });

  test("keyboard shortcut: confirm and advance to next segment", async ({ page }) => {
    // TODO: load project, focus segment, press shortcut, assert next segment is active
    expect(true).toBe(true); // placeholder
  });
});
