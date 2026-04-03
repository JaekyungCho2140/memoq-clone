import { describe, it, expect, beforeEach } from "vitest";
import { useProjectStore } from "../projectStore";
import type { Project, TmMatch } from "../../types";

const mockMatch: TmMatch = {
  source: "Hello",
  target: "안녕하세요",
  score: 1.0,
  matchType: "exact",
};

const mockProject: Project = {
  id: "proj-tm-1",
  name: "TM Integration Test Project",
  sourcePath: "/tmp/test.xliff",
  sourceLang: "en-US",
  targetLang: "ko-KR",
  createdAt: "2026-01-01T00:00:00Z",
  segments: [
    {
      id: "seg-1",
      source: "Hello",
      target: "",
      status: "untranslated",
      tmMatches: [],
      order: 0,
    },
    {
      id: "seg-2",
      source: "World",
      target: "",
      status: "untranslated",
      tmMatches: [],
      order: 1,
    },
  ],
};

describe("TM 통합 — projectStore tmMatches 반영", () => {
  beforeEach(() => {
    useProjectStore.setState({ project: null, currentSegmentIndex: 0 });
  });

  it("updateSegment으로 tmMatches를 세그먼트에 반영할 수 있다", () => {
    useProjectStore.getState().setProject(mockProject);
    useProjectStore.getState().updateSegment("seg-1", { tmMatches: [mockMatch] });

    const { project } = useProjectStore.getState();
    expect(project!.segments[0].tmMatches).toHaveLength(1);
    expect(project!.segments[0].tmMatches[0].score).toBe(1.0);
    expect(project!.segments[0].tmMatches[0].target).toBe("안녕하세요");
  });

  it("tmMatches 업데이트 시 다른 세그먼트에 영향을 주지 않는다 (불변성)", () => {
    useProjectStore.getState().setProject(mockProject);
    useProjectStore.getState().updateSegment("seg-1", { tmMatches: [mockMatch] });

    const { project } = useProjectStore.getState();
    // seg-1에만 tmMatches 반영
    expect(project!.segments[0].tmMatches).toHaveLength(1);
    // seg-2는 변경 없음
    expect(project!.segments[1].tmMatches).toHaveLength(0);
  });

  it("기존 세그먼트 배열 참조를 변경하지 않는다 (immutability)", () => {
    useProjectStore.getState().setProject(mockProject);
    const before = useProjectStore.getState().project!.segments;

    useProjectStore.getState().updateSegment("seg-1", { tmMatches: [mockMatch] });

    const after = useProjectStore.getState().project!.segments;
    // 새로운 배열 인스턴스여야 함
    expect(after).not.toBe(before);
    // seg-2 객체 자체는 동일한 참조여도 무방하지만 값은 유지
    expect(after[1].tmMatches).toHaveLength(0);
  });

  it("복수의 TM 매치를 동시에 저장할 수 있다", () => {
    const fuzzyMatch: TmMatch = {
      source: "Hi there",
      target: "안녕하세요",
      score: 0.75,
      matchType: "fuzzy",
    };
    useProjectStore.getState().setProject(mockProject);
    useProjectStore.getState().updateSegment("seg-1", { tmMatches: [mockMatch, fuzzyMatch] });

    const { project } = useProjectStore.getState();
    expect(project!.segments[0].tmMatches).toHaveLength(2);
    expect(project!.segments[0].tmMatches[1].matchType).toBe("fuzzy");
  });

  it("tmMatches를 빈 배열로 초기화할 수 있다", () => {
    // 먼저 매치 추가
    useProjectStore.getState().setProject({
      ...mockProject,
      segments: [
        { ...mockProject.segments[0], tmMatches: [mockMatch] },
        mockProject.segments[1],
      ],
    });

    // 빈 배열로 초기화
    useProjectStore.getState().updateSegment("seg-1", { tmMatches: [] });

    const { project } = useProjectStore.getState();
    expect(project!.segments[0].tmMatches).toHaveLength(0);
  });

  it("tmMatches 업데이트와 target 업데이트를 동시에 처리한다", () => {
    useProjectStore.getState().setProject(mockProject);
    useProjectStore.getState().updateSegment("seg-1", {
      target: "안녕하세요",
      status: "draft",
      tmMatches: [mockMatch],
    });

    const { project } = useProjectStore.getState();
    const seg = project!.segments[0];
    expect(seg.target).toBe("안녕하세요");
    expect(seg.status).toBe("draft");
    expect(seg.tmMatches).toHaveLength(1);
  });
});
