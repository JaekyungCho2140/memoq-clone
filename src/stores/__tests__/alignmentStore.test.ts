// @vitest-environment jsdom
/**
 * Alignment store unit tests
 */

import { describe, it, expect, beforeEach } from "vitest";
import { useAlignmentStore } from "../alignmentStore";
import type { AlignedPair } from "../../types";

// ── Helpers ──────────────────────────────────────────────────────────────────

function resetStore() {
  useAlignmentStore.setState({
    phase: "upload",
    sourceLang: "en",
    targetLang: "ko",
    tmId: null,
    pairs: [],
    progress: 0,
    error: null,
  });
}

function makePair(overrides: Partial<AlignedPair> = {}): AlignedPair {
  return {
    id: "p1",
    source: "Hello world",
    target: "안녕 세계",
    score: 0.95,
    confirmed: false,
    modified: false,
    ...overrides,
  };
}

// ── Tests ─────────────────────────────────────────────────────────────────────

describe("alignmentStore", () => {
  beforeEach(() => {
    resetStore();
  });

  it("initial state is upload phase with empty pairs", () => {
    const s = useAlignmentStore.getState();
    expect(s.phase).toBe("upload");
    expect(s.pairs).toHaveLength(0);
    expect(s.tmId).toBeNull();
    expect(s.progress).toBe(0);
    expect(s.error).toBeNull();
  });

  it("setPhase updates the phase", () => {
    useAlignmentStore.getState().setPhase("processing");
    expect(useAlignmentStore.getState().phase).toBe("processing");

    useAlignmentStore.getState().setPhase("review");
    expect(useAlignmentStore.getState().phase).toBe("review");
  });

  it("setConfig updates sourceLang, targetLang, tmId", () => {
    useAlignmentStore.getState().setConfig("ja", "ko", "tm-123");
    const s = useAlignmentStore.getState();
    expect(s.sourceLang).toBe("ja");
    expect(s.targetLang).toBe("ko");
    expect(s.tmId).toBe("tm-123");
  });

  it("setPairs replaces pair list", () => {
    const pairs = [makePair({ id: "p1" }), makePair({ id: "p2" })];
    useAlignmentStore.getState().setPairs(pairs);
    expect(useAlignmentStore.getState().pairs).toHaveLength(2);
  });

  it("setProgress updates progress", () => {
    useAlignmentStore.getState().setProgress(42);
    expect(useAlignmentStore.getState().progress).toBe(42);
  });

  it("setError updates error", () => {
    useAlignmentStore.getState().setError("Something went wrong");
    expect(useAlignmentStore.getState().error).toBe("Something went wrong");
  });

  it("confirmPair sets confirmed=true on the matching pair only", () => {
    const pairs = [
      makePair({ id: "p1", confirmed: false }),
      makePair({ id: "p2", confirmed: false }),
    ];
    useAlignmentStore.getState().setPairs(pairs);
    useAlignmentStore.getState().confirmPair("p1");

    const { pairs: updated } = useAlignmentStore.getState();
    expect(updated.find((p) => p.id === "p1")?.confirmed).toBe(true);
    expect(updated.find((p) => p.id === "p2")?.confirmed).toBe(false);
  });

  it("unconfirmPair sets confirmed=false on the matching pair", () => {
    const pairs = [makePair({ id: "p1", confirmed: true })];
    useAlignmentStore.getState().setPairs(pairs);
    useAlignmentStore.getState().unconfirmPair("p1");

    expect(useAlignmentStore.getState().pairs[0].confirmed).toBe(false);
  });

  it("deletePair removes the pair from the list", () => {
    const pairs = [
      makePair({ id: "p1" }),
      makePair({ id: "p2" }),
      makePair({ id: "p3" }),
    ];
    useAlignmentStore.getState().setPairs(pairs);
    useAlignmentStore.getState().deletePair("p2");

    const { pairs: updated } = useAlignmentStore.getState();
    expect(updated).toHaveLength(2);
    expect(updated.find((p) => p.id === "p2")).toBeUndefined();
  });

  it("editPair updates source, target, and sets modified=true", () => {
    const pairs = [makePair({ id: "p1", source: "old source", target: "old target" })];
    useAlignmentStore.getState().setPairs(pairs);
    useAlignmentStore.getState().editPair("p1", "new source", "new target");

    const updated = useAlignmentStore.getState().pairs[0];
    expect(updated.source).toBe("new source");
    expect(updated.target).toBe("new target");
    expect(updated.modified).toBe(true);
  });

  it("editPair does not affect other pairs", () => {
    const pairs = [
      makePair({ id: "p1", source: "s1" }),
      makePair({ id: "p2", source: "s2" }),
    ];
    useAlignmentStore.getState().setPairs(pairs);
    useAlignmentStore.getState().editPair("p1", "new s1", "new t1");

    const p2 = useAlignmentStore.getState().pairs.find((p) => p.id === "p2")!;
    expect(p2.source).toBe("s2");
    expect(p2.modified).toBe(false);
  });

  it("confirmAll sets all pairs to confirmed=true", () => {
    const pairs = [
      makePair({ id: "p1", confirmed: false }),
      makePair({ id: "p2", confirmed: false }),
      makePair({ id: "p3", confirmed: true }),
    ];
    useAlignmentStore.getState().setPairs(pairs);
    useAlignmentStore.getState().confirmAll();

    const { pairs: updated } = useAlignmentStore.getState();
    expect(updated.every((p) => p.confirmed)).toBe(true);
  });

  it("reset returns store to initial state", () => {
    useAlignmentStore.getState().setPhase("review");
    useAlignmentStore.getState().setProgress(80);
    useAlignmentStore.getState().setPairs([makePair()]);
    useAlignmentStore.getState().setError("oops");
    useAlignmentStore.getState().setConfig("ja", "zh", "tm-abc");

    useAlignmentStore.getState().reset();

    const s = useAlignmentStore.getState();
    expect(s.phase).toBe("upload");
    expect(s.progress).toBe(0);
    expect(s.pairs).toHaveLength(0);
    expect(s.error).toBeNull();
    expect(s.tmId).toBeNull();
    expect(s.sourceLang).toBe("en");
    expect(s.targetLang).toBe("ko");
  });

  it("confirmPair is idempotent", () => {
    const pairs = [makePair({ id: "p1", confirmed: false })];
    useAlignmentStore.getState().setPairs(pairs);
    useAlignmentStore.getState().confirmPair("p1");
    useAlignmentStore.getState().confirmPair("p1");

    expect(useAlignmentStore.getState().pairs[0].confirmed).toBe(true);
  });

  it("deletePair on non-existent id leaves list unchanged", () => {
    const pairs = [makePair({ id: "p1" })];
    useAlignmentStore.getState().setPairs(pairs);
    useAlignmentStore.getState().deletePair("non-existent");
    expect(useAlignmentStore.getState().pairs).toHaveLength(1);
  });
});
