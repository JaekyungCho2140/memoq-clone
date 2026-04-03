import { create } from "zustand";
import type { QaIssue } from "../types";

interface QaState {
  issues: QaIssue[];
  isRunning: boolean;
  lastRunAt: string | null;
  setIssues: (issues: QaIssue[]) => void;
  setRunning: (running: boolean) => void;
  clearIssues: () => void;
  issuesBySegmentId: (segmentId: string) => QaIssue[];
  errorCount: () => number;
  warningCount: () => number;
}

export const useQaStore = create<QaState>((set, get) => ({
  issues: [],
  isRunning: false,
  lastRunAt: null,

  setIssues: (issues) =>
    set({ issues, lastRunAt: new Date().toISOString() }),

  setRunning: (running) => set({ isRunning: running }),

  clearIssues: () => set({ issues: [], lastRunAt: null }),

  issuesBySegmentId: (segmentId) =>
    get().issues.filter((i) => i.segment_id === segmentId),

  errorCount: () =>
    get().issues.filter((i) => i.severity === "Error").length,

  warningCount: () =>
    get().issues.filter((i) => i.severity === "Warning").length,
}));
