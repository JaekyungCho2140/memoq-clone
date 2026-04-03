import { create } from "zustand";
import type { AlignedPair, AlignmentPhase } from "../types";

interface AlignmentState {
  phase: AlignmentPhase;
  sourceLang: string;
  targetLang: string;
  tmId: string | null;
  pairs: AlignedPair[];
  progress: number; // 0–100
  error: string | null;

  // Actions
  setPhase: (phase: AlignmentPhase) => void;
  setConfig: (sourceLang: string, targetLang: string, tmId: string) => void;
  setPairs: (pairs: AlignedPair[]) => void;
  setProgress: (progress: number) => void;
  setError: (error: string | null) => void;
  confirmPair: (id: string) => void;
  unconfirmPair: (id: string) => void;
  deletePair: (id: string) => void;
  editPair: (id: string, source: string, target: string) => void;
  confirmAll: () => void;
  reset: () => void;
}

const initialState = {
  phase: "upload" as AlignmentPhase,
  sourceLang: "en",
  targetLang: "ko",
  tmId: null,
  pairs: [],
  progress: 0,
  error: null,
};

export const useAlignmentStore = create<AlignmentState>((set) => ({
  ...initialState,

  setPhase: (phase) => set({ phase }),

  setConfig: (sourceLang, targetLang, tmId) =>
    set({ sourceLang, targetLang, tmId }),

  setPairs: (pairs) => set({ pairs }),

  setProgress: (progress) => set({ progress }),

  setError: (error) => set({ error }),

  confirmPair: (id) =>
    set((state) => ({
      pairs: state.pairs.map((p) =>
        p.id === id ? { ...p, confirmed: true } : p,
      ),
    })),

  unconfirmPair: (id) =>
    set((state) => ({
      pairs: state.pairs.map((p) =>
        p.id === id ? { ...p, confirmed: false } : p,
      ),
    })),

  deletePair: (id) =>
    set((state) => ({
      pairs: state.pairs.filter((p) => p.id !== id),
    })),

  editPair: (id, source, target) =>
    set((state) => ({
      pairs: state.pairs.map((p) =>
        p.id === id ? { ...p, source, target, modified: true } : p,
      ),
    })),

  confirmAll: () =>
    set((state) => ({
      pairs: state.pairs.map((p) => ({ ...p, confirmed: true })),
    })),

  reset: () => set({ ...initialState }),
}));
