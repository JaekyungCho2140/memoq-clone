import { describe, it, expect, beforeEach } from "vitest";
import { useProjectStore } from "../../../stores/projectStore";
import type { Project } from "../../../types";

const mockProject: Project = {
  id: "proj-1",
  name: "Test Project",
  sourcePath: "/tmp/test.xliff",
  sourceLang: "en-US",
  targetLang: "ko-KR",
  createdAt: "2026-01-01T00:00:00Z",
  segments: [
    { id: "seg-1", source: "Hello", target: "", status: "untranslated", tmMatches: [], order: 0 },
    { id: "seg-2", source: "World", target: "", status: "untranslated", tmMatches: [], order: 1 },
    { id: "seg-3", source: "Foo",   target: "", status: "untranslated", tmMatches: [], order: 2 },
  ],
};

describe("projectStore", () => {
  beforeEach(() => {
    useProjectStore.setState({ project: null, currentSegmentIndex: 0 });
  });

  it("setProject initializes project and resets index to 0", () => {
    useProjectStore.getState().setProject(mockProject);
    const { project, currentSegmentIndex } = useProjectStore.getState();
    expect(project?.id).toBe("proj-1");
    expect(project?.segments).toHaveLength(3);
    expect(currentSegmentIndex).toBe(0);
  });

  it("updateSegment sets target and status immutably", () => {
    useProjectStore.getState().setProject(mockProject);
    useProjectStore.getState().updateSegment("seg-1", { target: "안녕", status: "draft" });

    const { project } = useProjectStore.getState();
    expect(project!.segments[0].target).toBe("안녕");
    expect(project!.segments[0].status).toBe("draft");
    // 다른 세그먼트는 변경되지 않아야 함
    expect(project!.segments[1].target).toBe("");
    expect(project!.segments[1].status).toBe("untranslated");
  });

  it("status transition: untranslated → draft → confirmed", () => {
    useProjectStore.getState().setProject(mockProject);

    useProjectStore.getState().updateSegment("seg-1", { target: "안녕", status: "draft" });
    expect(useProjectStore.getState().project!.segments[0].status).toBe("draft");

    useProjectStore.getState().updateSegment("seg-1", { status: "confirmed" });
    expect(useProjectStore.getState().project!.segments[0].status).toBe("confirmed");
  });

  it("status transition: untranslated → translated → confirmed", () => {
    useProjectStore.getState().setProject(mockProject);

    useProjectStore.getState().updateSegment("seg-2", { target: "세계", status: "translated" });
    expect(useProjectStore.getState().project!.segments[1].status).toBe("translated");

    useProjectStore.getState().updateSegment("seg-2", { status: "confirmed" });
    expect(useProjectStore.getState().project!.segments[1].status).toBe("confirmed");
  });

  it("setCurrentSegmentIndex updates active segment index", () => {
    useProjectStore.getState().setProject(mockProject);
    useProjectStore.getState().setCurrentSegmentIndex(2);
    expect(useProjectStore.getState().currentSegmentIndex).toBe(2);
  });

  it("closeProject resets to initial state", () => {
    useProjectStore.getState().setProject(mockProject);
    useProjectStore.getState().setCurrentSegmentIndex(1);
    useProjectStore.getState().closeProject();

    const { project, currentSegmentIndex } = useProjectStore.getState();
    expect(project).toBeNull();
    expect(currentSegmentIndex).toBe(0);
  });

  it("updateSegment on null project is a no-op", () => {
    // project가 null인 상태에서 updateSegment를 호출해도 crash 없어야 함
    expect(() =>
      useProjectStore.getState().updateSegment("seg-1", { target: "test" })
    ).not.toThrow();
    expect(useProjectStore.getState().project).toBeNull();
  });
});
