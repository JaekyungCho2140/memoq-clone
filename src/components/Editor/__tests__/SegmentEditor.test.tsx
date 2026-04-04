// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom";
import { SegmentEditor } from "../SegmentEditor";
import { useProjectStore } from "../../../stores/projectStore";
import { useMtStore } from "../../../stores/mtStore";
import { useWsStore } from "../../../stores/wsStore";
import { useAuthStore } from "../../../stores/authStore";
import { useTmStore } from "../../../stores/tmStore";
import { useTbStore } from "../../../stores/tbStore";
import type { Project } from "../../../types";

// ── Mocks ──────────────────────────────────────────────────────────────────────

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockAdapter = vi.hoisted(() => ({
  saveSegment: vi.fn().mockResolvedValue({}),
  mtTranslate: vi.fn().mockResolvedValue({ source: "Hello", target: "안녕", provider: "deepl" }),
}));

vi.mock("../../../adapters", () => ({
  adapter: mockAdapter,
  isTauri: vi.fn(() => false),
}));

vi.mock("../../../hooks/useSegmentWs", () => ({
  useSegmentWs: vi.fn(() => ({
    lockSegment: vi.fn(),
    unlockSegment: vi.fn(),
    updateSegment: vi.fn(),
  })),
}));

// ── Fixtures ──────────────────────────────────────────────────────────────────

const mockProject: Project = {
  id: "proj-1",
  name: "테스트 프로젝트",
  sourcePath: "/tmp/test.xliff",
  sourceLang: "en-US",
  targetLang: "ko-KR",
  createdAt: "2026-01-01T00:00:00Z",
  segments: [
    { id: "seg-1", source: "Hello World", target: "", status: "untranslated", tmMatches: [], order: 0 },
    { id: "seg-2", source: "Goodbye", target: "안녕히 가세요", status: "translated", tmMatches: [], order: 1 },
  ],
};

function resetStores() {
  useProjectStore.setState({ project: null, currentSegmentIndex: 0, projectView: "editor" });
  useMtStore.setState({ provider: "deepl", apiKey: "", cache: {} });
  useWsStore.setState({ locks: {}, status: "disconnected" });
  useAuthStore.setState({ user: { id: "u1", username: "user1", role: "admin" }, accessToken: null, refreshToken: null, isLoading: false, error: null });
  useTmStore.setState({ activeTmId: null, currentTmMatches: [] });
  useTbStore.setState({ activeTbId: null, currentTbEntries: [] });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStores();
});

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("SegmentEditor", () => {
  it("project가 없으면 아무것도 렌더링하지 않는다 (빈 상태)", () => {
    const { container } = render(<SegmentEditor />);
    expect(container).toBeEmptyDOMElement();
  });

  it("project가 있지만 세그먼트가 없으면 선택 안내 메시지를 표시한다", () => {
    useProjectStore.setState({ project: mockProject, currentSegmentIndex: 999 });
    render(<SegmentEditor />);
    expect(screen.getByText("세그먼트를 선택하세요")).toBeInTheDocument();
  });

  it("선택된 세그먼트의 소스 텍스트가 표시된다 (데이터 있는 상태)", () => {
    useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
    render(<SegmentEditor />);
    expect(screen.getByLabelText("소스 텍스트")).toBeInTheDocument();
    expect(screen.getByLabelText("소스 텍스트")).toHaveTextContent("Hello World");
  });

  it("타겟 텍스트 입력 영역이 렌더링된다", () => {
    useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
    render(<SegmentEditor />);
    expect(screen.getByRole("textbox")).toBeInTheDocument();
  });

  it("다른 사용자가 잠근 세그먼트에는 잠금 배너가 표시된다 (에러 상태)", () => {
    useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
    useWsStore.setState({
      locks: { "seg-1": { userId: "other-user", username: "홍길동" } },
      status: "connected",
    });
    render(<SegmentEditor />);
    expect(screen.getByRole("status")).toBeInTheDocument();
    expect(screen.getByText(/홍길동님이 편집 중입니다/)).toBeInTheDocument();
  });
});
