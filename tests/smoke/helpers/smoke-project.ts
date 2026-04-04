/**
 * Smoke test helpers — shared utilities for loading fixture projects
 * without going through native file dialogs.
 *
 * Works by calling window.__smokeTest.setProject(), which is injected
 * by src/main.tsx in DEV mode only.
 *
 * In web mode, window.__smokeTest.setAdminAuth() is also called first to
 * bypass the authentication gate (LoginPage) that would otherwise block
 * the app from rendering the home/project views.
 */

import type { Page } from "@playwright/test";

/** Minimal Project shape matching src/types/index.ts */
export interface SmokeProject {
  id: string;
  name: string;
  sourcePath: string;
  sourceLang: string;
  targetLang: string;
  createdAt: string;
  files: unknown[];
  segments: SmokeSegment[];
}

export interface SmokeSegment {
  id: string;
  source: string;
  target: string;
  status: "untranslated" | "draft" | "translated" | "confirmed";
  tmMatches: unknown[];
  order: number;
}

/** Fixture project derived from tests/smoke/fixtures/sample.xliff */
export const SAMPLE_PROJECT: SmokeProject = {
  id: "smoke-test-xliff-project",
  name: "sample.xliff",
  sourcePath: "/smoke/fixtures/sample.xliff",
  sourceLang: "en",
  targetLang: "ko",
  createdAt: new Date().toISOString(),
  files: [],
  segments: [
    {
      id: "smoke-seg-1",
      source: "Hello, world!",
      target: "",
      status: "untranslated",
      tmMatches: [],
      order: 0,
    },
    {
      id: "smoke-seg-2",
      source: "Translation memory fuzzy match test.",
      target: "",
      status: "untranslated",
      tmMatches: [],
      order: 1,
    },
    {
      id: "smoke-seg-3",
      source: "The quick brown fox jumps over the lazy dog.",
      target: "",
      status: "untranslated",
      tmMatches: [],
      order: 2,
    },
  ],
};

/**
 * Wait until window.__smokeTest is available (injected by main.tsx in DEV mode),
 * then:
 *  1. Call setAdminAuth() to bypass the web-mode login gate if present.
 *  2. Wait for the login page to disappear (if it was shown).
 *  3. Load the given project into the app store, bypassing the file dialog.
 *
 * After this call the app transitions to the ProjectDashboard view.
 */
export async function loadSmokeProject(
  page: Page,
  project: SmokeProject = SAMPLE_PROJECT,
): Promise<void> {
  await page.waitForFunction(
    () => !!(window as { __smokeTest?: unknown }).__smokeTest,
    { timeout: 10_000 },
  );

  // Inject admin auth to bypass the web-mode login gate.
  await page.evaluate(() => {
    (window as { __smokeTest?: { setAdminAuth: () => void } }).__smokeTest!.setAdminAuth();
  });

  // Wait for the login page to detach (React re-renders after auth state change).
  await page.locator(".login-page").waitFor({ state: "detached", timeout: 5_000 }).catch(() => {
    // Login page was not present (Tauri mode or already authenticated) — that's fine.
  });

  await page.evaluate((p) => {
    (window as { __smokeTest?: { setProject: (p: unknown) => void } }).__smokeTest!.setProject(p);
  }, project);
}

/**
 * Load the project and then open the translation editor.
 * After this call the EditorLayout (segment list + editor pane) is visible.
 */
export async function loadProjectAndOpenEditor(
  page: Page,
  project: SmokeProject = SAMPLE_PROJECT,
): Promise<void> {
  await loadSmokeProject(page, project);
  // ProjectDashboard renders an "open editor" button after project is set
  const openBtn = page.locator('[data-testid="open-editor-btn"]');
  await openBtn.waitFor({ state: "visible", timeout: 5_000 });
  await openBtn.click();
}
