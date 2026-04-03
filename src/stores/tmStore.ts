import { create } from "zustand";

interface TmState {
  activeTmId: string | null;
  setActiveTmId: (id: string | null) => void;
}

export const useTmStore = create<TmState>((set) => ({
  activeTmId: null,
  setActiveTmId: (id) => set({ activeTmId: id }),
}));
