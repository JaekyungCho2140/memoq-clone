import { create } from "zustand";
import type { MtProvider, MtResult } from "../types";

interface MtState {
  provider: MtProvider;
  apiKey: string;
  /** Cache: segmentId → MT result */
  cache: Record<string, MtResult>;
  setProvider: (provider: MtProvider) => void;
  setApiKey: (key: string) => void;
  cacheResult: (segmentId: string, result: MtResult) => void;
  getCached: (segmentId: string) => MtResult | null;
  clearCache: () => void;
}

export const useMtStore = create<MtState>((set, get) => ({
  provider: "deepl",
  apiKey: "",
  cache: {},

  setProvider: (provider) => set({ provider }),
  setApiKey: (apiKey) => set({ apiKey }),

  cacheResult: (segmentId, result) =>
    set((state) => ({ cache: { ...state.cache, [segmentId]: result } })),

  getCached: (segmentId) => get().cache[segmentId] ?? null,

  clearCache: () => set({ cache: {} }),
}));
