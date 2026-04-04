/**
 * useSegmentWs — WebSocket hook for real-time segment collaboration.
 *
 * Connects to /api/projects/:projectId/ws?token=<access_token>
 * on mount and disconnects on unmount or when projectId changes.
 *
 * Inbound server messages:
 *  - segment:lock    → mark segment as locked by another user
 *  - segment:unlock  → clear lock
 *  - segment:update  → apply remote segment translation to local store
 *  - init_locks      → initial snapshot of all active locks
 *
 * Outbound (returned from the hook):
 *  - lockSegment(id)   → send Lock message
 *  - unlockSegment(id) → send Unlock message
 *  - updateSegment(id, target, status) → send Update message
 *
 * Reconnect strategy (non-intentional disconnects):
 *  - Exponential backoff: 1s → 2s → 4s → … up to 30s cap
 *  - Max 8 retries before giving up and setting status to "disconnected"
 *  - Token expiry (code 4001): refresh token first, then reconnect (resets counter)
 *
 * Only runs in web mode (window.__TAURI__ absent).
 */

import { useEffect, useRef, useCallback } from "react";
import { isTauri, projectWsUrl } from "../adapters";
import { useWsStore } from "../stores/wsStore";
import { useProjectStore } from "../stores/projectStore";
import { useAuthStore, tokenSecondsRemaining } from "../stores/authStore";

const MAX_RECONNECT_ATTEMPTS = 8;
const BASE_RECONNECT_DELAY_MS = 1000;
const MAX_RECONNECT_DELAY_MS = 30_000;

/** Compute exponential back-off delay for a given attempt (0-indexed). */
function reconnectDelay(attempt: number): number {
  return Math.min(BASE_RECONNECT_DELAY_MS * 2 ** attempt, MAX_RECONNECT_DELAY_MS);
}

interface ServerMsgLock {
  type: "segment:lock";
  segment_id: string;
  user_id: string;
  username: string;
}
interface ServerMsgUnlock {
  type: "segment:unlock";
  segment_id: string;
}
interface ServerMsgUpdate {
  type: "segment:update";
  segment_id: string;
  target: string;
  status: string;
}
interface ServerMsgInitLocks {
  type: "init_locks";
  locks: Record<string, { user_id: string; username: string }>;
}

type ServerMsg = ServerMsgLock | ServerMsgUnlock | ServerMsgUpdate | ServerMsgInitLocks;

interface UseSegmentWsReturn {
  lockSegment: (segmentId: string) => void;
  unlockSegment: (segmentId: string) => void;
  updateSegment: (segmentId: string, target: string, status: string) => void;
}

export function useSegmentWs(projectId: string | null): UseSegmentWsReturn {
  const wsRef = useRef<WebSocket | null>(null);
  const retryCountRef = useRef(0);
  const retryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isMountedRef = useRef(true);

  const { setLock, clearLock, setStatus, reset } = useWsStore();
  const updateSegmentInStore = useProjectStore((s) => s.updateSegment);
  const { accessToken, refresh } = useAuthStore();

  useEffect(() => {
    // Skip in Tauri desktop mode
    if (isTauri()) return;
    if (!projectId) return;
    if (!accessToken) return;

    isMountedRef.current = true;

    const clearRetryTimer = () => {
      if (retryTimerRef.current !== null) {
        clearTimeout(retryTimerRef.current);
        retryTimerRef.current = null;
      }
    };

    const scheduleReconnect = (attempt: number) => {
      if (!isMountedRef.current) return;
      if (attempt >= MAX_RECONNECT_ATTEMPTS) {
        setStatus("disconnected");
        return;
      }
      const delay = reconnectDelay(attempt);
      retryTimerRef.current = setTimeout(() => {
        if (isMountedRef.current) connect(attempt);
      }, delay);
    };

    const connect = (attempt = 0) => {
      if (!isMountedRef.current) return;
      setStatus("connecting");
      const url = projectWsUrl(projectId);
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        retryCountRef.current = 0;
        setStatus("connected");
      };

      ws.onmessage = (evt) => {
        let msg: ServerMsg;
        try {
          msg = JSON.parse(evt.data as string) as ServerMsg;
        } catch {
          return;
        }

        if (msg.type === "segment:lock") {
          setLock(msg.segment_id, { userId: msg.user_id, username: msg.username });
        } else if (msg.type === "segment:unlock") {
          clearLock(msg.segment_id);
        } else if (msg.type === "segment:update") {
          updateSegmentInStore(msg.segment_id, {
            target: msg.target,
            status: msg.status as "draft" | "translated" | "confirmed",
          });
        } else if (msg.type === "init_locks") {
          for (const [segId, info] of Object.entries(msg.locks)) {
            setLock(segId, { userId: info.user_id, username: info.username });
          }
        }
      };

      ws.onerror = () => {
        setStatus("error");
      };

      ws.onclose = async (evt) => {
        wsRef.current = null;

        // Intentional unmount close — do not reconnect
        if (evt.code === 1000) return;

        // Token expired — refresh first, then reconnect (reset retry counter)
        if (evt.code === 4001) {
          const ok = await refresh();
          if (ok && isMountedRef.current) {
            retryCountRef.current = 0;
            scheduleReconnect(0);
            return;
          }
          setStatus("disconnected");
          return;
        }

        // Network drop or server-side close — exponential back-off
        const nextAttempt = attempt + 1;
        retryCountRef.current = nextAttempt;
        scheduleReconnect(nextAttempt);
      };
    };

    // Pre-emptive refresh: if token expires within 5 min, refresh first
    const secondsLeft = tokenSecondsRemaining(accessToken);
    if (secondsLeft < 300) {
      refresh().then(() => connect(0)).catch(() => connect(0));
    } else {
      connect(0);
    }

    return () => {
      isMountedRef.current = false;
      clearRetryTimer();
      wsRef.current?.close(1000, "unmount");
      wsRef.current = null;
      retryCountRef.current = 0;
      reset();
    };
  }, [projectId, accessToken]);

  const send = useCallback((payload: object) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(payload));
    }
  }, []);

  const lockSegment = useCallback(
    (segmentId: string) => send({ type: "lock", segment_id: segmentId }),
    [send],
  );

  const unlockSegment = useCallback(
    (segmentId: string) => send({ type: "unlock", segment_id: segmentId }),
    [send],
  );

  const updateSegment = useCallback(
    (segmentId: string, target: string, status: string) =>
      send({ type: "update", segment_id: segmentId, target, status }),
    [send],
  );

  return { lockSegment, unlockSegment, updateSegment };
}

/** Exported for unit testing only. */
export { reconnectDelay, MAX_RECONNECT_ATTEMPTS };
