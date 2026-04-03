import { create } from "zustand";
import type { Project, Segment } from "../types";

/** Controls which top-level view is shown when a project is loaded. */
export type ProjectView = "dashboard" | "editor";

interface ProjectState {
  project: Project | null;
  currentSegmentIndex: number;
  /** "dashboard" = ProjectDashboard, "editor" = EditorLayout */
  projectView: ProjectView;
  setProject: (project: Project) => void;
  closeProject: () => void;
  updateSegment: (id: string, updates: Partial<Segment>) => void;
  setCurrentSegmentIndex: (index: number) => void;
  /** Switch from dashboard to the translation editor */
  openEditor: () => void;
  /** Switch back from editor to the dashboard */
  backToDashboard: () => void;
}

export const useProjectStore = create<ProjectState>((set) => ({
  project: null,
  currentSegmentIndex: 0,
  projectView: "dashboard",
  setProject: (project) => set({ project, currentSegmentIndex: 0, projectView: "dashboard" }),
  closeProject: () => set({ project: null, currentSegmentIndex: 0, projectView: "dashboard" }),
  updateSegment: (id, updates) =>
    set((state) => ({
      project: state.project
        ? {
            ...state.project,
            segments: state.project.segments.map((s) =>
              s.id === id ? { ...s, ...updates } : s
            ),
          }
        : null,
    })),
  setCurrentSegmentIndex: (index) => set({ currentSegmentIndex: index }),
  openEditor: () => set({ projectView: "editor" }),
  backToDashboard: () => set({ projectView: "dashboard" }),
}));
