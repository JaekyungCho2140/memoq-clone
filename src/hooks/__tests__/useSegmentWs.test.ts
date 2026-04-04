// @vitest-environment jsdom
/**
 * useSegmentWs hook unit tests
 *
 * Covers:
 * - WebSocket 연결 / 해제 (connect / disconnect)
 * - 서버 메시지 핸들러: segment:lock, segment:unlock, segment:update, init_locks
 * - 잘못된 JSON 무시
 * - 토큰 만료(4001) 시 refresh → 재연결
 * - 재연결 실패 시 disconnected 상태로 전환
 * - 선제적 토큰 갱신 (tokenSecondsRemaining < 300)
 * - Tauri 환경에서 WebSocket 미생성
 * - projectId / accessToken 없을 때 WebSocket 미생성
 * - lockSegment / unlockSegment / updateSegment 메시지 전송
 * - WebSocket이 OPEN이 아닐 때 send 무시
 */

import { describe, it, expect, vi, beforeEach, afterEach, type Mock } from "vitest";
import { renderHook, act } from "@testing-library/react";

// ─── mock modules ─────────────────────────────────────────────────────────────

vi.mock("../../adapters", () => ({
  isTauri: vi.fn(() => false),
  projectWsUrl: vi.fn((id: string) => `ws://localhost/api/projects/${id}/ws`),
}));

vi.mock("../../stores/wsStore", () => ({
  useWsStore: vi.fn(),
}));

vi.mock("../../stores/projectStore", () => ({
  useProjectStore: vi.fn(),
}));

vi.mock("../../stores/authStore", () => ({
  useAuthStore: vi.fn(),
  tokenSecondsRemaining: vi.fn(() => 3600),
}));

import { isTauri, projectWsUrl } from "../../adapters";
import { useWsStore } from "../../stores/wsStore";
import { useProjectStore } from "../../stores/projectStore";
import { useAuthStore, tokenSecondsRemaining } from "../../stores/authStore";
import { useSegmentWs } from "../useSegmentWs";

// ─── WebSocket mock ───────────────────────────────────────────────────────────

class MockWebSocket {
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;
  static CONNECTING = 0;

  readyState: number = MockWebSocket.CONNECTING;
  url: string;
  onopen: ((evt: Event) => void) | null = null;
  onmessage: ((evt: MessageEvent) => void) | null = null;
  onerror: ((evt: Event) => void) | null = null;
  onclose: ((evt: CloseEvent) => void) | null = null;

  static instances: MockWebSocket[] = [];

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  open() {
    this.readyState = MockWebSocket.OPEN;
    this.onopen?.(new Event("open"));
  }

  receiveMessage(data: unknown) {
    const evt = new MessageEvent("message", { data: JSON.stringify(data) });
    this.onmessage?.(evt);
  }

  receiveRawMessage(raw: string) {
    const evt = new MessageEvent("message", { data: raw });
    this.onmessage?.(evt);
  }

  triggerError() {
    this.onerror?.(new Event("error"));
  }

  triggerClose(code = 1000, reason = "") {
    this.readyState = MockWebSocket.CLOSED;
    // Use plain object to avoid jsdom CloseEvent field inconsistencies
    this.onclose?.({ code, reason, wasClean: code === 1000 } as unknown as CloseEvent);
  }

  send = vi.fn();

  close = vi.fn((code?: number, reason?: string) => {
    this.triggerClose(code ?? 1000, reason ?? "");
  });
}

// ─── helpers ─────────────────────────────────────────────────────────────────

function makeStoreMocks() {
  const setLock = vi.fn();
  const clearLock = vi.fn();
  const setStatus = vi.fn();
  const reset = vi.fn();
  const updateSegment = vi.fn();
  const refresh = vi.fn().mockResolvedValue(true);

  (useWsStore as unknown as Mock).mockReturnValue({ setLock, clearLock, setStatus, reset });
  (useProjectStore as unknown as Mock).mockReturnValue(updateSegment);
  (useAuthStore as unknown as Mock).mockReturnValue({ accessToken: "valid-token", refresh });

  return { setLock, clearLock, setStatus, reset, updateSegment, refresh };
}

/** Flush microtask queue without relying on fake timers. */
async function flushMicrotasks() {
  await new Promise((resolve) => queueMicrotask(resolve as () => void));
  await new Promise((resolve) => queueMicrotask(resolve as () => void));
}

// ─── setup / teardown ────────────────────────────────────────────────────────

let OriginalWebSocket: typeof WebSocket;

beforeEach(() => {
  // Only fake setTimeout/clearTimeout — do NOT fake setImmediate/nextTick/queueMicrotask
  // so that Promise callbacks and async/await continue to work normally.
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "setInterval", "clearInterval"] });
  MockWebSocket.instances = [];
  OriginalWebSocket = global.WebSocket;
  (global as unknown as Record<string, unknown>).WebSocket = MockWebSocket;
  vi.mocked(isTauri).mockReturnValue(false);
  vi.mocked(tokenSecondsRemaining).mockReturnValue(3600);
});

afterEach(() => {
  (global as unknown as Record<string, unknown>).WebSocket = OriginalWebSocket;
  vi.useRealTimers();
  vi.clearAllMocks();
  delete (window as unknown as Record<string, unknown>).__TAURI__;
});

// ─── tests ───────────────────────────────────────────────────────────────────

describe("useSegmentWs — 연결 / 해제", () => {
  it("projectId와 accessToken이 있으면 WebSocket을 생성하고 connecting 상태로 설정", () => {
    const { setStatus } = makeStoreMocks();
    renderHook(() => useSegmentWs("proj-1"));

    expect(MockWebSocket.instances).toHaveLength(1);
    expect(MockWebSocket.instances[0].url).toContain("proj-1");
    expect(setStatus).toHaveBeenCalledWith("connecting");
  });

  it("WebSocket 연결 성공 시 connected 상태로 설정", () => {
    const { setStatus } = makeStoreMocks();
    renderHook(() => useSegmentWs("proj-1"));

    act(() => {
      MockWebSocket.instances[0].open();
    });

    expect(setStatus).toHaveBeenCalledWith("connected");
  });

  it("projectId가 null이면 WebSocket을 생성하지 않음", () => {
    makeStoreMocks();
    renderHook(() => useSegmentWs(null));

    expect(MockWebSocket.instances).toHaveLength(0);
  });

  it("accessToken이 없으면 WebSocket을 생성하지 않음", () => {
    (useWsStore as unknown as Mock).mockReturnValue({ setLock: vi.fn(), clearLock: vi.fn(), setStatus: vi.fn(), reset: vi.fn() });
    (useProjectStore as unknown as Mock).mockReturnValue(vi.fn());
    (useAuthStore as unknown as Mock).mockReturnValue({ accessToken: null, refresh: vi.fn() });

    renderHook(() => useSegmentWs("proj-1"));

    expect(MockWebSocket.instances).toHaveLength(0);
  });

  it("Tauri 환경에서는 WebSocket을 생성하지 않음", () => {
    vi.mocked(isTauri).mockReturnValue(true);
    makeStoreMocks();
    renderHook(() => useSegmentWs("proj-1"));

    expect(MockWebSocket.instances).toHaveLength(0);
  });

  it("unmount 시 WebSocket을 닫고 store reset 호출", () => {
    const { reset } = makeStoreMocks();
    const { unmount } = renderHook(() => useSegmentWs("proj-1"));

    act(() => {
      MockWebSocket.instances[0].open();
    });

    unmount();

    expect(MockWebSocket.instances[0].close).toHaveBeenCalledWith(1000, "unmount");
    expect(reset).toHaveBeenCalled();
  });

  it("onerror 발생 시 error 상태로 설정", () => {
    const { setStatus } = makeStoreMocks();
    renderHook(() => useSegmentWs("proj-1"));

    act(() => {
      MockWebSocket.instances[0].triggerError();
    });

    expect(setStatus).toHaveBeenCalledWith("error");
  });

  it("정상 종료(code !== 4001) 시 disconnected 상태로 설정", async () => {
    const { setStatus } = makeStoreMocks();
    renderHook(() => useSegmentWs("proj-1"));

    act(() => {
      MockWebSocket.instances[0].open();
    });

    await act(async () => {
      MockWebSocket.instances[0].triggerClose(1000);
      await flushMicrotasks();
    });

    expect(setStatus).toHaveBeenCalledWith("disconnected");
  });
});

describe("useSegmentWs — 서버 메시지 핸들러", () => {
  it("segment:lock 메시지를 받으면 setLock 호출", async () => {
    const { setLock } = makeStoreMocks();
    renderHook(() => useSegmentWs("proj-1"));

    act(() => {
      MockWebSocket.instances[0].open();
      MockWebSocket.instances[0].receiveMessage({
        type: "segment:lock",
        segment_id: "seg-1",
        user_id: "user-1",
        username: "Alice",
      });
    });

    expect(setLock).toHaveBeenCalledWith("seg-1", { userId: "user-1", username: "Alice" });
  });

  it("segment:unlock 메시지를 받으면 clearLock 호출", () => {
    const { clearLock } = makeStoreMocks();
    renderHook(() => useSegmentWs("proj-1"));

    act(() => {
      MockWebSocket.instances[0].open();
      MockWebSocket.instances[0].receiveMessage({
        type: "segment:unlock",
        segment_id: "seg-2",
      });
    });

    expect(clearLock).toHaveBeenCalledWith("seg-2");
  });

  it("segment:update 메시지를 받으면 updateSegment 호출", () => {
    const { updateSegment } = makeStoreMocks();
    renderHook(() => useSegmentWs("proj-1"));

    act(() => {
      MockWebSocket.instances[0].open();
      MockWebSocket.instances[0].receiveMessage({
        type: "segment:update",
        segment_id: "seg-3",
        target: "번역된 텍스트",
        status: "translated",
      });
    });

    expect(updateSegment).toHaveBeenCalledWith("seg-3", { target: "번역된 텍스트", status: "translated" });
  });

  it("init_locks 메시지를 받으면 모든 lock에 setLock 호출", () => {
    const { setLock } = makeStoreMocks();
    renderHook(() => useSegmentWs("proj-1"));

    act(() => {
      MockWebSocket.instances[0].open();
      MockWebSocket.instances[0].receiveMessage({
        type: "init_locks",
        locks: {
          "seg-a": { user_id: "u1", username: "Bob" },
          "seg-b": { user_id: "u2", username: "Carol" },
        },
      });
    });

    expect(setLock).toHaveBeenCalledWith("seg-a", { userId: "u1", username: "Bob" });
    expect(setLock).toHaveBeenCalledWith("seg-b", { userId: "u2", username: "Carol" });
  });

  it("잘못된 JSON을 받으면 무시 (에러 없음)", () => {
    const { setLock, clearLock, updateSegment } = makeStoreMocks();
    renderHook(() => useSegmentWs("proj-1"));

    act(() => {
      MockWebSocket.instances[0].open();
      MockWebSocket.instances[0].receiveRawMessage("not valid json {{");
    });

    expect(setLock).not.toHaveBeenCalled();
    expect(clearLock).not.toHaveBeenCalled();
    expect(updateSegment).not.toHaveBeenCalled();
  });
});

describe("useSegmentWs — 재연결 로직", () => {
  it("code 4001 수신 후 refresh 성공 시 200ms 후 재연결", async () => {
    const { refresh } = makeStoreMocks();
    renderHook(() => useSegmentWs("proj-1"));

    act(() => {
      MockWebSocket.instances[0].open();
    });

    // trigger close and let the async onclose handler run
    await act(async () => {
      MockWebSocket.instances[0].triggerClose(4001);
      await flushMicrotasks();
    });

    // refresh가 호출되어야 함
    expect(refresh).toHaveBeenCalled();

    // 200ms 후 새 WebSocket 연결 시도
    act(() => {
      vi.advanceTimersByTime(200);
    });

    expect(MockWebSocket.instances).toHaveLength(2);
  });

  it("code 4001 수신 후 refresh 실패 시 disconnected 상태", async () => {
    const { refresh, setStatus } = makeStoreMocks();
    refresh.mockResolvedValueOnce(false);
    renderHook(() => useSegmentWs("proj-1"));

    act(() => {
      MockWebSocket.instances[0].open();
    });

    await act(async () => {
      MockWebSocket.instances[0].triggerClose(4001);
      await flushMicrotasks();
    });

    expect(setStatus).toHaveBeenCalledWith("disconnected");
  });
});

describe("useSegmentWs — 선제적 토큰 갱신", () => {
  it("토큰 만료 300초 미만 시 refresh 후 connect 호출", async () => {
    vi.mocked(tokenSecondsRemaining).mockReturnValue(100);
    const { refresh } = makeStoreMocks();
    refresh.mockResolvedValue(true);

    renderHook(() => useSegmentWs("proj-1"));

    // refresh가 호출되어야 함
    expect(refresh).toHaveBeenCalled();

    // refresh 완료 후 WebSocket 연결
    await act(async () => {
      await flushMicrotasks();
    });

    expect(MockWebSocket.instances).toHaveLength(1);
  });

  it("토큰 만료 300초 이상 시 refresh 없이 바로 connect", () => {
    vi.mocked(tokenSecondsRemaining).mockReturnValue(3600);
    const { refresh } = makeStoreMocks();

    renderHook(() => useSegmentWs("proj-1"));

    expect(refresh).not.toHaveBeenCalled();
    expect(MockWebSocket.instances).toHaveLength(1);
  });
});

describe("useSegmentWs — 아웃바운드 메시지 전송", () => {
  it("lockSegment: WebSocket OPEN 상태에서 lock 메시지 전송", () => {
    makeStoreMocks();
    const { result } = renderHook(() => useSegmentWs("proj-1"));

    act(() => {
      MockWebSocket.instances[0].readyState = MockWebSocket.OPEN;
    });

    act(() => {
      result.current.lockSegment("seg-10");
    });

    expect(MockWebSocket.instances[0].send).toHaveBeenCalledWith(
      JSON.stringify({ type: "lock", segment_id: "seg-10" }),
    );
  });

  it("unlockSegment: WebSocket OPEN 상태에서 unlock 메시지 전송", () => {
    makeStoreMocks();
    const { result } = renderHook(() => useSegmentWs("proj-1"));

    act(() => {
      MockWebSocket.instances[0].readyState = MockWebSocket.OPEN;
    });

    act(() => {
      result.current.unlockSegment("seg-11");
    });

    expect(MockWebSocket.instances[0].send).toHaveBeenCalledWith(
      JSON.stringify({ type: "unlock", segment_id: "seg-11" }),
    );
  });

  it("updateSegment: WebSocket OPEN 상태에서 update 메시지 전송", () => {
    makeStoreMocks();
    const { result } = renderHook(() => useSegmentWs("proj-1"));

    act(() => {
      MockWebSocket.instances[0].readyState = MockWebSocket.OPEN;
    });

    act(() => {
      result.current.updateSegment("seg-12", "translated text", "confirmed");
    });

    expect(MockWebSocket.instances[0].send).toHaveBeenCalledWith(
      JSON.stringify({ type: "update", segment_id: "seg-12", target: "translated text", status: "confirmed" }),
    );
  });

  it("WebSocket이 OPEN이 아닐 때 send 무시", () => {
    makeStoreMocks();
    const { result } = renderHook(() => useSegmentWs("proj-1"));

    // readyState는 CONNECTING(0)인 상태
    act(() => {
      result.current.lockSegment("seg-99");
    });

    expect(MockWebSocket.instances[0].send).not.toHaveBeenCalled();
  });

  it("WebSocket이 없을 때(projectId null) send 무시", () => {
    makeStoreMocks();
    const { result } = renderHook(() => useSegmentWs(null));

    act(() => {
      result.current.lockSegment("seg-99");
    });

    expect(MockWebSocket.instances).toHaveLength(0);
  });
});

describe("useSegmentWs — projectWsUrl 호출", () => {
  it("WebSocket URL이 projectWsUrl 반환값 사용", () => {
    makeStoreMocks();
    vi.mocked(projectWsUrl).mockReturnValue("ws://test-server/api/projects/abc/ws");

    renderHook(() => useSegmentWs("abc"));

    expect(projectWsUrl).toHaveBeenCalledWith("abc");
    expect(MockWebSocket.instances[0].url).toBe("ws://test-server/api/projects/abc/ws");
  });
});
