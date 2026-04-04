import { create } from "zustand";
import type { TmMatch } from "../types";

interface TmState {
  activeTmId: string | null;
  setActiveTmId: (id: string | null) => void;
  /** TM matches for the currently active segment — shared with SegmentEditor for hotkeys */
  currentTmMatches: TmMatch[];
  setCurrentTmMatches: (matches: TmMatch[]) => void;
}

export const useTmStore = create<TmState>((set) => ({
  activeTmId: null,
  setActiveTmId: (id) => set({ activeTmId: id }),
  currentTmMatches: [],
  setCurrentTmMatches: (matches) => set({ currentTmMatches: matches }),
}));
