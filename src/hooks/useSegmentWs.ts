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
 * Only runs in web mode (window.__TAURI__ absent).
 */

import { useEffect, useRef, useCallback } from "react";
import { isTauri, projectWsUrl } from "../adapters";
import { useWsStore } from "../stores/wsStore";
import { useProjectStore } from "../stores/projectStore";
import { useAuthStore, tokenSecondsRemaining } from "../stores/authStore";

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
  const { setLock, clearLock, setStatus, reset } = useWsStore();
  const updateSegmentInStore = useProjectStore((s) => s.updateSegment);
  const { accessToken, refresh } = useAuthStore();

  useEffect(() => {
    // Skip in Tauri desktop mode
    if (isTauri()) return;
    if (!projectId) return;
    if (!accessToken) return;

    const connect = () => {
      setStatus("connecting");
      const url = projectWsUrl(projectId);
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        setStatus("connected");
      };

      ws.onmessage = async (evt) => {
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
        // Code 4001 = token expired — try refresh then reconnect
        if (evt.code === 4001) {
          const ok = await refresh();
          if (ok) {
            setTimeout(connect, 200);
            return;
          }
        }
        setStatus("disconnected");
      };
    };

    // Pre-emptive refresh: if token expires within 5 min, refresh first
    const secondsLeft = tokenSecondsRemaining(accessToken);
    if (secondsLeft < 300) {
      refresh().then(connect).catch(connect);
    } else {
      connect();
    }

    return () => {
      wsRef.current?.close(1000, "unmount");
      wsRef.current = null;
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
