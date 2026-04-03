// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { TbPanel } from "../TbPanel";
import { useProjectStore } from "../../../stores/projectStore";
import { useTbStore } from "../../../stores/tbStore";
import type { Project, TbEntry } from "../../../types";

// Tauri API 모킹
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// Tauri 커맨드 래퍼 모킹
vi.mock("../../../tauri/commands", () => ({
  createTb: vi.fn().mockResolvedValue("tb-id-1"),
  lookupTb: vi.fn().mockResolvedValue([]),
  addToTb: vi.fn().mockResolvedValue({
    id: "tb-entry-1",
    sourceTerm: "Hello",
    targetTerm: "안녕하세요",
    sourceLang: "en-US",
    targetLang: "ko-KR",
    notes: "",
    forbidden: false,
  }),
}));

import { createTb, lookupTb, addToTb } from "../../../tauri/commands";

const mockTbEntries: TbEntry[] = [
  {
    id: "tb-entry-1",
    sourceTerm: "Hello",
    targetTerm: "안녕하세요",
    sourceLang: "en-US",
    targetLang: "ko-KR",
    notes: "일반 인사",
    forbidden: false,
  },
  {
    id: "tb-entry-2",
    sourceTerm: "Goodbye",
    targetTerm: "작별",
    sourceLang: "en-US",
    targetLang: "ko-KR",
    notes: "",
    forbidden: false,
  },
  {
    id: "tb-entry-3",
    sourceTerm: "Ban",
    targetTerm: "금지",
    sourceLang: "en-US",
    targetLang: "ko-KR",
    notes: "사용 금지 용어",
    forbidden: true,
  },
];

const mockProject: Project = {
  id: "proj-1",
  name: "Test Project",
  sourcePath: "/tmp/test.xliff",
  sourceLang: "en-US",
  targetLang: "ko-KR",
  createdAt: "2026-01-01T00:00:00Z",
  segments: [
    {
      id: "seg-1",
      source: "Hello world",
      target: "",
      status: "untranslated",
      tmMatches: [],
      order: 0,
    },
  ],
};

describe("TbPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useProjectStore.setState({ project: null, currentSegmentIndex: 0 });
    useTbStore.setState({ activeTbId: null });
    vi.mocked(createTb).mockResolvedValue("tb-id-1");
    vi.mocked(lookupTb).mockResolvedValue([]);
  });

  describe("빈 상태 처리", () => {
    it("project가 없을 때 '용어 없음' 메시지를 표시한다", () => {
      render(<TbPanel />);
      expect(screen.getByText("용어 없음")).toBeInTheDocument();
    });

    it("Term Base 헤더가 항상 표시된다", () => {
      render(<TbPanel />);
      expect(screen.getByText("Term Base")).toBeInTheDocument();
    });

    it("project가 있지만 TB 용어가 없을 때 '용어 없음' 메시지를 표시한다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      vi.mocked(lookupTb).mockResolvedValue([]);

      render(<TbPanel />);

      await waitFor(() => {
        expect(screen.getByText("용어 없음")).toBeInTheDocument();
      });
    });
  });

  describe("TB 용어 목록 렌더링", () => {
    it("TB 용어 목록을 소스 → 타겟 형식으로 표시한다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      useTbStore.setState({ activeTbId: "tb-id-1" });
      vi.mocked(lookupTb).mockResolvedValue(mockTbEntries);

      render(<TbPanel />);

      await waitFor(() => {
        // 소스 용어
        expect(screen.getByText("Hello")).toBeInTheDocument();
        expect(screen.getByText("Goodbye")).toBeInTheDocument();
        // 타겟 용어
        expect(screen.getByText("안녕하세요")).toBeInTheDocument();
        expect(screen.getByText("작별")).toBeInTheDocument();
      });
    });

    it("용어 노트가 있는 경우 표시한다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      useTbStore.setState({ activeTbId: "tb-id-1" });
      vi.mocked(lookupTb).mockResolvedValue([mockTbEntries[0]]);

      render(<TbPanel />);

      await waitFor(() => {
        expect(screen.getByText("일반 인사")).toBeInTheDocument();
      });
    });

    it("용어가 있을 때 '용어 없음' 메시지를 표시하지 않는다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      useTbStore.setState({ activeTbId: "tb-id-1" });
      vi.mocked(lookupTb).mockResolvedValue(mockTbEntries);

      render(<TbPanel />);

      await waitFor(() => {
        expect(screen.queryByText("용어 없음")).not.toBeInTheDocument();
      });
    });
  });

  describe("금지 용어(forbidden) 표시", () => {
    it("금지 용어에 '금지어' 배지를 표시한다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      useTbStore.setState({ activeTbId: "tb-id-1" });
      vi.mocked(lookupTb).mockResolvedValue([mockTbEntries[2]]); // forbidden: true

      render(<TbPanel />);

      await waitFor(() => {
        expect(screen.getByText("금지어")).toBeInTheDocument();
      });
    });

    it("금지 용어 항목에 'forbidden' CSS 클래스가 적용된다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      useTbStore.setState({ activeTbId: "tb-id-1" });
      vi.mocked(lookupTb).mockResolvedValue([mockTbEntries[2]]); // forbidden: true

      render(<TbPanel />);

      await waitFor(() => {
        const forbiddenEntry = document.querySelector(".tb-entry.forbidden");
        expect(forbiddenEntry).toBeInTheDocument();
      });
    });

    it("일반 용어에는 '금지어' 배지가 없다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      useTbStore.setState({ activeTbId: "tb-id-1" });
      // forbidden: false인 항목만
      vi.mocked(lookupTb).mockResolvedValue([mockTbEntries[0], mockTbEntries[1]]);

      render(<TbPanel />);

      await waitFor(() => {
        expect(screen.queryByText("금지어")).not.toBeInTheDocument();
      });
    });

    it("금지 용어와 일반 용어가 혼합된 경우 금지 용어만 배지를 표시한다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      useTbStore.setState({ activeTbId: "tb-id-1" });
      vi.mocked(lookupTb).mockResolvedValue(mockTbEntries); // 일반 2개 + 금지 1개

      render(<TbPanel />);

      await waitFor(() => {
        const forbiddenBadges = screen.getAllByText("금지어");
        expect(forbiddenBadges).toHaveLength(1);
      });
    });
  });

  describe("새 TB 생성 UI", () => {
    it("'새 TB' 버튼이 항상 표시된다", () => {
      render(<TbPanel />);
      expect(screen.getByTitle("새 TB 만들기")).toBeInTheDocument();
    });

    it("'새 TB' 버튼 클릭 시 생성 폼이 나타난다", () => {
      render(<TbPanel />);
      fireEvent.click(screen.getByTitle("새 TB 만들기"));
      expect(screen.getByPlaceholderText("TB 이름")).toBeInTheDocument();
    });

    it("취소 버튼 클릭 시 생성 폼이 사라진다", () => {
      render(<TbPanel />);
      fireEvent.click(screen.getByTitle("새 TB 만들기"));
      expect(screen.getByPlaceholderText("TB 이름")).toBeInTheDocument();

      const cancelButtons = screen.getAllByText("취소");
      fireEvent.click(cancelButtons[0]);
      expect(screen.queryByPlaceholderText("TB 이름")).not.toBeInTheDocument();
    });

    it("TB 이름 입력 후 만들기 클릭 시 createTb가 호출된다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      vi.mocked(createTb).mockResolvedValueOnce("new-tb-id");

      render(<TbPanel />);
      fireEvent.click(screen.getByTitle("새 TB 만들기"));

      const input = screen.getByPlaceholderText("TB 이름");
      fireEvent.change(input, { target: { value: "My Test TB" } });
      fireEvent.click(screen.getByText("만들기"));

      await waitFor(() => {
        expect(createTb).toHaveBeenCalledWith("My Test TB");
      });
    });

    it("TB 이름 없이 만들기 클릭해도 createTb가 호출되지 않는다", async () => {
      render(<TbPanel />);
      fireEvent.click(screen.getByTitle("새 TB 만들기"));
      fireEvent.click(screen.getByText("만들기"));

      expect(createTb).not.toHaveBeenCalled();
    });
  });

  describe("새 용어 추가 UI", () => {
    it("'+ 용어' 버튼이 항상 표시된다", () => {
      render(<TbPanel />);
      expect(screen.getByTitle("새 용어 추가")).toBeInTheDocument();
    });

    it("'+ 용어' 버튼 클릭 시 용어 입력 폼이 나타난다", () => {
      render(<TbPanel />);
      fireEvent.click(screen.getByTitle("새 용어 추가"));
      expect(screen.getByPlaceholderText("소스 용어")).toBeInTheDocument();
      expect(screen.getByPlaceholderText("타겟 용어")).toBeInTheDocument();
    });

    it("용어 입력 폼의 취소 버튼을 클릭하면 폼이 사라진다", () => {
      render(<TbPanel />);
      fireEvent.click(screen.getByTitle("새 용어 추가"));
      expect(screen.getByPlaceholderText("소스 용어")).toBeInTheDocument();

      const cancelButtons = screen.getAllByText("취소");
      fireEvent.click(cancelButtons[cancelButtons.length - 1]);
      expect(screen.queryByPlaceholderText("소스 용어")).not.toBeInTheDocument();
    });

    it("소스·타겟 용어 입력 후 추가 클릭 시 addToTb가 호출된다", async () => {
      vi.mocked(addToTb).mockResolvedValueOnce(mockTbEntries[0]);
      vi.mocked(lookupTb).mockResolvedValue([]);

      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      useTbStore.setState({ activeTbId: "tb-id-1" });

      render(<TbPanel />);
      fireEvent.click(screen.getByTitle("새 용어 추가"));

      fireEvent.change(screen.getByPlaceholderText("소스 용어"), { target: { value: "Hello" } });
      fireEvent.change(screen.getByPlaceholderText("타겟 용어"), { target: { value: "안녕하세요" } });
      fireEvent.click(screen.getByText("추가"));

      await waitFor(() => {
        expect(addToTb).toHaveBeenCalledWith(
          "tb-id-1",
          "Hello",
          "안녕하세요",
          "en-US",
          "ko-KR",
          "",
          false
        );
      });
    });
  });
});
