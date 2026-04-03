import { useProjectStore } from "../../stores/projectStore";
import type { QaIssue } from "../../types";

const CHECK_TYPE_LABEL: Record<string, string> = {
  TagMismatch: "태그 불일치",
  NumberMismatch: "숫자 불일치",
  Untranslated: "미번역",
  ForbiddenTerm: "금지 용어",
  SourceEqualsTarget: "소스=타겟",
};

interface QaIssueItemProps {
  issue: QaIssue;
}

export function QaIssueItem({ issue }: QaIssueItemProps) {
  const { project, setCurrentSegmentIndex } = useProjectStore();

  const handleClick = () => {
    if (!project) return;
    const idx = project.segments.findIndex((s) => s.id === issue.segment_id);
    if (idx !== -1) setCurrentSegmentIndex(idx);
  };

  const segNum = project?.segments.findIndex((s) => s.id === issue.segment_id) ?? -1;

  return (
    <div
      className={`qa-issue-item qa-issue-${issue.severity.toLowerCase()}`}
      onClick={handleClick}
      title={`세그먼트 ${segNum + 1}로 이동`}
    >
      <span className={`qa-severity-icon qa-icon-${issue.severity.toLowerCase()}`}>
        {issue.severity === "Error" ? "✕" : "⚠"}
      </span>
      <span className="qa-segment-num">#{segNum + 1}</span>
      <span className="qa-check-type">{CHECK_TYPE_LABEL[issue.check_type] ?? issue.check_type}</span>
      <span className="qa-message">{issue.message}</span>
    </div>
  );
}
