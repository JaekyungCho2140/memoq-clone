// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom";
import { PluginManager } from "../PluginManager";
import { PluginRow } from "../PluginRow";
import { PluginConfigModal } from "../PluginConfigModal";
import { PluginInstallPanel } from "../PluginInstallPanel";
import { usePluginStore } from "../../../stores/pluginStore";
import type { Plugin } from "../../../types";

// ── Mocks ─────────────────────────────────────────────────────────────────────

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

vi.mock("../../../adapters", () => ({
  adapter: {
    pluginList: vi.fn().mockResolvedValue([]),
    pluginInstall: vi.fn(),
    pluginUpdate: vi.fn(),
    pluginRemove: vi.fn(),
  },
  fileRefFromDrop: vi.fn((f: File) => `web-file://${f.name}`),
  isTauri: vi.fn(() => false),
}));

import { adapter, fileRefFromDrop } from "../../../adapters";

const mockPlugin: Plugin = {
  id: "p1",
  name: "Test MT Plugin",
  version: "1.0.0",
  kind: "MtProvider",
  enabled: true,
  params: [{ key: "apiKey", label: "API Key", value: "existing", secret: true }],
  error: null,
  installedAt: "2026-04-04T00:00:00Z",
};

const errorPlugin: Plugin = {
  ...mockPlugin,
  id: "p2",
  name: "Broken Plugin",
  error: "WASM load failed",
  enabled: false,
};

function resetStore() {
  usePluginStore.setState({ plugins: [], loading: false, error: null });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

// ── PluginRow ─────────────────────────────────────────────────────────────────

describe("PluginRow", () => {
  it("renders plugin name, version and kind", () => {
    render(
      <PluginRow
        plugin={mockPlugin}
        onToggle={vi.fn()}
        onConfigure={vi.fn()}
        onRemove={vi.fn()}
      />
    );
    expect(screen.getByText("Test MT Plugin")).toBeInTheDocument();
    expect(screen.getByText("v1.0.0")).toBeInTheDocument();
    expect(screen.getByText("MT 프로바이더")).toBeInTheDocument();
  });

  it("shows error text when plugin has error", () => {
    render(
      <PluginRow
        plugin={errorPlugin}
        onToggle={vi.fn()}
        onConfigure={vi.fn()}
        onRemove={vi.fn()}
      />
    );
    expect(screen.getByText(/WASM load failed/)).toBeInTheDocument();
  });

  it("toggle calls onToggle with correct args", () => {
    const onToggle = vi.fn();
    render(
      <PluginRow
        plugin={mockPlugin}
        onToggle={onToggle}
        onConfigure={vi.fn()}
        onRemove={vi.fn()}
      />
    );
    const checkbox = screen.getByRole("checkbox");
    fireEvent.click(checkbox);
    expect(onToggle).toHaveBeenCalledWith("p1", false);
  });

  it("configure button calls onConfigure with plugin", () => {
    const onConfigure = vi.fn();
    render(
      <PluginRow
        plugin={mockPlugin}
        onToggle={vi.fn()}
        onConfigure={onConfigure}
        onRemove={vi.fn()}
      />
    );
    fireEvent.click(screen.getByText("설정"));
    expect(onConfigure).toHaveBeenCalledWith(mockPlugin);
  });

  it("remove button calls onRemove with plugin id", () => {
    const onRemove = vi.fn();
    render(
      <PluginRow
        plugin={mockPlugin}
        onToggle={vi.fn()}
        onConfigure={vi.fn()}
        onRemove={onRemove}
      />
    );
    fireEvent.click(screen.getByText("제거"));
    expect(onRemove).toHaveBeenCalledWith("p1");
  });
});

// ── PluginConfigModal ─────────────────────────────────────────────────────────

describe("PluginConfigModal", () => {
  it("renders plugin name in title", () => {
    render(
      <PluginConfigModal
        plugin={mockPlugin}
        onSave={vi.fn()}
        onClose={vi.fn()}
      />
    );
    expect(screen.getByText(/Test MT Plugin/)).toBeInTheDocument();
  });

  it("renders param labels as inputs", () => {
    render(
      <PluginConfigModal
        plugin={mockPlugin}
        onSave={vi.fn()}
        onClose={vi.fn()}
      />
    );
    expect(screen.getByText("API Key")).toBeInTheDocument();
    expect(screen.getByDisplayValue("existing")).toBeInTheDocument();
  });

  it("shows message when no params", () => {
    const noParamPlugin = { ...mockPlugin, params: [] };
    render(
      <PluginConfigModal
        plugin={noParamPlugin}
        onSave={vi.fn()}
        onClose={vi.fn()}
      />
    );
    expect(screen.getByText(/설정 가능한 파라미터가 없습니다/)).toBeInTheDocument();
  });

  it("calls onSave with updated values on submit", () => {
    const onSave = vi.fn();
    render(
      <PluginConfigModal
        plugin={mockPlugin}
        onSave={onSave}
        onClose={vi.fn()}
      />
    );
    const input = screen.getByDisplayValue("existing");
    fireEvent.change(input, { target: { value: "new-key" } });
    fireEvent.click(screen.getByText("저장"));
    expect(onSave).toHaveBeenCalledWith({ apiKey: "new-key" });
  });

  it("calls onClose when cancel is clicked", () => {
    const onClose = vi.fn();
    render(
      <PluginConfigModal
        plugin={mockPlugin}
        onSave={vi.fn()}
        onClose={onClose}
      />
    );
    fireEvent.click(screen.getByText("취소"));
    expect(onClose).toHaveBeenCalled();
  });
});

// ── PluginInstallPanel ────────────────────────────────────────────────────────

describe("PluginInstallPanel", () => {
  it("install button is disabled when no file selected", () => {
    render(<PluginInstallPanel installing={false} onInstall={vi.fn()} />);
    expect(screen.getByText("설치")).toBeDisabled();
  });

  it("shows installing state", () => {
    render(<PluginInstallPanel installing={true} onInstall={vi.fn()} />);
    expect(screen.getByText("설치 중...")).toBeDisabled();
  });

  it("calls onInstall with file ref when file selected and install clicked", async () => {
    const onInstall = vi.fn();
    vi.mocked(fileRefFromDrop).mockReturnValueOnce("web-file://test.wasm");
    render(<PluginInstallPanel installing={false} onInstall={onInstall} />);

    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    const wasmFile = new File(["wasm"], "plugin.wasm", { type: "application/wasm" });
    Object.defineProperty(input, "files", { value: [wasmFile], configurable: true });
    fireEvent.change(input);

    fireEvent.click(screen.getByText("설치"));
    expect(onInstall).toHaveBeenCalledWith({ wasmFile: "web-file://test.wasm" });
  });
});

// ── PluginManager (integration) ───────────────────────────────────────────────

describe("PluginManager", () => {
  it("shows empty message when no plugins installed", async () => {
    vi.mocked(adapter.pluginList).mockResolvedValueOnce([]);
    render(<PluginManager />);
    await waitFor(() => {
      expect(screen.getByText("설치된 플러그인이 없습니다.")).toBeInTheDocument();
    });
  });

  it("renders plugin rows after fetch", async () => {
    vi.mocked(adapter.pluginList).mockResolvedValueOnce([mockPlugin]);
    render(<PluginManager />);
    await waitFor(() => {
      expect(screen.getByText("Test MT Plugin")).toBeInTheDocument();
    });
  });

  it("shows error banner on fetch failure", async () => {
    vi.mocked(adapter.pluginList).mockRejectedValueOnce(new Error("서버 오류"));
    render(<PluginManager />);
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("서버 오류");
    });
  });

  it("dismisses error banner when × clicked", async () => {
    vi.mocked(adapter.pluginList).mockRejectedValueOnce(new Error("서버 오류"));
    render(<PluginManager />);
    await waitFor(() => screen.getByRole("alert"));
    fireEvent.click(screen.getByText("×"));
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("opens config modal when 설정 clicked", async () => {
    vi.mocked(adapter.pluginList).mockResolvedValueOnce([mockPlugin]);
    render(<PluginManager />);
    await waitFor(() => screen.getByText("설정"));
    fireEvent.click(screen.getByText("설정"));
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("closes config modal on 취소", async () => {
    vi.mocked(adapter.pluginList).mockResolvedValueOnce([mockPlugin]);
    render(<PluginManager />);
    await waitFor(() => screen.getByText("설정"));
    fireEvent.click(screen.getByText("설정"));
    fireEvent.click(screen.getByText("취소"));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("calls removePlugin via confirm dialog", async () => {
    vi.mocked(adapter.pluginList).mockResolvedValueOnce([mockPlugin]);
    vi.mocked(adapter.pluginRemove).mockResolvedValueOnce(undefined);
    vi.spyOn(window, "confirm").mockReturnValueOnce(true);
    render(<PluginManager />);
    await waitFor(() => screen.getByText("제거"));
    fireEvent.click(screen.getByText("제거"));
    await waitFor(() => {
      expect(adapter.pluginRemove).toHaveBeenCalledWith("p1");
    });
  });
});
