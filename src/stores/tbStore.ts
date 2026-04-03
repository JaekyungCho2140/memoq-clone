import { create } from "zustand";

interface TbState {
  activeTbId: string | null;
  setActiveTbId: (id: string | null) => void;
}

export const useTbStore = create<TbState>((set) => ({
  activeTbId: null,
  setActiveTbId: (id) => set({ activeTbId: id }),
}));
