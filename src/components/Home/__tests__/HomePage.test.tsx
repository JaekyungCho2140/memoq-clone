// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom";
import { HomePage } from "../HomePage";
import { useProjectStore } from "../../../stores/projectStore";

// ── Mocks ──────────────────────────────────────────────────────────────────────

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockAdapter = vi.hoisted(() => ({
  getRecentProjects: vi.fn().mockResolvedValue([]),
  openFileDialog: vi.fn().mockResolvedValue(null),
  parseFile: vi.fn(),
  loadProject: vi.fn(),
}));

vi.mock("../../../adapters", () => ({
  adapter: mockAdapter,
  isTauri: vi.fn(() => false),
}));

beforeEach(() => {
  vi.clearAllMocks();
  useProjectStore.setState({ project: null, currentSegmentIndex: 0, projectView: "dashboard" });
});

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("HomePage", () => {
  it("홈 페이지가 렌더링된다 (빈 상태)", async () => {
    render(<HomePage />);
    await waitFor(() => {
      // 번역 파일 열기, 프로젝트 열기 등 액션 버튼이 렌더링된다
      const btns = screen.getAllByRole("button");
      expect(btns.length).toBeGreaterThan(0);
    });
    expect(screen.getByText("memoQ Clone")).toBeInTheDocument();
  });

  it("최근 프로젝트가 없으면 빈 목록이 표시된다", async () => {
    mockAdapter.getRecentProjects.mockResolvedValue([]);
    render(<HomePage />);
    await waitFor(() => {
      expect(mockAdapter.getRecentProjects).toHaveBeenCalled();
    });
  });

  it("최근 프로젝트 목록이 있으면 렌더링된다 (데이터 있는 상태)", async () => {
    mockAdapter.getRecentProjects.mockResolvedValue(["/tmp/project1.xliff", "/tmp/project2.xliff"]);
    render(<HomePage />);
    await waitFor(() => {
      const matches = screen.getAllByText(/project1\.xliff/);
      expect(matches.length).toBeGreaterThan(0);
    });
  });

  it("onOpenAlignment prop이 있으면 TM 정렬 버튼이 표시된다", async () => {
    const onOpenAlignment = vi.fn();
    render(<HomePage onOpenAlignment={onOpenAlignment} />);
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /정렬|alignment/i })).toBeInTheDocument();
    });
  });
});
