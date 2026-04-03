import { useEffect, useRef } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { saveSegment } from "../../tauri/commands";
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
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const segment = project?.segments[currentSegmentIndex];

  // 세그먼트가 바뀔 때마다 textarea에 포커스
  useEffect(() => {
    textareaRef.current?.focus();
  }, [segment?.id]);

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
    await saveSegment(project.id, segment.id, value, status);
  };

  const handleConfirm = async () => {
    updateSegment(segment.id, { status: "confirmed" });
    await saveSegment(project.id, segment.id, segment.target, "confirmed");
    goToNext();
  };

  const handleKeyDown = async (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && e.ctrlKey) {
      e.preventDefault();
      await handleConfirm();
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
        <span className={`segment-status status-${segment.status}`}>
          {STATUS_LABEL[segment.status]}
        </span>
      </div>
    </div>
  );
}
