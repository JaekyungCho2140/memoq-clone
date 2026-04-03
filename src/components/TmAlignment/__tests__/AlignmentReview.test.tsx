// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { AlignmentReview } from "../AlignmentReview";
import { AlignmentProgress } from "../AlignmentProgress";
import { TmAlignmentPage } from "../TmAlignmentPage";
import { useAlignmentStore } from "../../../stores/alignmentStore";
import type { AlignedPair } from "../../../types";

// ── Mocks ─────────────────────────────────────────────────────────────────────

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../../../adapters", () => ({
  adapter: {
    alignmentConfirm: vi.fn().mockResolvedValue(undefined),
    createTm: vi.fn().mockResolvedValue("tm-mock"),
    alignmentAlign: vi.fn().mockResolvedValue({ pairs: [] }),
  },
  isTauri: vi.fn(() => false),
  registerFile: vi.fn(() => "web-file://mock"),
}));

// ── Helpers ──────────────────────────────────────────────────────────────────

function makePair(overrides: Partial<AlignedPair> = {}): AlignedPair {
  return {
    id: "p1",
    source: "Hello world",
    target: "안녕 세계",
    score: 0.9,
    confirmed: false,
    modified: false,
    ...overrides,
  };
}

function resetStore(pairs: AlignedPair[] = [], phase: "review" | "upload" | "processing" | "saving" | "done" = "review") {
  useAlignmentStore.setState({
    phase,
    sourceLang: "en",
    targetLang: "ko",
    tmId: "tm-test",
    pairs,
    progress: 0,
    error: null,
  });
}

// ── AlignmentReview ───────────────────────────────────────────────────────────

describe("AlignmentReview", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders empty state when no pairs", () => {
    resetStore([]);
    render(<AlignmentReview />);
    expect(screen.getByText("정렬 결과가 없습니다.")).toBeInTheDocument();
  });

  it("renders pair list with source and target text", () => {
    resetStore([makePair({ id: "p1", source: "Hello", target: "안녕" })]);
    render(<AlignmentReview />);
    expect(screen.getByText("Hello")).toBeInTheDocument();
    expect(screen.getByText("안녕")).toBeInTheDocument();
  });

  it("shows confidence score as percentage", () => {
    resetStore([makePair({ score: 0.87 })]);
    render(<AlignmentReview />);
    expect(screen.getByText("87%")).toBeInTheDocument();
  });

  it("shows confirmed count in save button", () => {
    resetStore([
      makePair({ id: "p1", confirmed: true }),
      makePair({ id: "p2", confirmed: false }),
    ]);
    render(<AlignmentReview />);
    expect(screen.getByText("TM에 저장 (1)")).toBeInTheDocument();
  });

  it("clicking 확정 button confirms the pair", () => {
    resetStore([makePair({ id: "p1", confirmed: false })]);
    render(<AlignmentReview />);
    fireEvent.click(screen.getByTitle("TM에 저장"));
    expect(useAlignmentStore.getState().pairs[0].confirmed).toBe(true);
  });

  it("confirmed pair shows 확정됨 button", () => {
    resetStore([makePair({ id: "p1", confirmed: true })]);
    render(<AlignmentReview />);
    expect(screen.getByText("확정됨")).toBeInTheDocument();
  });

  it("clicking 확정됨 unconfirms the pair", () => {
    resetStore([makePair({ id: "p1", confirmed: true })]);
    render(<AlignmentReview />);
    fireEvent.click(screen.getByText("확정됨"));
    expect(useAlignmentStore.getState().pairs[0].confirmed).toBe(false);
  });

  it("clicking 삭제 removes the pair from the list", () => {
    resetStore([
      makePair({ id: "p1", source: "First" }),
      makePair({ id: "p2", source: "Second" }),
    ]);
    render(<AlignmentReview />);
    const deleteButtons = screen.getAllByTitle("삭제");
    fireEvent.click(deleteButtons[0]);
    expect(useAlignmentStore.getState().pairs).toHaveLength(1);
    expect(useAlignmentStore.getState().pairs[0].id).toBe("p2");
  });

  it("전체 확정 button confirms all pairs", () => {
    resetStore([
      makePair({ id: "p1", confirmed: false }),
      makePair({ id: "p2", confirmed: false }),
    ]);
    render(<AlignmentReview />);
    fireEvent.click(screen.getByText("전체 확정"));
    const { pairs } = useAlignmentStore.getState();
    expect(pairs.every((p) => p.confirmed)).toBe(true);
  });

  it("TM 저장 button is disabled when no confirmed pairs", () => {
    resetStore([makePair({ confirmed: false })]);
    render(<AlignmentReview />);
    const saveBtn = screen.getByText("TM에 저장 (0)");
    expect(saveBtn).toBeDisabled();
  });

  it("clicking 편집 shows text areas for editing", () => {
    resetStore([makePair({ id: "p1", source: "Hello", target: "안녕" })]);
    render(<AlignmentReview />);
    fireEvent.click(screen.getByTitle("편집"));
    expect(screen.getByDisplayValue("Hello")).toBeInTheDocument();
    expect(screen.getByDisplayValue("안녕")).toBeInTheDocument();
  });

  it("saving edit updates pair and sets modified=true", () => {
    resetStore([makePair({ id: "p1", source: "Hello", target: "안녕" })]);
    render(<AlignmentReview />);
    fireEvent.click(screen.getByTitle("편집"));
    const sourceTA = screen.getByDisplayValue("Hello");
    fireEvent.change(sourceTA, { target: { value: "Hi" } });
    fireEvent.click(screen.getByTitle("저장"));
    const updated = useAlignmentStore.getState().pairs[0];
    expect(updated.source).toBe("Hi");
    expect(updated.modified).toBe(true);
  });

  it("cancelling edit restores original text", () => {
    resetStore([makePair({ id: "p1", source: "Hello", target: "안녕" })]);
    render(<AlignmentReview />);
    fireEvent.click(screen.getByTitle("편집"));
    const sourceTA = screen.getByDisplayValue("Hello");
    fireEvent.change(sourceTA, { target: { value: "changed" } });
    fireEvent.click(screen.getByTitle("취소"));
    expect(screen.getByText("Hello")).toBeInTheDocument();
    expect(useAlignmentStore.getState().pairs[0].source).toBe("Hello");
  });

  it("다시 시작 button calls reset", () => {
    resetStore([makePair()]);
    render(<AlignmentReview />);
    fireEvent.click(screen.getByText("다시 시작"));
    expect(useAlignmentStore.getState().phase).toBe("upload");
    expect(useAlignmentStore.getState().pairs).toHaveLength(0);
  });
});

// ── AlignmentProgress ─────────────────────────────────────────────────────────

describe("AlignmentProgress", () => {
  it("renders progress percentage", () => {
    useAlignmentStore.setState({ progress: 45, phase: "processing", sourceLang: "en", targetLang: "ko", tmId: null, pairs: [], error: null });
    render(<AlignmentProgress />);
    expect(screen.getByText("45%")).toBeInTheDocument();
  });

  it("renders progress bar with correct width style", () => {
    useAlignmentStore.setState({ progress: 70, phase: "processing", sourceLang: "en", targetLang: "ko", tmId: null, pairs: [], error: null });
    render(<AlignmentProgress />);
    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveStyle({ width: "70%" });
    expect(bar).toHaveAttribute("aria-valuenow", "70");
    expect(bar).toHaveAttribute("aria-valuemin", "0");
    expect(bar).toHaveAttribute("aria-valuemax", "100");
  });
});

// ── TmAlignmentPage ───────────────────────────────────────────────────────────

describe("TmAlignmentPage", () => {
  beforeEach(() => {
    useAlignmentStore.setState({
      phase: "upload",
      sourceLang: "en",
      targetLang: "ko",
      tmId: null,
      pairs: [],
      progress: 0,
      error: null,
    });
  });

  it("renders upload phase initially", () => {
    render(<TmAlignmentPage onClose={vi.fn()} />);
    expect(screen.getByText("TM 정렬 — 파일 업로드")).toBeInTheDocument();
  });

  it("renders review phase when phase=review", () => {
    useAlignmentStore.setState({ phase: "review", pairs: [makePair()], sourceLang: "en", targetLang: "ko", tmId: "tm1", progress: 100, error: null });
    render(<TmAlignmentPage onClose={vi.fn()} />);
    expect(screen.getByText("정렬 결과 검토")).toBeInTheDocument();
  });

  it("renders done phase with success message", () => {
    useAlignmentStore.setState({ phase: "done", pairs: [], sourceLang: "en", targetLang: "ko", tmId: "tm1", progress: 100, error: null });
    render(<TmAlignmentPage onClose={vi.fn()} />);
    expect(screen.getByText("TM 저장 완료!")).toBeInTheDocument();
  });

  it("clicking 닫기 calls onClose and resets store", () => {
    const onClose = vi.fn();
    render(<TmAlignmentPage onClose={onClose} />);
    fireEvent.click(screen.getByText("닫기"));
    expect(onClose).toHaveBeenCalledOnce();
    expect(useAlignmentStore.getState().phase).toBe("upload");
  });

  it("phase indicator marks upload as active initially", () => {
    render(<TmAlignmentPage onClose={vi.fn()} />);
    const stepOne = screen.getByText("1. 파일 업로드");
    expect(stepOne).toHaveClass("active");
  });
});
