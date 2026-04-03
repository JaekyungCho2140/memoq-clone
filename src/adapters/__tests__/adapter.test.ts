// @vitest-environment jsdom
/**
 * Adapter layer unit tests
 *
 * Covers:
 * - isTauri() environment detection
 * - adapter selection (tauri vs web)
 * - fileRefFromDrop() for both environments
 * - web adapter: file registry (registerFile, resolveFile)
 * - web adapter: openFileDialog
 * - web adapter: saveFileDialog
 * - web adapter: mtSaveSettings / mtLoadSettings (localStorage)
 * - web adapter: getRecentProjects (graceful fallback)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// ─── helpers ─────────────────────────────────────────────────────────────────

function setTauriEnv(present: boolean) {
  if (present) {
    (window as unknown as Record<string, unknown>).__TAURI__ = {};
  } else {
    delete (window as unknown as Record<string, unknown>).__TAURI__;
  }
}

function makeFile(name = "test.xliff", content = "<x/>", type = "text/plain"): File {
  return new File([content], name, { type });
}

// ─── isTauri / adapter selection ─────────────────────────────────────────────

describe("isTauri()", () => {
  afterEach(() => {
    setTauriEnv(false);
  });

  it("returns false when __TAURI__ is absent", async () => {
    setTauriEnv(false);
    const { isTauri } = await import("../index");
    expect(isTauri()).toBe(false);
  });

  it("returns true when __TAURI__ is present", async () => {
    setTauriEnv(true);
    // Re-evaluate at runtime (isTauri reads window each call)
    const { isTauri } = await import("../index");
    expect(isTauri()).toBe(true);
  });
});

// ─── fileRefFromDrop ──────────────────────────────────────────────────────────

describe("fileRefFromDrop()", () => {
  afterEach(() => {
    setTauriEnv(false);
    vi.resetModules();
  });

  it("web mode: returns a web-file:// URI for a File without .path", async () => {
    setTauriEnv(false);
    vi.resetModules();
    const { fileRefFromDrop } = await import("../index");
    const file = makeFile("hello.xliff");
    const ref = fileRefFromDrop(file);
    expect(ref).toMatch(/^web-file:\/\//);
  });

  it("web mode: two different files get different refs", async () => {
    setTauriEnv(false);
    vi.resetModules();
    const { fileRefFromDrop } = await import("../index");
    const a = fileRefFromDrop(makeFile("a.xliff"));
    const b = fileRefFromDrop(makeFile("b.xliff"));
    expect(a).not.toBe(b);
  });

  it("tauri mode: uses file.path when present", async () => {
    setTauriEnv(true);
    vi.resetModules();
    const { fileRefFromDrop } = await import("../index");
    const file = Object.assign(makeFile("doc.xliff"), { path: "/Users/foo/doc.xliff" });
    const ref = fileRefFromDrop(file);
    expect(ref).toBe("/Users/foo/doc.xliff");
  });

  it("tauri mode: falls back to file.name when .path is absent", async () => {
    setTauriEnv(true);
    vi.resetModules();
    const { fileRefFromDrop } = await import("../index");
    const file = makeFile("fallback.xliff");
    const ref = fileRefFromDrop(file);
    expect(ref).toBe("fallback.xliff");
  });
});

// ─── web adapter: file registry ──────────────────────────────────────────────

describe("web adapter — file registry (registerFile)", () => {
  beforeEach(() => {
    vi.resetModules();
  });

  it("registerFile returns a web-file:// URI", async () => {
    const { registerFile } = await import("../web");
    const ref = registerFile(makeFile("seg.xliff"));
    expect(ref).toMatch(/^web-file:\/\//);
  });

  it("separate calls return unique URIs", async () => {
    const { registerFile } = await import("../web");
    const r1 = registerFile(makeFile("a.xliff"));
    const r2 = registerFile(makeFile("b.xliff"));
    expect(r1).not.toBe(r2);
  });
});

// ─── web adapter: openFileDialog ─────────────────────────────────────────────

describe("web adapter — openFileDialog", () => {
  let appendSpy: ReturnType<typeof vi.spyOn>;
  let removeSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    vi.resetModules();
    appendSpy = vi.spyOn(document.body, "appendChild");
    removeSpy = vi.spyOn(document.body, "removeChild").mockImplementation(() => null as unknown as Node);
  });

  afterEach(() => {
    appendSpy.mockRestore();
    removeSpy.mockRestore();
  });

  it("returns null when the user cancels (cancel event)", async () => {
    vi.resetModules();
    const origCreate = document.createElement.bind(document);
    const createSpy = vi.spyOn(document, "createElement").mockImplementation((tag: string) => {
      const el = origCreate(tag);
      if (tag === "input") {
        // Simulate immediate cancel
        setTimeout(() => el.dispatchEvent(new Event("cancel")), 0);
      }
      return el;
    });

    const { webAdapter } = await import("../web");
    const result = await webAdapter.openFileDialog();
    expect(result).toBeNull();

    createSpy.mockRestore();
  });

  it("returns a web-file:// ref when a file is selected", async () => {
    vi.resetModules();
    const file = makeFile("selected.xliff");
    const origCreate = document.createElement.bind(document);
    const createSpy = vi.spyOn(document, "createElement").mockImplementation((tag: string) => {
      const el = origCreate(tag) as HTMLInputElement;
      if (tag === "input") {
        // Override click to immediately trigger 'change' with a fake FileList
        el.click = () => {
          Object.defineProperty(el, "files", {
            value: Object.assign([file], { item: () => file, length: 1 }),
          });
          el.dispatchEvent(new Event("change"));
        };
      }
      return el;
    });

    const { webAdapter } = await import("../web");
    const result = await webAdapter.openFileDialog();
    expect(result).toMatch(/^web-file:\/\//);

    createSpy.mockRestore();
  });
});

// ─── web adapter: saveFileDialog ─────────────────────────────────────────────

describe("web adapter — saveFileDialog", () => {
  beforeEach(() => {
    vi.resetModules();
  });

  it("returns the filename from defaultPath", async () => {
    const { webAdapter } = await import("../web");
    const result = await webAdapter.saveFileDialog({ defaultPath: "/home/user/output.xliff" });
    expect(result).toBe("output.xliff");
  });

  it("falls back to first filter extension when defaultPath is absent", async () => {
    const { webAdapter } = await import("../web");
    const result = await webAdapter.saveFileDialog({
      filters: [{ name: "XLIFF", extensions: ["xliff"] }],
    });
    expect(result).toBe("xliff");
  });

  it("returns 'file' when no options are provided", async () => {
    const { webAdapter } = await import("../web");
    const result = await webAdapter.saveFileDialog();
    expect(result).toBe("file");
  });
});

// ─── web adapter: MT settings (localStorage) ─────────────────────────────────

describe("web adapter — mtSaveSettings / mtLoadSettings", () => {
  beforeEach(() => {
    vi.resetModules();
    localStorage.clear();
  });

  it("returns null when no settings have been saved", async () => {
    const { webAdapter } = await import("../web");
    const result = await webAdapter.mtLoadSettings();
    expect(result).toBeNull();
  });

  it("round-trips MT settings through localStorage", async () => {
    const { webAdapter } = await import("../web");
    const settings = { provider: "deepl" as const, apiKey: "secret-key" };
    await webAdapter.mtSaveSettings(settings);
    const loaded = await webAdapter.mtLoadSettings();
    expect(loaded).toEqual(settings);
  });

  it("returns null for corrupted localStorage data", async () => {
    localStorage.setItem("mt_settings", "not-valid-json{{{");
    const { webAdapter } = await import("../web");
    const result = await webAdapter.mtLoadSettings();
    expect(result).toBeNull();
  });
});

// ─── web adapter: getRecentProjects (graceful fallback) ───────────────────────

describe("web adapter — getRecentProjects", () => {
  beforeEach(() => {
    vi.resetModules();
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("returns empty array when server is unavailable", async () => {
    vi.mocked(fetch).mockRejectedValue(new Error("Network error"));
    const { webAdapter } = await import("../web");
    const result = await webAdapter.getRecentProjects();
    expect(result).toEqual([]);
  });

  it("returns empty array on non-2xx response", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response("Not Found", { status: 404 }),
    );
    const { webAdapter } = await import("../web");
    const result = await webAdapter.getRecentProjects();
    expect(result).toEqual([]);
  });
});

// ─── web adapter: liveDocsListLibraries (graceful fallback) ──────────────────

describe("web adapter — liveDocsListLibraries", () => {
  beforeEach(() => {
    vi.resetModules();
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("returns empty array when server is unavailable", async () => {
    vi.mocked(fetch).mockRejectedValue(new Error("Network error"));
    const { webAdapter } = await import("../web");
    const result = await webAdapter.liveDocsListLibraries();
    expect(result).toEqual([]);
  });
});
