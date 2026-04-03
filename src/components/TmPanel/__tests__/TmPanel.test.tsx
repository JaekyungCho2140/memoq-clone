// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom";
import { TmPanel } from "../TmPanel";
import { useProjectStore } from "../../../stores/projectStore";
import { useTmStore } from "../../../stores/tmStore";
import type { Project, TmMatch, LiveDocsMatch } from "../../../types";

// vi.mock은 호이스팅되므로 vi.hoisted()로 mockAdapter를 먼저 정의
const mockAdapter = vi.hoisted(() => ({
  createTm: vi.fn().mockResolvedValue("tm-id-1"),
  searchTm: vi.fn().mockResolvedValue([]),
  addToTm: vi.fn().mockResolvedValue({}),
  mtTranslate: vi.fn().mockResolvedValue({ source: "", target: "", provider: "deepl" }),
  liveDocsSearch: vi.fn().mockResolvedValue([]),
  liveDocsListLibraries: vi.fn().mockResolvedValue([]),
}));

vi.mock("../../../adapters", () => ({
  adapter: mockAdapter,
  isTauri: () => false,
  fileRefFromDrop: (file: File) => `web-file://${file.name}`,
}));

const mockMatches: TmMatch[] = [
  { source: "Hello", target: "안녕하세요", score: 1.0, matchType: "exact" },
  { source: "Hi there", target: "안녕", score: 0.75, matchType: "fuzzy" },
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
      source: "Hello",
      target: "",
      status: "untranslated",
      tmMatches: [],
      order: 0,
    },
  ],
};

describe("TmPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useProjectStore.setState({ project: null, currentSegmentIndex: 0 });
    useTmStore.setState({ activeTmId: null });
    mockAdapter.createTm.mockResolvedValue("tm-id-1");
    mockAdapter.searchTm.mockResolvedValue([]);
  });

  describe("빈 상태 렌더링", () => {
    it("project가 없을 때 '매치 없음' 메시지를 표시한다", () => {
      render(<TmPanel />);
      expect(screen.getByText("매치 없음")).toBeInTheDocument();
    });

    it("TM 탭이 항상 표시된다", () => {
      render(<TmPanel />);
      expect(screen.getByText("TM")).toBeInTheDocument();
    });

    it("project가 있지만 TM 매치가 없을 때 '매치 없음' 메시지를 표시한다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      mockAdapter.searchTm.mockResolvedValue([]);

      render(<TmPanel />);

      await waitFor(() => {
        expect(screen.getByText("매치 없음")).toBeInTheDocument();
      });
    });
  });

  describe("TM 매치 목록 렌더링", () => {
    it("TM 매치 목록을 점수, 소스, 타겟과 함께 표시한다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      useTmStore.setState({ activeTmId: "tm-id-1" });
      mockAdapter.searchTm.mockResolvedValue(mockMatches);

      render(<TmPanel />);

      await waitFor(() => {
        // 점수 표시 (100%, 75%)
        expect(screen.getByText("100%")).toBeInTheDocument();
        expect(screen.getByText("75%")).toBeInTheDocument();
        // 소스 텍스트
        expect(screen.getByText("Hello")).toBeInTheDocument();
        expect(screen.getByText("Hi there")).toBeInTheDocument();
        // 타겟 텍스트
        expect(screen.getByText("안녕하세요")).toBeInTheDocument();
        expect(screen.getByText("안녕")).toBeInTheDocument();
      });
    });

    it("매치가 있을 때 '매치 없음' 메시지를 표시하지 않는다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      useTmStore.setState({ activeTmId: "tm-id-1" });
      mockAdapter.searchTm.mockResolvedValue(mockMatches);

      render(<TmPanel />);

      await waitFor(() => {
        expect(screen.queryByText("매치 없음")).not.toBeInTheDocument();
      });
    });
  });

  describe("매치 클릭 시 콜백", () => {
    it("매치 클릭 시 updateSegment가 해당 타겟으로 호출된다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      useTmStore.setState({ activeTmId: "tm-id-1" });
      mockAdapter.searchTm.mockResolvedValue(mockMatches);

      render(<TmPanel />);

      await waitFor(() => {
        expect(screen.getByText("100%")).toBeInTheDocument();
      });

      // 첫 번째 매치 클릭 (100% exact match)
      const matchItems = document.querySelectorAll(".tm-match");
      fireEvent.click(matchItems[0]);

      const { project } = useProjectStore.getState();
      expect(project!.segments[0].target).toBe("안녕하세요");
      expect(project!.segments[0].status).toBe("draft");
    });

    it("두 번째 매치 클릭 시 해당 타겟이 적용된다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      useTmStore.setState({ activeTmId: "tm-id-1" });
      mockAdapter.searchTm.mockResolvedValue(mockMatches);

      render(<TmPanel />);

      await waitFor(() => {
        expect(screen.getByText("75%")).toBeInTheDocument();
      });

      const matchItems = document.querySelectorAll(".tm-match");
      fireEvent.click(matchItems[1]);

      const { project } = useProjectStore.getState();
      expect(project!.segments[0].target).toBe("안녕");
    });
  });

  describe("새 TM 생성 UI", () => {
    it("'새 TM' 버튼이 항상 표시된다", () => {
      render(<TmPanel />);
      expect(screen.getByTitle("새 TM 만들기")).toBeInTheDocument();
    });

    it("'새 TM' 버튼 클릭 시 생성 폼이 나타난다", () => {
      render(<TmPanel />);
      fireEvent.click(screen.getByTitle("새 TM 만들기"));
      expect(screen.getByPlaceholderText("TM 이름")).toBeInTheDocument();
    });

    it("취소 버튼 클릭 시 생성 폼이 사라진다", () => {
      render(<TmPanel />);
      fireEvent.click(screen.getByTitle("새 TM 만들기"));
      expect(screen.getByPlaceholderText("TM 이름")).toBeInTheDocument();

      fireEvent.click(screen.getByText("취소"));
      expect(screen.queryByPlaceholderText("TM 이름")).not.toBeInTheDocument();
    });

    it("TM 이름 입력 후 만들기 클릭 시 createTm이 호출된다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      mockAdapter.createTm.mockResolvedValueOnce("new-tm-id");

      render(<TmPanel />);
      fireEvent.click(screen.getByTitle("새 TM 만들기"));

      const input = screen.getByPlaceholderText("TM 이름");
      fireEvent.change(input, { target: { value: "My Test TM" } });
      fireEvent.click(screen.getByText("만들기"));

      await waitFor(() => {
        expect(mockAdapter.createTm).toHaveBeenCalledWith("My Test TM", "en-US", "ko-KR");
      });
    });

    it("TM 이름 없이 만들기 클릭해도 handleCreateTm이 TM을 생성하지 않는다", async () => {
      // project 없이 렌더링하면 auto-createTm이 호출되지 않음
      render(<TmPanel />);
      fireEvent.click(screen.getByTitle("새 TM 만들기"));

      // 빈 입력으로 만들기 클릭 (createTm은 handleCreateTm 내 조건에서 막힘)
      const beforeCallCount = mockAdapter.createTm.mock.calls.length;
      fireEvent.click(screen.getByText("만들기"));
      // 추가 호출이 없어야 함
      expect(mockAdapter.createTm.mock.calls.length).toBe(beforeCallCount);
    });
  });

  describe("LiveDocs 탭", () => {
    const mockLibraries = [
      { id: "lib-1", name: "Reference Docs", documents: [] },
      { id: "lib-2", name: "Tech Glossary", documents: [] },
    ];

    const mockLiveDocsMatches: LiveDocsMatch[] = [
      { sentence: "Hello world", docPath: "/docs/greetings.txt", score: 0.95 },
      { sentence: "Hello there", docPath: "/docs/phrases.txt", score: 0.80 },
    ];

    it("LiveDocs 탭 클릭 시 LiveDocs 버튼이 활성화된다", () => {
      render(<TmPanel />);
      const liveDocsBtn = screen.getByText("LiveDocs");
      fireEvent.click(liveDocsBtn);
      expect(liveDocsBtn).toHaveClass("active");
    });

    it("liveDocsListLibraries가 라이브러리 반환 시 각 라이브러리에 liveDocsSearch가 자동 호출된다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      mockAdapter.liveDocsListLibraries.mockResolvedValue(mockLibraries);
      mockAdapter.liveDocsSearch.mockResolvedValue([]);

      render(<TmPanel />);
      fireEvent.click(screen.getByText("LiveDocs"));

      await waitFor(() => {
        expect(mockAdapter.liveDocsListLibraries).toHaveBeenCalled();
        expect(mockAdapter.liveDocsSearch).toHaveBeenCalledTimes(2);
        expect(mockAdapter.liveDocsSearch).toHaveBeenCalledWith("Hello", "lib-1", 0.5);
        expect(mockAdapter.liveDocsSearch).toHaveBeenCalledWith("Hello", "lib-2", 0.5);
      });
    });

    it("검색 결과 매치 항목이 문장·경로·점수%로 렌더링된다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      mockAdapter.liveDocsListLibraries.mockResolvedValue(mockLibraries);
      mockAdapter.liveDocsSearch.mockResolvedValueOnce(mockLiveDocsMatches).mockResolvedValue([]);

      render(<TmPanel />);
      fireEvent.click(screen.getByText("LiveDocs"));

      await waitFor(() => {
        expect(screen.getByText("95%")).toBeInTheDocument();
        expect(screen.getByText("Hello world")).toBeInTheDocument();
        expect(screen.getByText("/docs/greetings.txt")).toBeInTheDocument();
      });
    });

    it("매치 클릭 시 세그먼트 target에 해당 sentence가 적용된다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      mockAdapter.liveDocsListLibraries.mockResolvedValue([mockLibraries[0]]);
      mockAdapter.liveDocsSearch.mockResolvedValue(mockLiveDocsMatches);

      render(<TmPanel />);
      fireEvent.click(screen.getByText("LiveDocs"));

      await waitFor(() => {
        expect(screen.getByText("Hello world")).toBeInTheDocument();
      });

      const matchItems = document.querySelectorAll(".tm-match");
      fireEvent.click(matchItems[0]);

      const { project } = useProjectStore.getState();
      expect(project!.segments[0].target).toBe("Hello world");
      expect(project!.segments[0].status).toBe("draft");
    });

    it("검색 중에 '검색 중...' 로딩 상태가 표시된다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      let resolveSearch: (value: LiveDocsMatch[]) => void;
      mockAdapter.liveDocsListLibraries.mockResolvedValue([mockLibraries[0]]);
      mockAdapter.liveDocsSearch.mockReturnValue(
        new Promise<LiveDocsMatch[]>((resolve) => { resolveSearch = resolve; })
      );

      render(<TmPanel />);
      fireEvent.click(screen.getByText("LiveDocs"));

      await waitFor(() => {
        expect(screen.getByText("검색 중...")).toBeInTheDocument();
      });

      resolveSearch!([]);
    });

    it("라이브러리가 없을 때 'LiveDocs 매치 없음' 메시지가 표시된다", async () => {
      useProjectStore.setState({ project: mockProject, currentSegmentIndex: 0 });
      mockAdapter.liveDocsListLibraries.mockResolvedValue([]);

      render(<TmPanel />);
      fireEvent.click(screen.getByText("LiveDocs"));

      await waitFor(() => {
        expect(screen.getByText("LiveDocs 매치 없음")).toBeInTheDocument();
      });
    });
  });

  describe("TM 추가 (+ TM 버튼)", () => {
    it("target이 있는 세그먼트에 + TM 버튼이 표시된다", () => {
      const projectWithTarget: Project = {
        ...mockProject,
        segments: [
          { ...mockProject.segments[0], target: "안녕하세요" },
        ],
      };
      useProjectStore.setState({ project: projectWithTarget, currentSegmentIndex: 0 });
      useTmStore.setState({ activeTmId: "tm-id-1" });

      render(<TmPanel />);
      expect(screen.getByTitle("현재 세그먼트를 TM에 추가")).toBeInTheDocument();
    });

    it("+ TM 버튼 클릭 시 addToTm이 호출된다", async () => {
      const projectWithTarget: Project = {
        ...mockProject,
        segments: [
          { ...mockProject.segments[0], target: "안녕하세요" },
        ],
      };
      useProjectStore.setState({ project: projectWithTarget, currentSegmentIndex: 0 });
      useTmStore.setState({ activeTmId: "tm-id-1" });
      mockAdapter.addToTm.mockResolvedValueOnce({
        id: "entry-1",
        source: "Hello",
        target: "안녕하세요",
        sourceLang: "en-US",
        targetLang: "ko-KR",
        createdAt: "2026-01-01T00:00:00Z",
        metadata: {},
      });
      mockAdapter.searchTm.mockResolvedValue([]);

      render(<TmPanel />);
      fireEvent.click(screen.getByTitle("현재 세그먼트를 TM에 추가"));

      await waitFor(() => {
        expect(mockAdapter.addToTm).toHaveBeenCalledWith(
          "tm-id-1",
          "Hello",
          "안녕하세요",
          "en-US",
          "ko-KR"
        );
      });
    });
  });
});
