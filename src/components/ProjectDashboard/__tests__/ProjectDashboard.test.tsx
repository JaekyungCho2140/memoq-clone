// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom";
import { ProjectDashboard } from "../ProjectDashboard";
import { useProjectStore } from "../../../stores/projectStore";
import type { Project } from "../../../types";

const mockAdapter = vi.hoisted(() => ({
  openFileDialog: vi.fn().mockResolvedValue(null),
  addFileToProject: vi.fn(),
  parseFile: vi.fn(),
}));

vi.mock("../../../adapters", () => ({
  adapter: mockAdapter,
  isTauri: vi.fn(() => false),
  fileRefFromDrop: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockProject: Project = {
  id: "proj-1",
  name: "테스트 프로젝트",
  sourcePath: "/tmp/test.xliff",
  sourceLang: "en-US",
  targetLang: "ko-KR",
  createdAt: "2026-01-01T00:00:00Z",
  segments: [
    { id: "seg-1", source: "Hello", target: "", status: "untranslated", tmMatches: [], order: 0 },
    { id: "seg-2", source: "World", target: "세계", status: "translated", tmMatches: [], order: 1 },
    { id: "seg-3", source: "Bye", target: "잘 가", status: "confirmed", tmMatches: [], order: 2 },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  useProjectStore.setState({ project: null, currentSegmentIndex: 0, projectView: "dashboard" });
});

describe("ProjectDashboard", () => {
  it("project가 없으면 아무것도 렌더링하지 않는다 (빈 상태)", () => {
    const { container } = render(<ProjectDashboard />);
    expect(container).toBeEmptyDOMElement();
  });

  it("project가 있으면 프로젝트 이름을 표시한다", () => {
    useProjectStore.setState({ project: mockProject });
    render(<ProjectDashboard />);
    expect(screen.getByText("테스트 프로젝트")).toBeInTheDocument();
  });

  it("번역 진행률 통계가 표시된다 (데이터 있는 상태)", () => {
    useProjectStore.setState({ project: mockProject });
    render(<ProjectDashboard />);
    // 3 segments, 2 translated/confirmed → 66% or 67%
    const pctEls = screen.getAllByText(/66%|67%/);
    expect(pctEls.length).toBeGreaterThan(0);
  });

  it("소스/타겟 언어가 표시된다", () => {
    useProjectStore.setState({ project: mockProject });
    render(<ProjectDashboard />);
    expect(screen.getByText(/en-US/i)).toBeInTheDocument();
    expect(screen.getByText(/ko-KR/i)).toBeInTheDocument();
  });

  it("번역 편집기 열기 버튼이 존재한다", () => {
    useProjectStore.setState({ project: mockProject });
    render(<ProjectDashboard />);
    expect(screen.getByTestId("open-editor-btn")).toBeInTheDocument();
  });
});
