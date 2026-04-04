# Smoke Tests — memoQ Clone

This directory contains the **Gate 2 Staging smoke tests** for the memoQ Clone application.

## Overview

These tests are run by `staging.yml` on real macOS and Windows CI runners after the installer is deployed. They verify that the core application functionality works end-to-end in a production-like environment.

## Structure

| File | Coverage area |
|------|---------------|
| `app-launch.spec.ts` | App launches, main window renders, no console errors |
| `core-workflow.spec.ts` | Create project, XLIFF import/export, segment editing, shortcuts |
| `tm-operations.spec.ts` | TM connect/disconnect, fuzzy match, concordance, alignment engine |
| `file-operations.spec.ts` | XLIFF 1.2 / 2.0 import, export round-trip, terminology extraction |
| `offline-mode.spec.ts` | Network-disconnected graceful fallback, reconnect recovery |
| `plugin-vendor.spec.ts` | Plugin system (WASM), vendor portal UI |
| `performance.spec.ts` | Startup time, 500-segment load time, memory baseline |

## Running Locally

```bash
# Install Playwright (first time)
npx playwright install

# Run all smoke tests
npm run test:smoke

# Run a specific suite
npx playwright test tests/smoke/app-launch.spec.ts --project=smoke
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SMOKE_APP_URL` | `tauri://localhost` | URL of the running app's webview |

## Fixtures

Place test fixture files under `tests/smoke/fixtures/`:

- `sample.xliff` — minimal XLIFF 1.2 file (a few segments)
- `sample-xliff12.xliff` — standard XLIFF 1.2 sample
- `sample-xliff20.xlf` — standard XLIFF 2.0 sample
- `500-segments.xliff` — large XLIFF file for performance tests
- `한국어/sample.xliff` — fixture at a Unicode path (special-char path test)

## Playwright Configuration

Add a `smoke` project to `playwright.config.ts`:

```ts
import { defineConfig } from "@playwright/test";

export default defineConfig({
  projects: [
    {
      name: "smoke",
      testDir: "./tests/smoke",
      use: {
        baseURL: process.env.SMOKE_APP_URL ?? "tauri://localhost",
        // Tauri desktop apps use a custom protocol; configure accordingly
      },
    },
  ],
});
```

## Status

All test bodies are currently **placeholders** — they pass but do not yet exercise real UI interactions. Each `TODO` comment describes what the final assertion should do. Implement one suite at a time as the relevant UI becomes stable.
