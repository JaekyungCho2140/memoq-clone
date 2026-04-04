// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom";
import { VendorDashboard } from "../VendorDashboard";
import { VendorDeliveryModal } from "../VendorDeliveryModal";
import { AdminVendorDashboard } from "../AdminVendorDashboard";
import { useVendorStore } from "../../../stores/vendorStore";
import { useAuthStore } from "../../../stores/authStore";
import type { VendorAssignment, VendorInfo } from "../../../types";

// ── Mocks ─────────────────────────────────────────────────────────────────────

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../../../adapters", () => ({
  adapter: {},
  isTauri: vi.fn(() => false),
}));

// Mock fetch functions as no-ops so useEffect doesn't overwrite test state
vi.mock("../../../stores/vendorStore", async (importOriginal) => {
  const mod = await importOriginal<typeof import("../../../stores/vendorStore")>();
  return mod;
});

// ── Helpers ──────────────────────────────────────────────────────────────────

function setupVendorAuth() {
  useAuthStore.setState({
    user: { id: "u1", username: "vendor_user", role: "vendor" },
    accessToken: "test_token",
    refreshToken: null,
    isLoading: false,
    error: null,
  });
}

function resetStores(preloadedAssignments: VendorAssignment[] = [], isLoading = false) {
  setupVendorAuth();
  useVendorStore.setState({
    myAssignments: preloadedAssignments,
    allAssignments: [],
    vendors: [],
    activeAssignment: null,
    isLoading,
    error: null,
    // Override fetch functions so useEffect doesn't reload mock data
    fetchMyAssignments: vi.fn().mockResolvedValue(undefined),
    fetchAllAssignments: vi.fn().mockResolvedValue(undefined),
    fetchVendors: vi.fn().mockResolvedValue(undefined),
  });
}

const makeAssignment = (overrides: Partial<VendorAssignment> = {}): VendorAssignment => ({
  id: "a1",
  projectId: "p1",
  projectName: "Test Project",
  fileName: "doc.xliff",
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

const makeVendor = (overrides: Partial<VendorInfo> = {}): VendorInfo => ({
  id: "v1",
  username: "trans_user",
  displayName: "테스트번역가",
  email: "trans@example.com",
  langPairs: ["en→ko"],
  activeAssignments: 1,
  totalDelivered: 5,
  ...overrides,
});

// ── VendorDashboard ───────────────────────────────────────────────────────────

describe("VendorDashboard", () => {
  beforeEach(() => resetStores());

  it("shows loading state", () => {
    resetStores([], true);
    render(<VendorDashboard />);
    expect(screen.getByText("불러오는 중...")).toBeInTheDocument();
  });

  it("shows empty message when no assignments", async () => {
    resetStores([]);
    render(<VendorDashboard />);
    await waitFor(() => {
      expect(screen.getByText("할당된 작업이 없습니다.")).toBeInTheDocument();
    });
  });

  it("renders assignments with file and project name", () => {
    const a = makeAssignment({ projectName: "Annual Report", fileName: "chapter1.xliff" });
    resetStores([a]);
    render(<VendorDashboard />);
    expect(screen.getByText("Annual Report")).toBeInTheDocument();
    expect(screen.getByText("chapter1.xliff")).toBeInTheDocument();
  });

  it("shows language pair", () => {
    const a = makeAssignment({ sourceLang: "en", targetLang: "ko" });
    resetStores([a]);
    render(<VendorDashboard />);
    expect(screen.getByText("EN → KO")).toBeInTheDocument();
  });

  it("shows summary pending/in_progress/delivered counts", () => {
    resetStores([
      makeAssignment({ id: "a1", status: "pending" }),
      makeAssignment({ id: "a2", status: "in_progress" }),
      makeAssignment({ id: "a3", status: "delivered" }),
    ]);
    render(<VendorDashboard />);
    const numbers = document.querySelectorAll(".summary-number");
    expect(numbers[0].textContent).toBe("1"); // pending
    expect(numbers[1].textContent).toBe("1"); // in_progress
    expect(numbers[2].textContent).toBe("1"); // delivered
  });

  it("shows '편집 열기' button for in_progress assignments", () => {
    resetStores([makeAssignment({ status: "in_progress" })]);
    render(<VendorDashboard />);
    expect(screen.getByText("편집 열기")).toBeInTheDocument();
  });

  it("shows '편집 열기' for pending status", () => {
    resetStores([makeAssignment({ status: "pending" })]);
    render(<VendorDashboard />);
    expect(screen.getByText("편집 열기")).toBeInTheDocument();
  });

  it("does not show '편집 열기' for delivered assignments", () => {
    resetStores([makeAssignment({ status: "delivered" })]);
    render(<VendorDashboard />);
    expect(screen.queryByText("편집 열기")).not.toBeInTheDocument();
  });

  it("calls openAssignment when '편집 열기' is clicked", () => {
    const a = makeAssignment({ status: "in_progress" });
    const openSpy = vi.fn();
    resetStores([a]);
    useVendorStore.setState({ openAssignment: openSpy });
    render(<VendorDashboard />);
    fireEvent.click(screen.getByText("편집 열기"));
    expect(openSpy).toHaveBeenCalledWith(a);
  });

  it("marks overdue assignments with '기한 초과' badge", () => {
    const overdue = makeAssignment({
      status: "in_progress",
      deadline: "2000-01-01T00:00:00.000Z",
    });
    resetStores([overdue]);
    render(<VendorDashboard />);
    expect(screen.getByText("기한 초과")).toBeInTheDocument();
  });

  it("does not show overdue badge for delivered assignments past deadline", () => {
    const past = makeAssignment({
      status: "delivered",
      deadline: "2000-01-01T00:00:00.000Z",
    });
    resetStores([past]);
    render(<VendorDashboard />);
    expect(screen.queryByText("기한 초과")).not.toBeInTheDocument();
  });

  it("shows error alert when error is set", () => {
    resetStores([]);
    useVendorStore.setState({ error: "서버 오류 발생" });
    render(<VendorDashboard />);
    expect(screen.getByRole("alert")).toHaveTextContent("서버 오류 발생");
  });

  it("shows username in header", () => {
    resetStores([]);
    render(<VendorDashboard />);
    expect(screen.getByText("vendor_user")).toBeInTheDocument();
  });

  it("shows progress bar with correct aria attributes", () => {
    const a = makeAssignment({ progressPct: 75 });
    resetStores([a]);
    render(<VendorDashboard />);
    const bar = screen.getByRole("progressbar");
    expect(bar).toHaveAttribute("aria-valuenow", "75");
  });
});

// ── VendorDeliveryModal ───────────────────────────────────────────────────────

describe("VendorDeliveryModal", () => {
  beforeEach(() => {
    setupVendorAuth();
    useVendorStore.setState({
      myAssignments: [],
      allAssignments: [],
      vendors: [],
      activeAssignment: null,
      isLoading: false,
      error: null,
      deliver: vi.fn().mockResolvedValue(undefined),
      fetchMyAssignments: vi.fn().mockResolvedValue(undefined),
      fetchAllAssignments: vi.fn().mockResolvedValue(undefined),
      fetchVendors: vi.fn().mockResolvedValue(undefined),
    });
  });

  it("renders assignment info", () => {
    const a = makeAssignment({ projectName: "My Project", fileName: "doc.xliff" });
    render(<VendorDeliveryModal assignment={a} onCancel={vi.fn()} />);
    expect(screen.getByText("My Project")).toBeInTheDocument();
    expect(screen.getByText("doc.xliff")).toBeInTheDocument();
  });

  it("submit button is disabled until checkbox is checked", () => {
    render(<VendorDeliveryModal assignment={makeAssignment()} onCancel={vi.fn()} />);
    const submitBtn = screen.getByRole("button", { name: "납품 제출" });
    expect(submitBtn).toBeDisabled();
    fireEvent.click(screen.getByRole("checkbox"));
    expect(submitBtn).not.toBeDisabled();
  });

  it("shows incomplete warning when segments remain", () => {
    const a = makeAssignment({ totalSegments: 100, translatedSegments: 80 });
    render(<VendorDeliveryModal assignment={a} onCancel={vi.fn()} />);
    expect(screen.getByRole("alert")).toHaveTextContent("20개");
  });

  it("does not show warning when fully translated", () => {
    const a = makeAssignment({ totalSegments: 100, translatedSegments: 100 });
    render(<VendorDeliveryModal assignment={a} onCancel={vi.fn()} />);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("calls onCancel when cancel button clicked", () => {
    const onCancel = vi.fn();
    render(<VendorDeliveryModal assignment={makeAssignment()} onCancel={onCancel} />);
    fireEvent.click(screen.getByRole("button", { name: "취소" }));
    expect(onCancel).toHaveBeenCalled();
  });

  it("calls deliver on submit after confirming", async () => {
    const deliverMock = vi.fn().mockResolvedValueOnce(undefined);
    useVendorStore.setState({ deliver: deliverMock });
    const a = makeAssignment({ id: "a1" });
    render(<VendorDeliveryModal assignment={a} onCancel={vi.fn()} />);
    fireEvent.click(screen.getByRole("checkbox"));
    fireEvent.click(screen.getByRole("button", { name: "납품 제출" }));
    await waitFor(() => expect(deliverMock).toHaveBeenCalledWith("a1"));
  });

  it("shows 진행률 in delivery info", () => {
    const a = makeAssignment({ translatedSegments: 70, totalSegments: 100, progressPct: 70 });
    render(<VendorDeliveryModal assignment={a} onCancel={vi.fn()} />);
    expect(screen.getByText("70/100 (70%)")).toBeInTheDocument();
  });

  it("has accessible dialog role", () => {
    render(<VendorDeliveryModal assignment={makeAssignment()} onCancel={vi.fn()} />);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });
});

// ── AdminVendorDashboard ──────────────────────────────────────────────────────

describe("AdminVendorDashboard", () => {
  function resetAdmin(
    vendors: VendorInfo[] = [],
    allAssignments: VendorAssignment[] = [],
    isLoading = false,
    error: string | null = null,
  ) {
    setupVendorAuth();
    useVendorStore.setState({
      myAssignments: [],
      allAssignments,
      vendors,
      activeAssignment: null,
      isLoading,
      error,
      fetchMyAssignments: vi.fn().mockResolvedValue(undefined),
      fetchAllAssignments: vi.fn().mockResolvedValue(undefined),
      fetchVendors: vi.fn().mockResolvedValue(undefined),
    });
  }

  it("shows loading state", () => {
    resetAdmin([], [], true);
    render(<AdminVendorDashboard />);
    expect(screen.getByText("불러오는 중...")).toBeInTheDocument();
  });

  it("shows empty message when no vendors", async () => {
    resetAdmin([], [], false);
    render(<AdminVendorDashboard />);
    await waitFor(() => {
      expect(screen.getByText("등록된 벤더가 없습니다.")).toBeInTheDocument();
    });
  });

  it("renders vendor cards with display name", () => {
    const vendor = makeVendor({ displayName: "김번역사" });
    resetAdmin([vendor], [makeAssignment({ vendorId: "v1" })]);
    render(<AdminVendorDashboard />);
    expect(screen.getByText("김번역사")).toBeInTheDocument();
  });

  it("shows summary: total vendors, active, delivered", () => {
    const vendor = makeVendor();
    resetAdmin(
      [vendor],
      [
        makeAssignment({ id: "a1", status: "in_progress" }),
        makeAssignment({ id: "a2", status: "delivered" }),
        makeAssignment({ id: "a3", status: "pending" }),
      ],
    );
    render(<AdminVendorDashboard />);
    const numbers = document.querySelectorAll(".summary-number");
    expect(numbers[0].textContent).toBe("1"); // 1 vendor
    expect(numbers[1].textContent).toBe("2"); // in_progress + pending
    expect(numbers[2].textContent).toBe("1"); // delivered
  });

  it("expands vendor card to show assignment file on click", () => {
    const vendor = makeVendor();
    resetAdmin([vendor], [makeAssignment({ vendorId: "v1", fileName: "secret.xliff" })]);
    render(<AdminVendorDashboard />);
    // file should not be visible before expansion
    expect(screen.queryByText("secret.xliff")).not.toBeInTheDocument();
    const cardHeader = screen.getByRole("button");
    fireEvent.click(cardHeader);
    expect(screen.getByText("secret.xliff")).toBeInTheDocument();
  });

  it("shows error alert", () => {
    resetAdmin([], [], false, "데이터 로드 실패");
    render(<AdminVendorDashboard />);
    expect(screen.getByRole("alert")).toHaveTextContent("데이터 로드 실패");
  });

  it("shows '—' for deliveredAt when null", () => {
    const vendor = makeVendor();
    resetAdmin([vendor], [makeAssignment({ vendorId: "v1", deliveredAt: null })]);
    render(<AdminVendorDashboard />);
    const cardHeader = screen.getByRole("button");
    fireEvent.click(cardHeader);
    expect(screen.getAllByText("—").length).toBeGreaterThan(0);
  });
});
