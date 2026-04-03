// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { QaPanel } from "../QaPanel";
import { QaIssueItem } from "../QaIssueItem";
import { useQaStore } from "../../../stores/qaStore";
import { useProjectStore } from "../../../stores/projectStore";
import type { QaIssue, Project } from "../../../types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../../../tauri/commands", () => ({
  runQaCheck: vi.fn().mockResolvedValue([]),
}));

const mockProject: Project = {
  id: "proj-1",
  name: "Test",
  sourcePath: "/tmp/test.xliff",
  sourceLang: "en-US",
  targetLang: "ko-KR",
  createdAt: "2026-01-01T00:00:00Z",
  segments: [
    { id: "seg-1", source: "Hello", target: "", status: "untranslated", tmMatches: [], order: 0 },
    { id: "seg-2", source: "World", target: "세계", status: "translated", tmMatches: [], order: 1 },
  ],
};

const mockIssues: QaIssue[] = [
  { segment_id: "seg-1", check_type: "Untranslated", severity: "Error", message: "타겟이 비어 있습니다" },
  { segment_id: "seg-2", check_type: "NumberMismatch", severity: "Warning", message: "숫자 불일치" },
];

describe("qaStore", () => {
  beforeEach(() => {
    useQaStore.setState({ issues: [], isRunning: false, lastRunAt: null });
  });

  it("setIssues가 issues와 lastRunAt을 업데이트한다", () => {
    const { setIssues } = useQaStore.getState();
    setIssues(mockIssues);
    const state = useQaStore.getState();
    expect(state.issues).toHaveLength(2);
    expect(state.lastRunAt).not.toBeNull();
  });

  it("errorCount가 Error severity 이슈 수를 반환한다", () => {
    useQaStore.setState({ issues: mockIssues });
    expect(useQaStore.getState().errorCount()).toBe(1);
  });

  it("warningCount가 Warning severity 이슈 수를 반환한다", () => {
    useQaStore.setState({ issues: mockIssues });
    expect(useQaStore.getState().warningCount()).toBe(1);
  });

  it("issuesBySegmentId가 해당 세그먼트 이슈만 반환한다", () => {
    useQaStore.setState({ issues: mockIssues });
    const issues = useQaStore.getState().issuesBySegmentId("seg-1");
    expect(issues).toHaveLength(1);
    expect(issues[0].check_type).toBe("Untranslated");
  });

  it("clearIssues가 issues와 lastRunAt을 초기화한다", () => {
    useQaStore.setState({ issues: mockIssues, lastRunAt: "2026-01-01T00:00:00Z" });
    useQaStore.getState().clearIssues();
    const state = useQaStore.getState();
    expect(state.issues).toHaveLength(0);
    expect(state.lastRunAt).toBeNull();
  });

  it("setRunning이 isRunning을 토글한다", () => {
    useQaStore.getState().setRunning(true);
    expect(useQaStore.getState().isRunning).toBe(true);
    useQaStore.getState().setRunning(false);
    expect(useQaStore.getState().isRunning).toBe(false);
  });
});

describe("QaPanel", () => {
  beforeEach(() => {
    useQaStore.setState({ issues: [], isRunning: false, lastRunAt: null });
  });

  it("초기 상태에서 안내 메시지를 표시한다", () => {
    render(<QaPanel />);
    expect(screen.getByText("F9를 눌러 QA 체크를 실행하세요")).toBeInTheDocument();
  });

  it("isRunning이 true면 '실행 중' 메시지를 표시한다", () => {
    useQaStore.setState({ isRunning: true });
    render(<QaPanel />);
    expect(screen.getByText("QA 체크 실행 중...")).toBeInTheDocument();
  });

  it("이슈 없이 lastRunAt이 있으면 '문제 없음' 메시지를 표시한다", () => {
    useQaStore.setState({ issues: [], lastRunAt: "2026-01-01T00:00:00Z" });
    render(<QaPanel />);
    expect(screen.getByText("문제 없음 ✓")).toBeInTheDocument();
  });

  it("오류 섹션에 오류 이슈가 표시된다", () => {
    useQaStore.setState({ issues: mockIssues, lastRunAt: "2026-01-01T00:00:00Z" });
    useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
    render(<QaPanel />);
    expect(screen.getByText("오류 (1)")).toBeInTheDocument();
    expect(screen.getByText("경고 (1)")).toBeInTheDocument();
  });

  it("QA 헤더가 항상 표시된다", () => {
    render(<QaPanel />);
    expect(screen.getByText("QA 체크")).toBeInTheDocument();
  });
});

describe("QaIssueItem", () => {
  beforeEach(() => {
    useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
  });

  it("이슈 메시지와 체크 타입 레이블을 표시한다", () => {
    render(<QaIssueItem issue={mockIssues[0]} />);
    expect(screen.getByText("타겟이 비어 있습니다")).toBeInTheDocument();
    expect(screen.getByText("미번역")).toBeInTheDocument();
  });

  it("클릭 시 해당 세그먼트로 인덱스가 변경된다", () => {
    render(<QaIssueItem issue={mockIssues[1]} />);
    const item = screen.getByText("⚠").closest(".qa-issue-item");
    fireEvent.click(item!);
    expect(useProjectStore.getState().currentSegmentIndex).toBe(1);
  });

  it("Error 이슈에는 에러 아이콘(✕)이 표시된다", () => {
    render(<QaIssueItem issue={mockIssues[0]} />);
    expect(screen.getByText("✕")).toBeInTheDocument();
  });

  it("Warning 이슈에는 경고 아이콘(⚠)이 표시된다", () => {
    render(<QaIssueItem issue={mockIssues[1]} />);
    expect(screen.getByText("⚠")).toBeInTheDocument();
  });
});
