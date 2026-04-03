// @vitest-environment jsdom
/**
 * WS store unit tests
 */

import { describe, it, expect, beforeEach } from "vitest";
import { useWsStore } from "../wsStore";

describe("useWsStore", () => {
  beforeEach(() => {
    useWsStore.getState().reset();
  });

  it("initial state is empty", () => {
    const s = useWsStore.getState();
    expect(s.locks).toEqual({});
    expect(s.status).toBe("disconnected");
  });

  it("setLock adds a lock entry", () => {
    useWsStore.getState().setLock("seg-1", { userId: "u1", username: "alice" });
    expect(useWsStore.getState().locks["seg-1"]).toEqual({ userId: "u1", username: "alice" });
  });

  it("clearLock removes a lock entry", () => {
    useWsStore.getState().setLock("seg-1", { userId: "u1", username: "alice" });
    useWsStore.getState().clearLock("seg-1");
    expect(useWsStore.getState().locks["seg-1"]).toBeUndefined();
  });

  it("clearLock is a no-op for unknown segment", () => {
    expect(() => useWsStore.getState().clearLock("unknown")).not.toThrow();
  });

  it("setStatus transitions status", () => {
    useWsStore.getState().setStatus("connecting");
    expect(useWsStore.getState().status).toBe("connecting");
    useWsStore.getState().setStatus("connected");
    expect(useWsStore.getState().status).toBe("connected");
    useWsStore.getState().setStatus("error");
    expect(useWsStore.getState().status).toBe("error");
  });

  it("reset clears all state", () => {
    useWsStore.getState().setLock("seg-1", { userId: "u1", username: "alice" });
    useWsStore.getState().setStatus("connected");
    useWsStore.getState().reset();
    expect(useWsStore.getState().locks).toEqual({});
    expect(useWsStore.getState().status).toBe("disconnected");
  });

  it("multiple locks coexist without interference", () => {
    useWsStore.getState().setLock("seg-1", { userId: "u1", username: "alice" });
    useWsStore.getState().setLock("seg-2", { userId: "u2", username: "bob" });
    expect(Object.keys(useWsStore.getState().locks)).toHaveLength(2);
    useWsStore.getState().clearLock("seg-1");
    expect(useWsStore.getState().locks["seg-2"]).toBeDefined();
  });
});
