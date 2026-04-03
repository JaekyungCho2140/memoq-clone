import { useProjectStore } from "../../stores/projectStore";
import { useQaStore } from "../../stores/qaStore";
import type { SegmentStatus } from "../../types";

const STATUS_BADGE: Record<SegmentStatus, { label: string; bg: string }> = {
  untranslated: { label: "미번역", bg: "#555" },
  draft:        { label: "초안",   bg: "#b8860b" },
  translated:   { label: "번역됨", bg: "#2e7d32" },
  confirmed:    { label: "확정",   bg: "#1565c0" },
};

export function SegmentList() {
  const { project, currentSegmentIndex, setCurrentSegmentIndex } = useProjectStore();
  const { issuesBySegmentId } = useQaStore();
  if (!project) return null;

  return (
    <div className="segment-list">
      <div className="segment-list-header">
        <span className="col-num">#</span>
        <span className="col-source">소스</span>
        <span className="col-target">타겟</span>
        <span className="col-status">상태</span>
      </div>
      {project.segments.map((seg, idx) => {
        const badge = STATUS_BADGE[seg.status];
        const segIssues = issuesBySegmentId(seg.id);
        const hasError = segIssues.some((i) => i.severity === "Error");
        const hasWarning = segIssues.some((i) => i.severity === "Warning");

        return (
          <div
            key={seg.id}
            className={`segment-row${idx === currentSegmentIndex ? " active" : ""}`}
            onClick={() => setCurrentSegmentIndex(idx)}
          >
            <span className="col-num">
              {seg.order + 1}
              {hasError && <span className="qa-dot qa-dot-error" title="QA 오류" />}
              {!hasError && hasWarning && <span className="qa-dot qa-dot-warning" title="QA 경고" />}
            </span>
            <span className="col-source">{seg.source.slice(0, 60)}</span>
            <span className="col-target">
              {seg.target ? seg.target.slice(0, 60) : <em>—</em>}
            </span>
            <span className="col-status">
              <span className="status-badge" style={{ background: badge.bg }}>
                {badge.label}
              </span>
            </span>
          </div>
        );
      })}
    </div>
  );
}
