import { create } from "zustand";
import type { TbEntry } from "../types";

interface TbState {
  activeTbId: string | null;
  setActiveTbId: (id: string | null) => void;
  /** TB entries matched against the current segment source — shared with SegmentEditor for highlights */
  currentTbEntries: TbEntry[];
  setCurrentTbEntries: (entries: TbEntry[]) => void;
}

export const useTbStore = create<TbState>((set) => ({
  activeTbId: null,
  setActiveTbId: (id) => set({ activeTbId: id }),
  currentTbEntries: [],
  setCurrentTbEntries: (entries) => set({ currentTbEntries: entries }),
}));
