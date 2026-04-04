// @vitest-environment jsdom
/**
 * Plugin store unit tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { usePluginStore } from "../pluginStore";
import type { Plugin } from "../../types";

// ── Mock adapter ──────────────────────────────────────────────────────────────

const mockPlugin: Plugin = {
  id: "p1",
  name: "Test MT Plugin",
  version: "1.0.0",
  kind: "MtProvider",
  enabled: true,
  params: [{ key: "apiKey", label: "API Key", value: "secret", secret: true }],
  error: null,
  installedAt: "2026-04-04T00:00:00Z",
};

vi.mock("../../adapters", () => ({
  adapter: {
    pluginList: vi.fn(),
    pluginInstall: vi.fn(),
    pluginUpdate: vi.fn(),
    pluginRemove: vi.fn(),
  },
  fileRefFromDrop: vi.fn((f: File) => `web-file://${f.name}`),
  isTauri: vi.fn(() => false),
}));

import { adapter } from "../../adapters";

function resetStore() {
  usePluginStore.setState({ plugins: [], loading: false, error: null });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

// ── fetchPlugins ──────────────────────────────────────────────────────────────

describe("fetchPlugins()", () => {
  it("populates plugins on success", async () => {
    vi.mocked(adapter.pluginList).mockResolvedValueOnce([mockPlugin]);
    await usePluginStore.getState().fetchPlugins();
    expect(usePluginStore.getState().plugins).toHaveLength(1);
    expect(usePluginStore.getState().plugins[0].id).toBe("p1");
    expect(usePluginStore.getState().loading).toBe(false);
  });

  it("sets error on failure", async () => {
    vi.mocked(adapter.pluginList).mockRejectedValueOnce(new Error("network error"));
    await usePluginStore.getState().fetchPlugins();
    expect(usePluginStore.getState().error).toBe("network error");
    expect(usePluginStore.getState().plugins).toHaveLength(0);
  });

  it("sets loading=true while fetching", async () => {
    let resolve!: (v: Plugin[]) => void;
    vi.mocked(adapter.pluginList).mockReturnValueOnce(
      new Promise<Plugin[]>((r) => { resolve = r; })
    );
    const promise = usePluginStore.getState().fetchPlugins();
    expect(usePluginStore.getState().loading).toBe(true);
    resolve([]);
    await promise;
    expect(usePluginStore.getState().loading).toBe(false);
  });
});

// ── installPlugin ─────────────────────────────────────────────────────────────

describe("installPlugin()", () => {
  it("appends installed plugin to list", async () => {
    vi.mocked(adapter.pluginInstall).mockResolvedValueOnce(mockPlugin);
    const result = await usePluginStore.getState().installPlugin({ wasmFile: "web-file://x" });
    expect(result.id).toBe("p1");
    expect(usePluginStore.getState().plugins).toHaveLength(1);
  });

  it("propagates error and stores error message", async () => {
    vi.mocked(adapter.pluginInstall).mockRejectedValueOnce(new Error("invalid wasm"));
    await expect(
      usePluginStore.getState().installPlugin({ wasmFile: "web-file://x" })
    ).rejects.toThrow("invalid wasm");
    expect(usePluginStore.getState().error).toBe("invalid wasm");
  });
});

// ── togglePlugin ──────────────────────────────────────────────────────────────

describe("togglePlugin()", () => {
  it("updates enabled state for the given plugin", async () => {
    usePluginStore.setState({ plugins: [mockPlugin] });
    const updated = { ...mockPlugin, enabled: false };
    vi.mocked(adapter.pluginUpdate).mockResolvedValueOnce(updated);
    await usePluginStore.getState().togglePlugin("p1", false);
    expect(usePluginStore.getState().plugins[0].enabled).toBe(false);
    expect(adapter.pluginUpdate).toHaveBeenCalledWith("p1", { enabled: false });
  });

  it("does not change other plugins", async () => {
    const other: Plugin = { ...mockPlugin, id: "p2", name: "Other" };
    usePluginStore.setState({ plugins: [mockPlugin, other] });
    vi.mocked(adapter.pluginUpdate).mockResolvedValueOnce({ ...mockPlugin, enabled: false });
    await usePluginStore.getState().togglePlugin("p1", false);
    expect(usePluginStore.getState().plugins[1].id).toBe("p2");
    expect(usePluginStore.getState().plugins[1].enabled).toBe(true);
  });

  it("sets error on adapter failure", async () => {
    usePluginStore.setState({ plugins: [mockPlugin] });
    vi.mocked(adapter.pluginUpdate).mockRejectedValueOnce(new Error("update failed"));
    await usePluginStore.getState().togglePlugin("p1", false);
    expect(usePluginStore.getState().error).toBe("update failed");
  });
});

// ── updateParams ──────────────────────────────────────────────────────────────

describe("updateParams()", () => {
  it("sends paramValues to adapter and updates store", async () => {
    usePluginStore.setState({ plugins: [mockPlugin] });
    const updated = {
      ...mockPlugin,
      params: [{ key: "apiKey", label: "API Key", value: "new-key", secret: true }],
    };
    vi.mocked(adapter.pluginUpdate).mockResolvedValueOnce(updated);
    await usePluginStore.getState().updateParams("p1", { apiKey: "new-key" });
    expect(adapter.pluginUpdate).toHaveBeenCalledWith("p1", { paramValues: { apiKey: "new-key" } });
    expect(usePluginStore.getState().plugins[0].params[0].value).toBe("new-key");
  });
});

// ── removePlugin ──────────────────────────────────────────────────────────────

describe("removePlugin()", () => {
  it("removes plugin from store on success", async () => {
    usePluginStore.setState({ plugins: [mockPlugin] });
    vi.mocked(adapter.pluginRemove).mockResolvedValueOnce(undefined);
    await usePluginStore.getState().removePlugin("p1");
    expect(usePluginStore.getState().plugins).toHaveLength(0);
    expect(adapter.pluginRemove).toHaveBeenCalledWith("p1");
  });

  it("sets error on failure", async () => {
    usePluginStore.setState({ plugins: [mockPlugin] });
    vi.mocked(adapter.pluginRemove).mockRejectedValueOnce(new Error("remove failed"));
    await usePluginStore.getState().removePlugin("p1");
    expect(usePluginStore.getState().error).toBe("remove failed");
    expect(usePluginStore.getState().plugins).toHaveLength(1);
  });
});

// ── clearError ────────────────────────────────────────────────────────────────

describe("clearError()", () => {
  it("resets error to null", () => {
    usePluginStore.setState({ error: "some error" });
    usePluginStore.getState().clearError();
    expect(usePluginStore.getState().error).toBeNull();
  });
});
