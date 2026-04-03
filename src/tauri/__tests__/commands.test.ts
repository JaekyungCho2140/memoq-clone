import { describe, it, expect, vi, beforeEach } from "vitest";

// Tauri API 모킹 (invoke 함수)
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  parseFile,
  searchTm,
  lookupTb,
  exportFile,
  saveSegment,
  createTm,
  addToTm,
  createTb,
  addToTb,
} from "../commands";
import type { Project, TmMatch, TbEntry, Segment } from "../../types";

const mockProject: Project = {
  id: "proj-1",
  name: "Test",
  sourcePath: "/tmp/test.xliff",
  sourceLang: "en-US",
  targetLang: "ko-KR",
  createdAt: "2026-01-01T00:00:00Z",
  segments: [],
};

const mockSegment: Segment = {
  id: "seg-1",
  source: "Hello",
  target: "안녕",
  status: "draft",
  tmMatches: [],
  order: 0,
};

const mockTmMatches: TmMatch[] = [
  { source: "Hello", target: "안녕하세요", score: 1.0, matchType: "exact" },
];

const mockTbEntries: TbEntry[] = [
  {
    id: "tb-1",
    sourceTerm: "Hello",
    targetTerm: "안녕하세요",
    sourceLang: "en-US",
    targetLang: "ko-KR",
    notes: "",
    forbidden: false,
  },
];

describe("Tauri commands", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("parseFile()", () => {
    it("parse_file 커맨드를 올바른 파라미터로 invoke한다", async () => {
      vi.mocked(invoke).mockResolvedValueOnce(mockProject);

      const result = await parseFile("/tmp/test.xliff");

      expect(invoke).toHaveBeenCalledWith("parse_file", { path: "/tmp/test.xliff" });
      expect(result).toEqual(mockProject);
    });

    it("invoke가 reject되면 에러를 전파한다", async () => {
      const error = new Error("파일을 파싱할 수 없습니다");
      vi.mocked(invoke).mockRejectedValueOnce(error);

      await expect(parseFile("/invalid/path.txt")).rejects.toThrow("파일을 파싱할 수 없습니다");
    });

    it("빈 경로로 호출 시에도 invoke가 실행된다", async () => {
      vi.mocked(invoke).mockResolvedValueOnce(mockProject);

      await parseFile("");

      expect(invoke).toHaveBeenCalledWith("parse_file", { path: "" });
    });
  });

  describe("searchTm()", () => {
    it("tm_search 커맨드를 올바른 파라미터로 invoke한다", async () => {
      vi.mocked(invoke).mockResolvedValueOnce(mockTmMatches);

      const params = {
        tmId: "tm-1",
        query: "Hello",
        sourceLang: "en-US",
        targetLang: "ko-KR",
        minScore: 0.5,
      };
      const result = await searchTm(params);

      expect(invoke).toHaveBeenCalledWith("tm_search", expect.objectContaining({
        tmId: "tm-1",
        query: "Hello",
        sourceLang: "en-US",
        targetLang: "ko-KR",
        minScore: 0.5,
      }));
      expect(result).toEqual(mockTmMatches);
    });

    it("모든 TmSearchParams 필드가 전달된다", async () => {
      vi.mocked(invoke).mockResolvedValueOnce([]);

      await searchTm({
        tmId: "tm-abc",
        query: "test query",
        sourceLang: "ja-JP",
        targetLang: "ko-KR",
        minScore: 0.8,
      });

      const callArgs = vi.mocked(invoke).mock.calls[0];
      expect(callArgs[0]).toBe("tm_search");
      const passedParams = callArgs[1] as Record<string, unknown>;
      expect(passedParams["tmId"]).toBe("tm-abc");
      expect(passedParams["query"]).toBe("test query");
      expect(passedParams["minScore"]).toBe(0.8);
    });

    it("invoke가 reject되면 에러를 전파한다", async () => {
      const error = new Error("TM 검색 실패");
      vi.mocked(invoke).mockRejectedValueOnce(error);

      await expect(
        searchTm({ tmId: "tm-1", query: "Hello", sourceLang: "en-US", targetLang: "ko-KR", minScore: 0.5 })
      ).rejects.toThrow("TM 검색 실패");
    });

    it("결과가 없을 때 빈 배열을 반환한다", async () => {
      vi.mocked(invoke).mockResolvedValueOnce([]);

      const result = await searchTm({
        tmId: "tm-1",
        query: "unknown",
        sourceLang: "en-US",
        targetLang: "ko-KR",
        minScore: 0.5,
      });

      expect(result).toEqual([]);
    });
  });

  describe("lookupTb()", () => {
    it("tb_lookup 커맨드를 올바른 파라미터로 invoke한다", async () => {
      vi.mocked(invoke).mockResolvedValueOnce(mockTbEntries);

      const params = {
        tbId: "tb-1",
        term: "Hello",
        sourceLang: "en-US",
      };
      const result = await lookupTb(params);

      expect(invoke).toHaveBeenCalledWith("tb_lookup", expect.objectContaining({
        tbId: "tb-1",
        term: "Hello",
        sourceLang: "en-US",
      }));
      expect(result).toEqual(mockTbEntries);
    });

    it("모든 TbLookupParams 필드가 전달된다", async () => {
      vi.mocked(invoke).mockResolvedValueOnce([]);

      await lookupTb({
        tbId: "tb-xyz",
        term: "테스트 용어",
        sourceLang: "ko-KR",
      });

      const callArgs = vi.mocked(invoke).mock.calls[0];
      expect(callArgs[0]).toBe("tb_lookup");
      const passedParams = callArgs[1] as Record<string, unknown>;
      expect(passedParams["tbId"]).toBe("tb-xyz");
      expect(passedParams["term"]).toBe("테스트 용어");
      expect(passedParams["sourceLang"]).toBe("ko-KR");
    });

    it("invoke가 reject되면 에러를 전파한다", async () => {
      const error = new Error("TB 조회 실패");
      vi.mocked(invoke).mockRejectedValueOnce(error);

      await expect(
        lookupTb({ tbId: "tb-1", term: "Hello", sourceLang: "en-US" })
      ).rejects.toThrow("TB 조회 실패");
    });

    it("금지 용어를 포함한 결과를 반환한다", async () => {
      const entriesWithForbidden: TbEntry[] = [
        { ...mockTbEntries[0] },
        {
          id: "tb-forbidden",
          sourceTerm: "Ban",
          targetTerm: "금지",
          sourceLang: "en-US",
          targetLang: "ko-KR",
          notes: "사용 금지",
          forbidden: true,
        },
      ];
      vi.mocked(invoke).mockResolvedValueOnce(entriesWithForbidden);

      const result = await lookupTb({ tbId: "tb-1", term: "Ban", sourceLang: "en-US" });

      expect(result).toHaveLength(2);
      expect(result[1].forbidden).toBe(true);
    });
  });

  describe("에러 핸들링", () => {
    it("parseFile — invoke가 string 에러를 reject하면 전파된다", async () => {
      vi.mocked(invoke).mockRejectedValueOnce("Invalid file format");

      await expect(parseFile("/tmp/broken.docx")).rejects.toBe("Invalid file format");
    });

    it("searchTm — invoke가 string 에러를 reject하면 전파된다", async () => {
      vi.mocked(invoke).mockRejectedValueOnce("TM not found");

      await expect(
        searchTm({ tmId: "nonexistent", query: "Hello", sourceLang: "en-US", targetLang: "ko-KR", minScore: 0.5 })
      ).rejects.toBe("TM not found");
    });

    it("lookupTb — invoke가 string 에러를 reject하면 전파된다", async () => {
      vi.mocked(invoke).mockRejectedValueOnce("TB not found");

      await expect(
        lookupTb({ tbId: "nonexistent", term: "Hello", sourceLang: "en-US" })
      ).rejects.toBe("TB not found");
    });
  });

  describe("기타 커맨드", () => {
    it("exportFile — export_file 커맨드를 invoke한다", async () => {
      vi.mocked(invoke).mockResolvedValueOnce(undefined);

      await exportFile([mockSegment], "/tmp/source.xliff", "/tmp/output.xliff");

      expect(invoke).toHaveBeenCalledWith("export_file", {
        segments: [mockSegment],
        sourcePath: "/tmp/source.xliff",
        outputPath: "/tmp/output.xliff",
      });
    });

    it("exportFile — 빈 세그먼트 배열로 호출할 수 있다", async () => {
      vi.mocked(invoke).mockResolvedValueOnce(undefined);

      await exportFile([], "/tmp/source.xliff", "/tmp/output.xliff");

      expect(invoke).toHaveBeenCalledWith("export_file", {
        segments: [],
        sourcePath: "/tmp/source.xliff",
        outputPath: "/tmp/output.xliff",
      });
    });

    it("saveSegment — save_segment 커맨드를 올바른 파라미터로 invoke한다", async () => {
      vi.mocked(invoke).mockResolvedValueOnce(mockSegment);

      const result = await saveSegment("proj-1", "seg-1", "Hello", "안녕", "draft", 0);

      expect(invoke).toHaveBeenCalledWith("save_segment", {
        projectId: "proj-1",
        segmentId: "seg-1",
        source: "Hello",
        target: "안녕",
        status: "draft",
        order: 0,
      });
      expect(result).toEqual(mockSegment);
    });

    it("saveSegment — invoke reject 시 에러를 전파한다", async () => {
      vi.mocked(invoke).mockRejectedValueOnce(new Error("세그먼트 저장 실패"));

      await expect(
        saveSegment("proj-1", "seg-1", "Hello", "안녕", "confirmed", 0)
      ).rejects.toThrow("세그먼트 저장 실패");
    });

    it("createTm — tm_create 커맨드를 invoke하고 TM ID를 반환한다", async () => {
      vi.mocked(invoke).mockResolvedValueOnce("new-tm-id");

      const result = await createTm("My TM", "en-US", "ko-KR");

      expect(invoke).toHaveBeenCalledWith("tm_create", {
        name: "My TM",
        sourceLang: "en-US",
        targetLang: "ko-KR",
      });
      expect(result).toBe("new-tm-id");
    });

    it("addToTm — tm_add 커맨드를 올바른 파라미터로 invoke한다", async () => {
      const mockEntry = {
        id: "entry-1",
        source: "Hello",
        target: "안녕하세요",
        sourceLang: "en-US",
        targetLang: "ko-KR",
        createdAt: "2026-01-01T00:00:00Z",
        metadata: {},
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockEntry);

      const result = await addToTm("tm-1", "Hello", "안녕하세요", "en-US", "ko-KR");

      expect(invoke).toHaveBeenCalledWith("tm_add", {
        tmId: "tm-1",
        source: "Hello",
        target: "안녕하세요",
        sourceLang: "en-US",
        targetLang: "ko-KR",
      });
      expect(result).toEqual(mockEntry);
    });

    it("addToTm — invoke reject 시 에러를 전파한다", async () => {
      vi.mocked(invoke).mockRejectedValueOnce(new Error("TM 추가 실패"));

      await expect(
        addToTm("tm-1", "Hello", "안녕하세요", "en-US", "ko-KR")
      ).rejects.toThrow("TM 추가 실패");
    });

    it("createTb — tb_create 커맨드를 invoke하고 TB ID를 반환한다", async () => {
      vi.mocked(invoke).mockResolvedValueOnce("new-tb-id");

      const result = await createTb("My TB");

      expect(invoke).toHaveBeenCalledWith("tb_create", { name: "My TB" });
      expect(result).toBe("new-tb-id");
    });

    it("addToTb — tb_add 커맨드를 올바른 파라미터로 invoke한다", async () => {
      const mockTbEntry: TbEntry = {
        id: "tb-entry-new",
        sourceTerm: "Hello",
        targetTerm: "안녕하세요",
        sourceLang: "en-US",
        targetLang: "ko-KR",
        notes: "인사말",
        forbidden: false,
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockTbEntry);

      const result = await addToTb("tb-1", "Hello", "안녕하세요", "en-US", "ko-KR", "인사말", false);

      expect(invoke).toHaveBeenCalledWith("tb_add", {
        tbId: "tb-1",
        sourceTerm: "Hello",
        targetTerm: "안녕하세요",
        sourceLang: "en-US",
        targetLang: "ko-KR",
        notes: "인사말",
        forbidden: false,
      });
      expect(result).toEqual(mockTbEntry);
    });

    it("addToTb — forbidden=true 용어 추가를 지원한다", async () => {
      const mockForbiddenEntry: TbEntry = {
        id: "tb-forbidden",
        sourceTerm: "Ban",
        targetTerm: "금지",
        sourceLang: "en-US",
        targetLang: "ko-KR",
        notes: "",
        forbidden: true,
      };
      vi.mocked(invoke).mockResolvedValueOnce(mockForbiddenEntry);

      const result = await addToTb("tb-1", "Ban", "금지", "en-US", "ko-KR", "", true);

      expect(invoke).toHaveBeenCalledWith("tb_add", expect.objectContaining({
        forbidden: true,
      }));
      expect(result.forbidden).toBe(true);
    });
  });
});
