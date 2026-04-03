/**
 * WebSocket store — tracks real-time segment lock state.
 *
 * Populated by the useSegmentWs hook; consumed by SegmentEditor
 * and the segment list to show locked/editing indicators.
 */

import { create } from "zustand";

export interface SegmentLockInfo {
  userId: string;
  username: string;
}

interface WsState {
  /** segment_id → who is editing it */
  locks: Record<string, SegmentLockInfo>;
  /** WebSocket connection status */
  status: "disconnected" | "connecting" | "connected" | "error";

  setLock: (segmentId: string, info: SegmentLockInfo) => void;
  clearLock: (segmentId: string) => void;
  setStatus: (status: WsState["status"]) => void;
  reset: () => void;
}

export const useWsStore = create<WsState>((set) => ({
  locks: {},
  status: "disconnected",

  setLock(segmentId, info) {
    set((s) => ({ locks: { ...s.locks, [segmentId]: info } }));
  },
  clearLock(segmentId) {
    set((s) => {
      const next = { ...s.locks };
      delete next[segmentId];
      return { locks: next };
    });
  },
  setStatus(status) {
    set({ status });
  },
  reset() {
    set({ locks: {}, status: "disconnected" });
  },
}));
