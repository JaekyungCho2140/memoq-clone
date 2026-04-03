/**
 * Auth Store — JWT token management for web mode.
 *
 * Stores access + refresh tokens in localStorage.
 * In Tauri mode, auth is a no-op (the adapter skips it).
 *
 * Token lifecycle:
 * - Access token: 30 min TTL; auto-refreshed when < 5 min remain.
 * - Refresh token: 7 day TTL; used once to get new access+refresh pair.
 */

import { create } from "zustand";

const ACCESS_KEY = "mq_access_token";
const REFRESH_KEY = "mq_refresh_token";
const USER_KEY = "mq_user";

export interface AuthUser {
  id: string;
  username: string;
}

interface AuthState {
  user: AuthUser | null;
  accessToken: string | null;
  refreshToken: string | null;
  isLoading: boolean;
  error: string | null;

  /** Called once on app boot (web mode) to restore session from localStorage. */
  rehydrate: () => void;
  login: (username: string, password: string) => Promise<void>;
  logout: () => void;
  /** Silently obtain a new access token using the stored refresh token. */
  refresh: () => Promise<boolean>;
  setError: (msg: string | null) => void;
}

function apiBase(): string {
  return ((window as unknown) as Record<string, unknown>).__WEB_API_BASE__ as string ?? "";
}

async function authFetch(path: string, body: unknown, token?: string): Promise<Response> {
  return fetch(`${apiBase()}${path}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify(body),
  });
}

export const useAuthStore = create<AuthState>((set, get) => ({
  user: null,
  accessToken: null,
  refreshToken: null,
  isLoading: false,
  error: null,

  rehydrate() {
    const accessToken = localStorage.getItem(ACCESS_KEY);
    const refreshToken = localStorage.getItem(REFRESH_KEY);
    const raw = localStorage.getItem(USER_KEY);
    const user: AuthUser | null = raw ? (JSON.parse(raw) as AuthUser) : null;
    set({ accessToken, refreshToken, user });
  },

  async login(username: string, password: string) {
    set({ isLoading: true, error: null });
    try {
      const res = await authFetch("/api/auth/login", { username, password });
      if (!res.ok) {
        const text = await res.text().catch(() => res.statusText);
        throw new Error(text || `Login failed: ${res.status}`);
      }
      const data = await res.json() as {
        access_token: string;
        refresh_token: string;
        user: AuthUser;
      };
      localStorage.setItem(ACCESS_KEY, data.access_token);
      localStorage.setItem(REFRESH_KEY, data.refresh_token);
      localStorage.setItem(USER_KEY, JSON.stringify(data.user));
      set({
        accessToken: data.access_token,
        refreshToken: data.refresh_token,
        user: data.user,
        isLoading: false,
        error: null,
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      set({ isLoading: false, error: msg });
      throw e;
    }
  },

  logout() {
    const { accessToken } = get();
    // Fire-and-forget server logout
    if (accessToken) {
      fetch(`${apiBase()}/api/auth/logout`, {
        method: "POST",
        headers: { Authorization: `Bearer ${accessToken}` },
      }).catch(() => {/* ignore */});
    }
    localStorage.removeItem(ACCESS_KEY);
    localStorage.removeItem(REFRESH_KEY);
    localStorage.removeItem(USER_KEY);
    set({ user: null, accessToken: null, refreshToken: null, error: null });
  },

  async refresh(): Promise<boolean> {
    const { refreshToken } = get();
    if (!refreshToken) return false;
    try {
      const res = await authFetch("/api/auth/refresh", { refresh_token: refreshToken });
      if (!res.ok) return false;
      const data = await res.json() as {
        access_token: string;
        refresh_token: string;
      };
      localStorage.setItem(ACCESS_KEY, data.access_token);
      localStorage.setItem(REFRESH_KEY, data.refresh_token);
      set({ accessToken: data.access_token, refreshToken: data.refresh_token });
      return true;
    } catch {
      return false;
    }
  },

  setError(msg) {
    set({ error: msg });
  },
}));

/** Parse JWT payload without verifying signature (browser-side display only). */
export function parseJwtPayload(token: string): Record<string, unknown> | null {
  try {
    const parts = token.split(".");
    if (parts.length !== 3) return null;
    const padded = parts[1].padEnd(parts[1].length + (4 - (parts[1].length % 4)) % 4, "=");
    return JSON.parse(atob(padded)) as Record<string, unknown>;
  } catch {
    return null;
  }
}

/** Seconds until the access token expires. Returns 0 if expired/invalid. */
export function tokenSecondsRemaining(token: string): number {
  const payload = parseJwtPayload(token);
  if (!payload || typeof payload.exp !== "number") return 0;
  return Math.max(0, payload.exp - Math.floor(Date.now() / 1000));
}
