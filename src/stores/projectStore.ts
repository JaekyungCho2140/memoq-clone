import { create } from "zustand";
import type { Project, Segment } from "../types";

interface ProjectState {
  project: Project | null;
  currentSegmentIndex: number;
  setProject: (project: Project) => void;
  closeProject: () => void;
  updateSegment: (id: string, updates: Partial<Segment>) => void;
  setCurrentSegmentIndex: (index: number) => void;
}

export const useProjectStore = create<ProjectState>((set) => ({
  project: null,
  currentSegmentIndex: 0,
  setProject: (project) => set({ project, currentSegmentIndex: 0 }),
  closeProject: () => set({ project: null, currentSegmentIndex: 0 }),
  updateSegment: (id, updates) =>
    set((state) => ({
      project: state.project
        ? { ...state.project, segments: state.project.segments.map((s) => s.id === id ? { ...s, ...updates } : s) }
        : null,
    })),
  setCurrentSegmentIndex: (index) => set({ currentSegmentIndex: index }),
}));
