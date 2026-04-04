import { defineConfig } from "@playwright/test";

/**
 * Playwright configuration for Gate 2 Staging smoke tests.
 *
 * By default the `smoke` project runs against the Vite dev server at
 * http://localhost:1420. Set SMOKE_APP_URL to override (e.g. tauri://localhost
 * for a real Tauri desktop build).
 *
 * Usage:
 *   npm run test:smoke          # starts Vite automatically, runs all smoke tests
 *   npx playwright test tests/smoke/ --project=smoke
 *   SMOKE_APP_URL=tauri://localhost npx playwright test tests/smoke/ --project=smoke
 */

const BASE_URL = process.env.SMOKE_APP_URL ?? "http://localhost:1420";

export default defineConfig({
  testDir: "./tests",
  timeout: 30_000,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["github"], ["html", { open: "never" }]] : "list",

  // Start the Vite dev server automatically when targeting the web URL.
  // Skipped when SMOKE_APP_URL points to a Tauri/non-localhost endpoint.
  webServer: BASE_URL.startsWith("http://localhost")
    ? {
        command: "npm run dev",
        url: BASE_URL,
        reuseExistingServer: true,
        timeout: 120_000,
      }
    : undefined,

  projects: [
    {
      name: "smoke",
      testDir: "./tests/smoke",
      use: {
        baseURL: BASE_URL,
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
