import { useEffect, useRef, useState } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { useMtStore } from "../../stores/mtStore";
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
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [mtLoading, setMtLoading] = useState(false);
  const [mtError, setMtError] = useState<string | null>(null);

  const segment = project?.segments[currentSegmentIndex];

  // 세그먼트가 바뀔 때마다 textarea에 포커스
  useEffect(() => {
    textareaRef.current?.focus();
    setMtError(null);
  }, [segment?.id]);

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

  const goToNext = () => {
    const next = currentSegmentIndex + 1;
    if (next < project.segments.length) {
      setCurrentSegmentIndex(next);
    }
  };

  const handleTargetChange = async (value: string) => {
    const status: SegmentStatus = value.trim() ? "draft" : "untranslated";
    updateSegment(segment.id, { target: value, status });
    await adapter.saveSegment(project.id, segment.id, segment.source, value, status, segment.order);
  };

  const handleConfirm = async () => {
    updateSegment(segment.id, { status: "confirmed" });
    await adapter.saveSegment(project.id, segment.id, segment.source, segment.target, "confirmed", segment.order);
    goToNext();
  };

  const handleMt = async () => {
    if (!segment || mtLoading) return;
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
      <div className="source-cell">{segment.source}</div>
      <div className="target-cell">
        <textarea
          ref={textareaRef}
          value={segment.target}
          onChange={(e) => handleTargetChange(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="번역 입력..."
          rows={6}
        />
      </div>
      <div className="editor-actions">
        <button onClick={handleConfirm} disabled={segment.status === "confirmed"}>
          확정 (Ctrl+Enter)
        </button>
        <button
          className="btn-mt"
          onClick={handleMt}
          disabled={mtLoading}
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
