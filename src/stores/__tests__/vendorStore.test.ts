// @vitest-environment jsdom
/**
 * Vendor store unit tests
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { useVendorStore } from "../vendorStore";
import type { VendorAssignment } from "../../types";

// ── Helpers ──────────────────────────────────────────────────────────────────

function resetStore() {
  useVendorStore.setState({
    myAssignments: [],
    allAssignments: [],
    vendors: [],
    activeAssignment: null,
    isLoading: false,
    error: null,
  });
}

const makeAssignment = (overrides: Partial<VendorAssignment> = {}): VendorAssignment => ({
  id: "a1",
  projectId: "p1",
  projectName: "Test Project",
  fileName: "file.xliff",
  sourceLang: "en",
  targetLang: "ko",
  deadline: "2099-01-01T00:00:00.000Z",
  status: "in_progress",
  progressPct: 50,
  totalSegments: 100,
  translatedSegments: 50,
  vendorId: "v1",
  vendorName: "테스트번역가",
  deliveredAt: null,
  rejectionNote: null,
  ...overrides,
});

// ── Tests ─────────────────────────────────────────────────────────────────────

describe("useVendorStore — openAssignment / closeAssignment", () => {
  beforeEach(resetStore);

  it("sets activeAssignment when opened", () => {
    const assignment = makeAssignment();
    useVendorStore.getState().openAssignment(assignment);
    expect(useVendorStore.getState().activeAssignment).toEqual(assignment);
  });

  it("clears activeAssignment when closed", () => {
    const assignment = makeAssignment();
    useVendorStore.getState().openAssignment(assignment);
    useVendorStore.getState().closeAssignment();
    expect(useVendorStore.getState().activeAssignment).toBeNull();
  });
});

describe("useVendorStore — updateProgress", () => {
  beforeEach(resetStore);

  it("updates progressPct and translatedSegments in myAssignments", () => {
    const a = makeAssignment({ id: "a1", totalSegments: 100, translatedSegments: 50, progressPct: 50 });
    useVendorStore.setState({ myAssignments: [a] });

    useVendorStore.getState().updateProgress("a1", 80);

    const updated = useVendorStore.getState().myAssignments[0];
    expect(updated.translatedSegments).toBe(80);
    expect(updated.progressPct).toBe(80);
  });

  it("also updates allAssignments", () => {
    const a = makeAssignment({ id: "a1", totalSegments: 100 });
    useVendorStore.setState({ myAssignments: [a], allAssignments: [a] });

    useVendorStore.getState().updateProgress("a1", 60);

    expect(useVendorStore.getState().allAssignments[0].progressPct).toBe(60);
  });

  it("updates activeAssignment translatedSegments when it matches", () => {
    const a = makeAssignment({ id: "a1", totalSegments: 100 });
    useVendorStore.setState({ myAssignments: [a], activeAssignment: a });

    useVendorStore.getState().updateProgress("a1", 75);

    expect(useVendorStore.getState().activeAssignment?.translatedSegments).toBe(75);
  });

  it("leaves activeAssignment unchanged when id does not match", () => {
    const a = makeAssignment({ id: "a1", totalSegments: 100 });
    const active = makeAssignment({ id: "a2" });
    useVendorStore.setState({ myAssignments: [a], activeAssignment: active });

    useVendorStore.getState().updateProgress("a1", 90);

    expect(useVendorStore.getState().activeAssignment?.id).toBe("a2");
  });

  it("clamps progressPct to 100 when translatedSegments equals totalSegments", () => {
    const a = makeAssignment({ id: "a1", totalSegments: 50 });
    useVendorStore.setState({ myAssignments: [a] });

    useVendorStore.getState().updateProgress("a1", 50);

    expect(useVendorStore.getState().myAssignments[0].progressPct).toBe(100);
  });

  it("sets progressPct to 0 for zero totalSegments without dividing by zero", () => {
    const a = makeAssignment({ id: "a1", totalSegments: 0 });
    useVendorStore.setState({ myAssignments: [a] });

    useVendorStore.getState().updateProgress("a1", 0);

    expect(useVendorStore.getState().myAssignments[0].progressPct).toBe(0);
  });
});

describe("useVendorStore — fetchMyAssignments (mock fallback)", () => {
  beforeEach(resetStore);

  it("loads mock assignments when apiBase is empty", async () => {
    // __WEB_API_BASE__ is not set in test environment → mock fallback
    await useVendorStore.getState().fetchMyAssignments();

    const { myAssignments, isLoading, error } = useVendorStore.getState();
    expect(myAssignments.length).toBeGreaterThan(0);
    expect(isLoading).toBe(false);
    expect(error).toBeNull();
  });
});

describe("useVendorStore — fetchAllAssignments (mock fallback)", () => {
  beforeEach(resetStore);

  it("loads mock all-assignments when apiBase is empty", async () => {
    await useVendorStore.getState().fetchAllAssignments();

    const { allAssignments, isLoading } = useVendorStore.getState();
    expect(allAssignments.length).toBeGreaterThan(0);
    expect(isLoading).toBe(false);
  });
});

describe("useVendorStore — fetchVendors (mock fallback)", () => {
  beforeEach(resetStore);

  it("loads mock vendors when apiBase is empty", async () => {
    await useVendorStore.getState().fetchVendors();

    const { vendors } = useVendorStore.getState();
    expect(vendors.length).toBeGreaterThan(0);
    expect(vendors[0]).toHaveProperty("displayName");
    expect(vendors[0]).toHaveProperty("langPairs");
  });
});

describe("useVendorStore — deliver (mock path)", () => {
  beforeEach(resetStore);

  it("marks assignment as delivered and clears activeAssignment", async () => {
    const a = makeAssignment({ id: "a1", status: "in_progress" });
    useVendorStore.setState({ myAssignments: [a], allAssignments: [a], activeAssignment: a });

    await useVendorStore.getState().deliver("a1");

    const { myAssignments, activeAssignment } = useVendorStore.getState();
    expect(myAssignments[0].status).toBe("delivered");
    expect(myAssignments[0].deliveredAt).not.toBeNull();
    expect(activeAssignment).toBeNull();
  });

  it("sets error and rethrows when fetch fails", async () => {
    // Simulate API base being set, so it tries to fetch and fails
    const w = window as unknown as Record<string, unknown>;
    const originalFetch = w.fetch as typeof fetch;
    w.fetch = vi.fn().mockResolvedValueOnce({ ok: false, status: 500 });
    // Set a fake API base so the fetch path is taken
    w.__WEB_API_BASE__ = "http://localhost:3000";

    const a = makeAssignment({ id: "a1" });
    useVendorStore.setState({ myAssignments: [a] });

    await expect(useVendorStore.getState().deliver("a1")).rejects.toThrow();
    expect(useVendorStore.getState().error).not.toBeNull();

    // cleanup
    delete w.__WEB_API_BASE__;
    w.fetch = originalFetch;
  });
});

describe("useVendorStore — setError", () => {
  beforeEach(resetStore);

  it("sets error message", () => {
    useVendorStore.getState().setError("테스트 에러");
    expect(useVendorStore.getState().error).toBe("테스트 에러");
  });

  it("clears error when null is passed", () => {
    useVendorStore.setState({ error: "이전 에러" });
    useVendorStore.getState().setError(null);
    expect(useVendorStore.getState().error).toBeNull();
  });
});
