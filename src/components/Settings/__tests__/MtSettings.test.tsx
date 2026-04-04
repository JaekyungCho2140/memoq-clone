// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom";
import { MtSettings } from "../MtSettings";
import { useMtStore } from "../../../stores/mtStore";

// ── Mocks ──────────────────────────────────────────────────────────────────────

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockAdapter = vi.hoisted(() => ({
  mtLoadSettings: vi.fn().mockResolvedValue(null),
  mtSaveSettings: vi.fn().mockResolvedValue({}),
  mtTranslate: vi.fn().mockResolvedValue({ source: "Hello", target: "안녕", provider: "deepl" }),
}));

vi.mock("../../../adapters", () => ({
  adapter: mockAdapter,
  isTauri: vi.fn(() => false),
}));

beforeEach(() => {
  vi.clearAllMocks();
  useMtStore.setState({ provider: "deepl", apiKey: "", cache: {} });
});

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("MtSettings", () => {
  it("설정 폼이 렌더링된다 (빈 상태)", async () => {
    render(<MtSettings />);
    await waitFor(() => {
      expect(screen.getByRole("combobox")).toBeInTheDocument();
    });
  });

  it("저장된 설정이 있으면 로드 후 표시된다 (데이터 있는 상태)", async () => {
    mockAdapter.mtLoadSettings.mockResolvedValue({ provider: "google", apiKey: "saved-key" });
    render(<MtSettings />);
    await waitFor(() => {
      expect(useMtStore.getState().provider).toBe("google");
      expect(useMtStore.getState().apiKey).toBe("saved-key");
    });
  });

  it("저장 버튼이 존재한다", async () => {
    render(<MtSettings />);
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /저장/i })).toBeInTheDocument();
    });
  });

  it("저장 성공 시 완료 메시지가 표시된다", async () => {
    render(<MtSettings />);
    await waitFor(() => screen.getByRole("button", { name: /저장/i }));
    fireEvent.click(screen.getByRole("button", { name: /저장/i }));
    await waitFor(() => {
      expect(screen.getByText(/저장되었습니다/)).toBeInTheDocument();
    });
  });

  it("저장 실패 시 에러 메시지가 표시된다 (에러 상태)", async () => {
    mockAdapter.mtSaveSettings.mockRejectedValue(new Error("Network error"));
    render(<MtSettings />);
    await waitFor(() => screen.getByRole("button", { name: /저장/i }));
    fireEvent.click(screen.getByRole("button", { name: /저장/i }));
    await waitFor(() => {
      expect(screen.getByText(/저장 실패/)).toBeInTheDocument();
    });
  });
});
