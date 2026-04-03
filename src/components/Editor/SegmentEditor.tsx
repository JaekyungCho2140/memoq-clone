import { useProjectStore } from "../../stores/projectStore";
import { saveSegment } from "../../tauri/commands";
import type { SegmentStatus } from "../../types";

export function SegmentEditor() {
  const { project, currentSegmentIndex, updateSegment } = useProjectStore();
  if (!project) return null;
  const segment = project.segments[currentSegmentIndex];
  if (!segment) return <div className="empty-editor">세그먼트를 선택하세요</div>;

  const handleTargetChange = async (value: string) => {
    const status: SegmentStatus = value.trim() ? "draft" : "untranslated";
    updateSegment(segment.id, { target: value, status });
    await saveSegment(project.id, segment.id, value, status);
  };

  const handleConfirm = async () => {
    updateSegment(segment.id, { status: "confirmed" });
    await saveSegment(project.id, segment.id, segment.target, "confirmed");
  };

  return (
    <div className="segment-editor">
      <div className="source-cell">{segment.source}</div>
      <div className="target-cell">
        <textarea value={segment.target} onChange={(e) => handleTargetChange(e.target.value)} placeholder="번역 입력..." rows={6} />
      </div>
      <div className="editor-actions">
        <button onClick={handleConfirm} disabled={segment.status === "confirmed"}>확정 (Ctrl+Enter)</button>
        <span className="segment-status">{segment.status}</span>
      </div>
    </div>
  );
}
