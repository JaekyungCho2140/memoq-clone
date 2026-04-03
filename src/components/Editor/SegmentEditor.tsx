import { useEffect, useRef, useState } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { useMtStore } from "../../stores/mtStore";
import { useWsStore } from "../../stores/wsStore";
import { useAuthStore } from "../../stores/authStore";
import { useSegmentWs } from "../../hooks/useSegmentWs";
import { adapter } from "../../adapters";
import type { SegmentStatus } from "../../types";

const STATUS_LABEL: Record<SegmentStatus, string> = {
  untranslated: "미번역",
  draft: "초안",
  translated: "번역됨",
  confirmed: "확정",
};

export function SegmentEditor() {
  const { project, currentSegmentIndex, updateSegment, setCurrentSegmentIndex } =
    useProjectStore();
  const { provider, apiKey, cacheResult, getCached } = useMtStore();
  const { locks } = useWsStore();
  const { user } = useAuthStore();
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [mtLoading, setMtLoading] = useState(false);
  const [mtError, setMtError] = useState<string | null>(null);

  const { lockSegment, unlockSegment, updateSegment: wsUpdateSegment } = useSegmentWs(
    project?.id ?? null,
  );

  const segment = project?.segments[currentSegmentIndex];
  const prevSegmentId = useRef<string | undefined>(undefined);

  // Focus textarea and manage WS locks when segment changes
  useEffect(() => {
    const prevId = prevSegmentId.current;
    const nextId = segment?.id;

    if (prevId && prevId !== nextId) {
      unlockSegment(prevId);
    }
    if (nextId) {
      lockSegment(nextId);
      prevSegmentId.current = nextId;
    }

    textareaRef.current?.focus();
    setMtError(null);
  }, [segment?.id]);

  // Unlock on unmount
  useEffect(() => {
    return () => {
      if (prevSegmentId.current) {
        unlockSegment(prevSegmentId.current);
      }
    };
  }, []);

  // Ctrl+M global handler (fires even when textarea doesn't have focus)
  useEffect(() => {
    const handleGlobal = (e: KeyboardEvent) => {
      if (e.key === "m" && e.ctrlKey && !e.shiftKey && !e.altKey) {
        e.preventDefault();
        handleMt();
      }
    };
    window.addEventListener("keydown", handleGlobal);
    return () => window.removeEventListener("keydown", handleGlobal);
  }, [segment?.id, project?.id, provider, apiKey]);

  if (!project) return null;
  if (!segment) return <div className="empty-editor">세그먼트를 선택하세요</div>;

  const lockInfo = locks[segment.id];
  const isLockedByOther = lockInfo && lockInfo.userId !== (user?.id ?? "");

  const goToNext = () => {
    const next = currentSegmentIndex + 1;
    if (next < project.segments.length) {
      setCurrentSegmentIndex(next);
    }
  };

  const handleTargetChange = async (value: string) => {
    if (isLockedByOther) return;
    const status: SegmentStatus = value.trim() ? "draft" : "untranslated";
    updateSegment(segment.id, { target: value, status });
    // Broadcast to other users via WS
    wsUpdateSegment(segment.id, value, status);
    await adapter.saveSegment(project.id, segment.id, segment.source, value, status, segment.order);
  };

  const handleConfirm = async () => {
    if (isLockedByOther) return;
    updateSegment(segment.id, { status: "confirmed" });
    wsUpdateSegment(segment.id, segment.target, "confirmed");
    await adapter.saveSegment(project.id, segment.id, segment.source, segment.target, "confirmed", segment.order);
    goToNext();
  };

  const handleMt = async () => {
    if (!segment || mtLoading || isLockedByOther) return;
    setMtError(null);

    // Use cached result if available
    const cached = getCached(segment.id);
    if (cached) {
      await applyMtResult(cached.target);
      return;
    }

    if (!apiKey.trim()) {
      setMtError("MT API 키가 설정되지 않았습니다. 설정 패널을 확인하세요.");
      return;
    }

    setMtLoading(true);
    try {
      const result = await adapter.mtTranslate({
        source: segment.source,
        sourceLang: project.sourceLang,
        targetLang: project.targetLang,
        provider,
        apiKey,
      });
      cacheResult(segment.id, result);
      await applyMtResult(result.target);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.includes("mt_translate")) {
        setMtError("MT 백엔드가 아직 준비 중입니다 (AFR-16 대기).");
      } else {
        setMtError(`MT 오류: ${msg}`);
      }
    } finally {
      setMtLoading(false);
    }
  };

  const applyMtResult = async (target: string) => {
    updateSegment(segment.id, { target, status: "draft" });
    wsUpdateSegment(segment.id, target, "draft");
    await adapter.saveSegment(project.id, segment.id, segment.source, target, "draft", segment.order);
    textareaRef.current?.focus();
  };

  const handleKeyDown = async (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && e.ctrlKey) {
      e.preventDefault();
      await handleConfirm();
    } else if (e.key === "m" && e.ctrlKey && !e.shiftKey && !e.altKey) {
      e.preventDefault();
      await handleMt();
    } else if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.altKey) {
      e.preventDefault();
      goToNext();
    }
  };

  return (
    <div className="segment-editor">
      {isLockedByOther && (
        <div className="segment-lock-banner" role="status">
          🔒 {lockInfo.username}님이 편집 중입니다
        </div>
      )}
      <div className="source-cell">{segment.source}</div>
      <div className="target-cell">
        <textarea
          ref={textareaRef}
          value={segment.target}
          onChange={(e) => handleTargetChange(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="번역 입력..."
          rows={6}
          disabled={isLockedByOther}
          aria-label={isLockedByOther ? `${lockInfo.username}님이 편집 중` : "번역 입력"}
        />
      </div>
      <div className="editor-actions">
        <button onClick={handleConfirm} disabled={segment.status === "confirmed" || isLockedByOther}>
          확정 (Ctrl+Enter)
        </button>
        <button
          className="btn-mt"
          onClick={handleMt}
          disabled={mtLoading || isLockedByOther}
          title="MT 번역 삽입 (Ctrl+M)"
        >
          {mtLoading ? <span className="mt-spinner" /> : "MT (Ctrl+M)"}
        </button>
        <span className={`segment-status status-${segment.status}`}>
          {STATUS_LABEL[segment.status]}
        </span>
      </div>
      {mtError && <div className="mt-error">{mtError}</div>}
    </div>
  );
}
