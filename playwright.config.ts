import { defineConfig } from "@playwright/test";

/**
 * Playwright configuration for Gate 2 Staging smoke tests.
 *
 * The `smoke` project targets a running Tauri webview.
 * Set SMOKE_APP_URL to point to the app's webview endpoint before running.
 *
 * Usage:
 *   npm run test:smoke
 *   npx playwright test tests/smoke/ --project=smoke
 */
export default defineConfig({
  testDir: "./tests",
  timeout: 30_000,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["github"], ["html", { open: "never" }]] : "list",

  projects: [
    {
      name: "smoke",
      testDir: "./tests/smoke",
      use: {
        baseURL: process.env.SMOKE_APP_URL ?? "tauri://localhost",
        // Desktop viewport matching tauri.conf.json default
        viewport: { width: 1280, height: 800 },
        // Capture screenshot on failure for CI artifact upload
        screenshot: "only-on-failure",
        // Capture video on retry
        video: "on-first-retry",
      },
    },
  ],
});
