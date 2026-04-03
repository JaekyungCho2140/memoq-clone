/**
 * UpdateChecker — 앱 시작 시 자동 업데이트를 확인하고 사용자에게 알린다.
 *
 * - Tauri 환경에서만 실행 (웹 빌드에서는 no-op).
 * - 60초 후 최초 체크, 이후 6시간마다 반복.
 * - 업데이트 발견 시 비침습적 배너(상단 고정)로 알림.
 */

import { useEffect, useState } from "react";

// tauri-plugin-updater 타입 (선택적 임포트 — 웹 빌드에서는 undefined)
let checkUpdate: (() => Promise<unknown>) | undefined;
let installUpdate: ((update: unknown) => Promise<void>) | undefined;

if (typeof window !== "undefined" && (window as Window & { __TAURI__?: unknown }).__TAURI__) {
  void import("@tauri-apps/plugin-updater").then((m) => {
    checkUpdate = m.check;
  });
  void import("@tauri-apps/plugin-process").then((m) => {
    installUpdate = async (update) => {
      await (update as { downloadAndInstall: () => Promise<void> }).downloadAndInstall();
      await m.relaunch();
    };
  });
}

interface PendingUpdate {
  version: string;
  body: string | null | undefined;
  raw: unknown;
}

const CHECK_DELAY_MS = 60_000;        // 1 min after startup
const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000; // 6 hours

export function UpdateChecker() {
  const [pending, setPending] = useState<PendingUpdate | null>(null);
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    if (!checkUpdate) return;

    const run = async () => {
      try {
        const update = checkUpdate && (await checkUpdate()) as { available?: boolean; version?: string; body?: string | null } | null;
        if (update && update.available) {
          setPending({
            version: update.version ?? "unknown",
            body: update.body,
            raw: update,
          });
        }
      } catch {
        // Silently ignore — network unavailable, etc.
      }
    };

    const initial = window.setTimeout(run, CHECK_DELAY_MS);
    const periodic = window.setInterval(run, CHECK_INTERVAL_MS);
    return () => {
      window.clearTimeout(initial);
      window.clearInterval(periodic);
    };
  }, []);

  if (!pending) return null;

  const handleInstall = async () => {
    if (!installUpdate) return;
    setInstalling(true);
    try {
      await installUpdate(pending.raw);
    } catch {
      setInstalling(false);
    }
  };

  const handleDismiss = () => setPending(null);

  return (
    <div
      role="alert"
      aria-live="polite"
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        right: 0,
        zIndex: 9999,
        background: "#2563eb",
        color: "#fff",
        padding: "8px 16px",
        display: "flex",
        alignItems: "center",
        gap: 12,
        fontSize: 13,
      }}
    >
      <span style={{ flex: 1 }}>
        🎉 새 버전 <strong>v{pending.version}</strong> 이 출시됐습니다.
        {pending.body && (
          <span style={{ marginLeft: 6, opacity: 0.85 }}>{pending.body}</span>
        )}
      </span>
      <button
        onClick={handleInstall}
        disabled={installing}
        style={{
          background: "#fff",
          color: "#2563eb",
          border: "none",
          borderRadius: 4,
          padding: "4px 12px",
          cursor: installing ? "wait" : "pointer",
          fontWeight: 600,
          fontSize: 12,
        }}
      >
        {installing ? "설치 중…" : "지금 설치"}
      </button>
      <button
        onClick={handleDismiss}
        aria-label="닫기"
        style={{
          background: "transparent",
          border: "none",
          color: "rgba(255,255,255,0.75)",
          cursor: "pointer",
          fontSize: 18,
          lineHeight: 1,
          padding: "0 4px",
        }}
      >
        ×
      </button>
    </div>
  );
}
