// @vitest-environment jsdom
/**
 * Auth store unit tests
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { useAuthStore, parseJwtPayload, tokenSecondsRemaining } from "../authStore";

const ACCESS_KEY = "mq_access_token";
const REFRESH_KEY = "mq_refresh_token";
const USER_KEY = "mq_user";

const fakeUser = { id: "u1", username: "alice", role: "admin" as const };
const fakeTokens = {
  access_token: "fake-access",
  refresh_token: "fake-refresh",
  user: fakeUser,
};

function resetStore() {
  useAuthStore.setState({
    user: null,
    accessToken: null,
    refreshToken: null,
    isLoading: false,
    error: null,
  });
}

describe("useAuthStore — rehydrate()", () => {
  beforeEach(() => {
    resetStore();
    localStorage.clear();
  });

  it("restores tokens and user from localStorage", () => {
    localStorage.setItem(ACCESS_KEY, "tok-a");
    localStorage.setItem(REFRESH_KEY, "tok-r");
    localStorage.setItem(USER_KEY, JSON.stringify(fakeUser));

    useAuthStore.getState().rehydrate();
    const s = useAuthStore.getState();
    expect(s.accessToken).toBe("tok-a");
    expect(s.refreshToken).toBe("tok-r");
    expect(s.user).toEqual(fakeUser);
  });

  it("sets nulls when localStorage is empty", () => {
    useAuthStore.getState().rehydrate();
    const s = useAuthStore.getState();
    expect(s.accessToken).toBeNull();
    expect(s.user).toBeNull();
  });
});

describe("useAuthStore — login()", () => {
  beforeEach(() => {
    resetStore();
    localStorage.clear();
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("stores tokens and user on success", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(JSON.stringify(fakeTokens), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    await useAuthStore.getState().login("alice", "pass");
    const s = useAuthStore.getState();
    expect(s.user).toEqual(fakeUser);
    expect(s.accessToken).toBe("fake-access");
    expect(s.refreshToken).toBe("fake-refresh");
    expect(s.isLoading).toBe(false);
    expect(s.error).toBeNull();
    expect(localStorage.getItem(ACCESS_KEY)).toBe("fake-access");
  });

  it("sets error on failed login", async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response("Unauthorized", { status: 401 }),
    );

    await expect(useAuthStore.getState().login("bad", "creds")).rejects.toBeTruthy();
    const s = useAuthStore.getState();
    expect(s.user).toBeNull();
    expect(s.error).toBeTruthy();
    expect(s.isLoading).toBe(false);
  });

  it("sets error on network failure", async () => {
    vi.mocked(fetch).mockRejectedValue(new Error("Network error"));

    await expect(useAuthStore.getState().login("alice", "pass")).rejects.toBeTruthy();
    expect(useAuthStore.getState().error).toBeTruthy();
  });
});

describe("useAuthStore — logout()", () => {
  beforeEach(() => {
    localStorage.setItem(ACCESS_KEY, "tok-a");
    localStorage.setItem(REFRESH_KEY, "tok-r");
    localStorage.setItem(USER_KEY, JSON.stringify(fakeUser));
    useAuthStore.setState({ user: fakeUser, accessToken: "tok-a", refreshToken: "tok-r" });
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response("", { status: 200 })));
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    localStorage.clear();
  });

  it("clears state and localStorage", () => {
    useAuthStore.getState().logout();
    const s = useAuthStore.getState();
    expect(s.user).toBeNull();
    expect(s.accessToken).toBeNull();
    expect(s.refreshToken).toBeNull();
    expect(localStorage.getItem(ACCESS_KEY)).toBeNull();
    expect(localStorage.getItem(REFRESH_KEY)).toBeNull();
    expect(localStorage.getItem(USER_KEY)).toBeNull();
  });
});

describe("useAuthStore — refresh()", () => {
  beforeEach(() => {
    resetStore();
    localStorage.clear();
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("returns false when no refresh token is stored", async () => {
    const ok = await useAuthStore.getState().refresh();
    expect(ok).toBe(false);
  });

  it("updates tokens on success", async () => {
    useAuthStore.setState({ refreshToken: "old-refresh" });
    vi.mocked(fetch).mockResolvedValue(
      new Response(JSON.stringify({ access_token: "new-access", refresh_token: "new-refresh" }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    const ok = await useAuthStore.getState().refresh();
    expect(ok).toBe(true);
    expect(useAuthStore.getState().accessToken).toBe("new-access");
    expect(localStorage.getItem(ACCESS_KEY)).toBe("new-access");
  });

  it("returns false on HTTP error", async () => {
    useAuthStore.setState({ refreshToken: "old-refresh" });
    vi.mocked(fetch).mockResolvedValue(new Response("Forbidden", { status: 403 }));
    const ok = await useAuthStore.getState().refresh();
    expect(ok).toBe(false);
  });
});

describe("useAuthStore — setError()", () => {
  it("sets and clears error", () => {
    useAuthStore.getState().setError("oops");
    expect(useAuthStore.getState().error).toBe("oops");
    useAuthStore.getState().setError(null);
    expect(useAuthStore.getState().error).toBeNull();
  });
});

// ── parseJwtPayload ────────────────────────────────────────────────────────

describe("parseJwtPayload()", () => {
  it("returns null for malformed token", () => {
    expect(parseJwtPayload("not-a-jwt")).toBeNull();
  });

  it("decodes a simple payload", () => {
    // Build a minimal JWT: header.payload.sig (signature not verified)
    const payload = { sub: "u1", exp: 9999999999 };
    const encoded = btoa(JSON.stringify(payload)).replace(/=/g, "");
    const token = `eyJhbGciOiJIUzI1NiJ9.${encoded}.fakesig`;
    const result = parseJwtPayload(token);
    expect(result).not.toBeNull();
    expect(result?.sub).toBe("u1");
  });
});

// ── tokenSecondsRemaining ──────────────────────────────────────────────────

describe("tokenSecondsRemaining()", () => {
  it("returns 0 for expired token", () => {
    const payload = { exp: 1 }; // long ago
    const encoded = btoa(JSON.stringify(payload)).replace(/=/g, "");
    const token = `h.${encoded}.s`;
    expect(tokenSecondsRemaining(token)).toBe(0);
  });

  it("returns positive seconds for valid token", () => {
    const exp = Math.floor(Date.now() / 1000) + 600; // 10 min
    const payload = { exp };
    const encoded = btoa(JSON.stringify(payload)).replace(/=/g, "");
    const token = `h.${encoded}.s`;
    const remaining = tokenSecondsRemaining(token);
    expect(remaining).toBeGreaterThan(0);
    expect(remaining).toBeLessThanOrEqual(600);
  });
});
