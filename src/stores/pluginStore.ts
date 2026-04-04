/**
 * Plugin store — manages the list of installed WASM plugins.
 *
 * All mutations go through the adapter so they are platform-agnostic.
 * The UI imports this store and calls the action helpers.
 */

import { create } from "zustand";
import { adapter } from "../adapters";
import type { Plugin, PluginInstallRequest, PluginUpdateRequest } from "../types";

interface PluginState {
  plugins: Plugin[];
  loading: boolean;
  error: string | null;
}

interface PluginActions {
  fetchPlugins(): Promise<void>;
  installPlugin(req: PluginInstallRequest): Promise<Plugin>;
  togglePlugin(id: string, enabled: boolean): Promise<void>;
  updateParams(id: string, paramValues: Record<string, string>): Promise<void>;
  removePlugin(id: string): Promise<void>;
  clearError(): void;
}

export const usePluginStore = create<PluginState & PluginActions>((set) => ({
  plugins: [],
  loading: false,
  error: null,

  async fetchPlugins() {
    set({ loading: true, error: null });
    try {
      const plugins = await adapter.pluginList();
      set({ plugins, loading: false });
    } catch (e) {
      set({ loading: false, error: errorMessage(e) });
    }
  },

  async installPlugin(req: PluginInstallRequest) {
    set({ loading: true, error: null });
    try {
      const plugin = await adapter.pluginInstall(req);
      set((s) => ({ plugins: [...s.plugins, plugin], loading: false }));
      return plugin;
    } catch (e) {
      set({ loading: false, error: errorMessage(e) });
      throw e;
    }
  },

  async togglePlugin(id: string, enabled: boolean) {
    const req: PluginUpdateRequest = { enabled };
    try {
      const updated = await adapter.pluginUpdate(id, req);
      set((s) => ({
        plugins: s.plugins.map((p) => (p.id === id ? updated : p)),
      }));
    } catch (e) {
      set({ error: errorMessage(e) });
    }
  },

  async updateParams(id: string, paramValues: Record<string, string>) {
    const req: PluginUpdateRequest = { paramValues };
    try {
      const updated = await adapter.pluginUpdate(id, req);
      set((s) => ({
        plugins: s.plugins.map((p) => (p.id === id ? updated : p)),
      }));
    } catch (e) {
      set({ error: errorMessage(e) });
    }
  },

  async removePlugin(id: string) {
    try {
      await adapter.pluginRemove(id);
      set((s) => ({ plugins: s.plugins.filter((p) => p.id !== id) }));
    } catch (e) {
      set({ error: errorMessage(e) });
    }
  },

  clearError() {
    set({ error: null });
  },
}));

function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
