/**
 * Vendor Store — manages vendor assignments and delivery workflow.
 *
 * In web mode (when AFR-44 API is available) real HTTP calls replace
 * the mock helpers below. The store interface stays identical so no
 * component changes are needed when the API is wired up.
 *
 * Mock data is used until the backend (AFR-44) is complete.
 */

import { create } from "zustand";
import type { VendorAssignment, VendorInfo } from "../types";

// ── Mock data ─────────────────────────────────────────────────────────────

const MOCK_ASSIGNMENTS: VendorAssignment[] = [
  {
    id: "a1",
    projectId: "p1",
    projectName: "Annual Report 2025",
    fileName: "annual_report_ch1.xliff",
    sourceLang: "en",
    targetLang: "ko",
    deadline: "2026-04-10T18:00:00.000Z",
    status: "in_progress",
    progressPct: 42,
    totalSegments: 120,
    translatedSegments: 50,
    vendorId: "v1",
    vendorName: "김번역",
    deliveredAt: null,
    rejectionNote: null,
  },
  {
    id: "a2",
    projectId: "p1",
    projectName: "Annual Report 2025",
    fileName: "annual_report_ch2.xliff",
    sourceLang: "en",
    targetLang: "ko",
    deadline: "2026-04-12T18:00:00.000Z",
    status: "pending",
    progressPct: 0,
    totalSegments: 85,
    translatedSegments: 0,
    vendorId: "v1",
    vendorName: "김번역",
    deliveredAt: null,
    rejectionNote: null,
  },
  {
    id: "a3",
    projectId: "p2",
    projectName: "User Manual v3",
    fileName: "manual_section_a.xliff",
    sourceLang: "ja",
    targetLang: "ko",
    deadline: "2026-04-08T18:00:00.000Z",
    status: "delivered",
    progressPct: 100,
    totalSegments: 60,
    translatedSegments: 60,
    vendorId: "v1",
    vendorName: "김번역",
    deliveredAt: "2026-04-05T10:30:00.000Z",
    rejectionNote: null,
  },
];

const MOCK_VENDORS: VendorInfo[] = [
  {
    id: "v1",
    username: "kim_translator",
    displayName: "김번역",
    email: "kim@vendor.com",
    langPairs: ["en→ko", "ja→ko"],
    activeAssignments: 2,
    totalDelivered: 18,
  },
  {
    id: "v2",
    username: "park_trans",
    displayName: "박번역",
    email: "park@vendor.com",
    langPairs: ["en→ko", "zh→ko"],
    activeAssignments: 1,
    totalDelivered: 7,
  },
];

function apiBase(): string {
  return ((window as unknown) as Record<string, unknown>).__WEB_API_BASE__ as string ?? "";
}

function authHeader(): Record<string, string> {
  const token = localStorage.getItem("mq_access_token");
  return token ? { Authorization: `Bearer ${token}` } : {};
}

// ── Store ─────────────────────────────────────────────────────────────────

interface VendorState {
  /** My assignments (vendor view) */
  myAssignments: VendorAssignment[];
  /** All assignments for all vendors (admin view) */
  allAssignments: VendorAssignment[];
  /** Vendor list (admin view) */
  vendors: VendorInfo[];
  /** Currently open assignment for editing */
  activeAssignment: VendorAssignment | null;
  isLoading: boolean;
  error: string | null;

  // Vendor actions
  fetchMyAssignments: () => Promise<void>;
  openAssignment: (assignment: VendorAssignment) => void;
  closeAssignment: () => void;
  updateProgress: (assignmentId: string, translatedSegments: number) => void;
  deliver: (assignmentId: string) => Promise<void>;

  // Admin actions
  fetchAllAssignments: () => Promise<void>;
  fetchVendors: () => Promise<void>;
  acceptDelivery: (assignmentId: string) => Promise<void>;
  rejectDelivery: (assignmentId: string, note: string) => Promise<void>;

  setError: (msg: string | null) => void;
}

export const useVendorStore = create<VendorState>((set, _get) => ({
  myAssignments: [],
  allAssignments: [],
  vendors: [],
  activeAssignment: null,
  isLoading: false,
  error: null,

  async fetchMyAssignments() {
    set({ isLoading: true, error: null });
    try {
      const base = apiBase();
      if (base) {
        const res = await fetch(`${base}/api/vendor/assignments`, {
          headers: { ...authHeader(), Accept: "application/json" },
        });
        if (!res.ok) throw new Error(`서버 오류: ${res.status}`);
        const data = await res.json() as VendorAssignment[];
        set({ myAssignments: data, isLoading: false });
      } else {
        // Mock fallback (development)
        set({ myAssignments: MOCK_ASSIGNMENTS, isLoading: false });
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      set({ isLoading: false, error: msg, myAssignments: MOCK_ASSIGNMENTS });
    }
  },

  openAssignment(assignment) {
    set({ activeAssignment: assignment });
  },

  closeAssignment() {
    set({ activeAssignment: null });
  },

  updateProgress(assignmentId, translatedSegments) {
    const update = (list: VendorAssignment[]): VendorAssignment[] =>
      list.map((a) => {
        if (a.id !== assignmentId) return a;
        const progressPct =
          a.totalSegments > 0
            ? Math.round((translatedSegments / a.totalSegments) * 100)
            : 0;
        return {
          ...a,
          translatedSegments,
          progressPct,
          status: progressPct > 0 ? "in_progress" : a.status,
        };
      });

    set((state) => ({
      myAssignments: update(state.myAssignments),
      allAssignments: update(state.allAssignments),
      activeAssignment:
        state.activeAssignment?.id === assignmentId
          ? { ...state.activeAssignment, translatedSegments }
          : state.activeAssignment,
    }));
  },

  async deliver(assignmentId) {
    set({ isLoading: true, error: null });
    try {
      const base = apiBase();
      if (base) {
        const res = await fetch(`${base}/api/vendor/assignments/${assignmentId}/deliver`, {
          method: "POST",
          headers: { ...authHeader(), "Content-Type": "application/json" },
        });
        if (!res.ok) throw new Error(`납품 제출 실패: ${res.status}`);
      }
      const deliveredAt = new Date().toISOString();
      const update = (list: VendorAssignment[]): VendorAssignment[] =>
        list.map((a) =>
          a.id === assignmentId
            ? { ...a, status: "delivered", progressPct: 100, deliveredAt }
            : a
        );
      set((state) => ({
        isLoading: false,
        myAssignments: update(state.myAssignments),
        allAssignments: update(state.allAssignments),
        activeAssignment: null,
      }));
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      set({ isLoading: false, error: msg });
      throw e;
    }
  },

  async acceptDelivery(assignmentId) {
    set({ isLoading: true, error: null });
    try {
      const base = apiBase();
      if (base) {
        const res = await fetch(`${base}/api/admin/vendor/assignments/${assignmentId}/accept`, {
          method: "POST",
          headers: { ...authHeader(), "Content-Type": "application/json" },
        });
        if (!res.ok) throw new Error(`수락 실패: ${res.status}`);
      }
      const update = (list: VendorAssignment[]): VendorAssignment[] =>
        list.map((a) =>
          a.id === assignmentId ? { ...a, status: "accepted" } : a
        );
      set((state) => ({
        isLoading: false,
        allAssignments: update(state.allAssignments),
        myAssignments: update(state.myAssignments),
      }));
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      set({ isLoading: false, error: msg });
      throw e;
    }
  },

  async rejectDelivery(assignmentId, note) {
    set({ isLoading: true, error: null });
    try {
      const base = apiBase();
      if (base) {
        const res = await fetch(`${base}/api/admin/vendor/assignments/${assignmentId}/reject`, {
          method: "POST",
          headers: { ...authHeader(), "Content-Type": "application/json" },
          body: JSON.stringify({ note }),
        });
        if (!res.ok) throw new Error(`반려 실패: ${res.status}`);
      }
      const update = (list: VendorAssignment[]): VendorAssignment[] =>
        list.map((a) =>
          a.id === assignmentId ? { ...a, status: "rejected", rejectionNote: note } : a
        );
      set((state) => ({
        isLoading: false,
        allAssignments: update(state.allAssignments),
        myAssignments: update(state.myAssignments),
      }));
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      set({ isLoading: false, error: msg });
      throw e;
    }
  },

  async fetchAllAssignments() {
    set({ isLoading: true, error: null });
    try {
      const base = apiBase();
      if (base) {
        const res = await fetch(`${base}/api/admin/vendor/assignments`, {
          headers: { ...authHeader(), Accept: "application/json" },
        });
        if (!res.ok) throw new Error(`서버 오류: ${res.status}`);
        const data = await res.json() as VendorAssignment[];
        set({ allAssignments: data, isLoading: false });
      } else {
        // Aggregate mock data with vendor-2 items
        const allMock: VendorAssignment[] = [
          ...MOCK_ASSIGNMENTS,
          {
            id: "a4",
            projectId: "p2",
            projectName: "User Manual v3",
            fileName: "manual_section_b.xliff",
            sourceLang: "en",
            targetLang: "ko",
            deadline: "2026-04-09T18:00:00.000Z",
            status: "in_progress",
            progressPct: 70,
            totalSegments: 50,
            translatedSegments: 35,
            vendorId: "v2",
            vendorName: "박번역",
            deliveredAt: null,
            rejectionNote: null,
          },
        ];
        set({ allAssignments: allMock, isLoading: false });
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      set({ isLoading: false, error: msg });
    }
  },

  async fetchVendors() {
    set({ isLoading: true, error: null });
    try {
      const base = apiBase();
      if (base) {
        const res = await fetch(`${base}/api/admin/vendors`, {
          headers: { ...authHeader(), Accept: "application/json" },
        });
        if (!res.ok) throw new Error(`서버 오류: ${res.status}`);
        const data = await res.json() as VendorInfo[];
        set({ vendors: data, isLoading: false });
      } else {
        set({ vendors: MOCK_VENDORS, isLoading: false });
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      set({ isLoading: false, error: msg });
    }
  },

  setError(msg) {
    set({ error: msg });
  },
}));
