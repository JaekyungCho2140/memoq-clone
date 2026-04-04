import { useCallback, useEffect, useRef, useState } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { useMtStore } from "../../stores/mtStore";
import { useWsStore } from "../../stores/wsStore";
import { useAuthStore } from "../../stores/authStore";
import { useTmStore } from "../../stores/tmStore";
import { useTbStore } from "../../stores/tbStore";
import { useSegmentWs } from "../../hooks/useSegmentWs";
import { adapter } from "../../adapters";
import type { SegmentStatus } from "../../types";

const STATUS_LABEL: Record<SegmentStatus, string> = {
  untranslated: "미번역",
  draft: "퍼지",
  translated: "100%",
  confirmed: "확인",
};

/** Highlight TB terms found in the source text. Returns an array of React nodes. */
function highlightTbTerms(text: string, terms: string[]): React.ReactNode[] {
  if (terms.length === 0) return [text];

  // Build a regex that matches any of the terms (case-insensitive)
  const escaped = terms.map((t) => t.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  const regex = new RegExp(`(${escaped.join("|")})`, "gi");

  const parts = text.split(regex);
  return parts.map((part, i) => {
    if (i % 2 === 1) {
      return (
        <mark key={i} className="tb-highlight">
          {part}
        </mark>
      );
    }
    return part;
  });
}

export function SegmentEditor() {
  const { project, currentSegmentIndex, updateSegment, setCurrentSegmentIndex } =
    useProjectStore();
  const { provider, apiKey, cacheResult, getCached } = useMtStore();
  const { locks } = useWsStore();
  const { user } = useAuthStore();
  const { currentTmMatches } = useTmStore();
  const { currentTbEntries } = useTbStore();
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [mtLoading, setMtLoading] = useState(false);
  const [mtError, setMtError] = useState<string | null>(null);
  /** Non-null when a segment save has failed */
  const [saveError, setSaveError] = useState<string | null>(null);
  /** Auto-clear save error after 5 s */
  const saveErrorTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const { lockSegment, unlockSegment, updateSegment: wsUpdateSegment } = useSegmentWs(
    project?.id ?? null,
  );

  const segment = project?.segments[currentSegmentIndex];
  const prevSegmentId = useRef<string | undefined>(undefined);

  const showSaveError = useCallback((msg: string) => {
    setSaveError(msg);
    if (saveErrorTimer.current) clearTimeout(saveErrorTimer.current);
    saveErrorTimer.current = setTimeout(() => setSaveError(null), 5000);
  }, []);

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
    setSaveError(null);
  }, [segment?.id]);

  // Unlock on unmount
  useEffect(() => {
    return () => {
      if (prevSegmentId.current) {
        unlockSegment(prevSegmentId.current);
      }
      if (saveErrorTimer.current) clearTimeout(saveErrorTimer.current);
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

  const goToPrev = () => {
    const prev = currentSegmentIndex - 1;
    if (prev >= 0) {
      setCurrentSegmentIndex(prev);
    }
  };

  const handleTargetChange = async (value: string) => {
    if (isLockedByOther) return;
    const status: SegmentStatus = value.trim() ? "draft" : "untranslated";
    updateSegment(segment.id, { target: value, status });
    wsUpdateSegment(segment.id, value, status);
    try {
      await adapter.saveSegment(project.id, segment.id, segment.source, value, status, segment.order);
    } catch {
      showSaveError("세그먼트 저장 실패. 네트워크 연결을 확인하세요.");
    }
  };

  const handleConfirm = async () => {
    if (isLockedByOther) return;
    updateSegment(segment.id, { status: "confirmed" });
    wsUpdateSegment(segment.id, segment.target, "confirmed");
    try {
      await adapter.saveSegment(project.id, segment.id, segment.source, segment.target, "confirmed", segment.order);
    } catch {
      showSaveError("확정 저장 실패. 네트워크 연결을 확인하세요.");
    }
    goToNext();
  };

  const handleMt = async () => {
    if (!segment || mtLoading || isLockedByOther) return;
    setMtError(null);

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
    try {
      await adapter.saveSegment(project.id, segment.id, segment.source, target, "draft", segment.order);
    } catch {
      showSaveError("MT 결과 저장 실패. 네트워크 연결을 확인하세요.");
    }
    textareaRef.current?.focus();
  };

  const applyTmMatch = async (target: string) => {
    if (isLockedByOther) return;
    updateSegment(segment.id, { target, status: "draft" });
    wsUpdateSegment(segment.id, target, "draft");
    try {
      await adapter.saveSegment(project.id, segment.id, segment.source, target, "draft", segment.order);
    } catch {
      showSaveError("TM 매치 저장 실패. 네트워크 연결을 확인하세요.");
    }
    textareaRef.current?.focus();
  };

  const handleKeyDown = async (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && e.ctrlKey) {
      // Ctrl+Enter = confirm + next
      e.preventDefault();
      await handleConfirm();
    } else if (e.key === "m" && e.ctrlKey && !e.shiftKey && !e.altKey) {
      // Ctrl+M = MT translate
      e.preventDefault();
      await handleMt();
    } else if (e.key === "Tab" && !e.ctrlKey && !e.altKey) {
      // Tab = next segment, Shift+Tab = previous segment
      e.preventDefault();
      if (e.shiftKey) {
        goToPrev();
      } else {
        goToNext();
      }
    } else if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.altKey) {
      // Plain Enter = go to next
      e.preventDefault();
      goToNext();
    } else if (e.ctrlKey && ["1", "2", "3"].includes(e.key)) {
      // Ctrl+1~3 = insert TM match
      e.preventDefault();
      const idx = parseInt(e.key, 10) - 1;
      const match = currentTmMatches[idx];
      if (match) {
        await applyTmMatch(match.target);
      }
    }
  };

  const tbTerms = currentTbEntries
    .filter((e) => !e.forbidden)
    .map((e) => e.sourceTerm);

  const sourceNodes = highlightTbTerms(segment.source, tbTerms);

  return (
    <div className="segment-editor">
      {isLockedByOther && (
        <div className="segment-lock-banner" role="status">
          🔒 {lockInfo.username}님이 편집 중입니다
        </div>
      )}

      {/* Source cell with TB term highlights */}
      <div className="source-cell" aria-label="소스 텍스트">
        {sourceNodes}
        {tbTerms.length > 0 && (
          <div className="tb-terms-hint" aria-label="TB 용어 표시">
            {currentTbEntries.filter((e) => !e.forbidden).map((e) => (
              <span key={e.id} className="tb-term-chip">
                <span className="tb-chip-source">{e.sourceTerm}</span>
                <span className="tb-chip-arrow">→</span>
                <span className="tb-chip-target">{e.targetTerm}</span>
              </span>
            ))}
          </div>
        )}
      </div>

      {/* TM match quick-insert bar (top-3) */}
      {currentTmMatches.length > 0 && (
        <div className="tm-quick-bar" aria-label="TM 매치 빠른 삽입">
          {currentTmMatches.slice(0, 3).map((m, i) => (
            <button
              key={i}
              className="tm-quick-btn"
              title={`Ctrl+${i + 1}: ${m.target}`}
              onClick={() => applyTmMatch(m.target)}
              disabled={isLockedByOther}
            >
              <span className="tm-quick-score">{Math.round(m.score * 100)}%</span>
              <span className="tm-quick-target">{m.target.slice(0, 40)}{m.target.length > 40 ? "…" : ""}</span>
              <kbd className="tm-quick-kbd">Ctrl+{i + 1}</kbd>
            </button>
          ))}
        </div>
      )}

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

      {mtError && (
        <div className="editor-error-banner" role="alert">
          {mtError}
        </div>
      )}
      {saveError && (
        <div className="editor-error-banner editor-save-error" role="alert">
          ⚠ {saveError}
        </div>
      )}
    </div>
  );
}
