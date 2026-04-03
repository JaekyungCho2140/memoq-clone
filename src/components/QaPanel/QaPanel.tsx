import { useQaStore } from "../../stores/qaStore";
import { QaIssueItem } from "./QaIssueItem";

export function QaPanel() {
  const { issues, isRunning, lastRunAt, errorCount, warningCount } = useQaStore();

  const errors = issues.filter((i) => i.severity === "Error");
  const warnings = issues.filter((i) => i.severity === "Warning");

  return (
    <div className="qa-panel">
      <div className="panel-header">
        <h3>QA 체크</h3>
        {lastRunAt && (
          <span className="qa-last-run">
            {new Date(lastRunAt).toLocaleTimeString()}
          </span>
        )}
      </div>

      {isRunning && (
        <div className="qa-running">QA 체크 실행 중...</div>
      )}

      {!isRunning && issues.length === 0 && lastRunAt && (
        <div className="qa-empty">문제 없음 ✓</div>
      )}

      {!isRunning && issues.length === 0 && !lastRunAt && (
        <div className="qa-empty">F9를 눌러 QA 체크를 실행하세요</div>
      )}

      {!isRunning && errors.length > 0 && (
        <div className="qa-section">
          <div className="qa-section-header qa-section-error">
            오류 ({errorCount()})
          </div>
          {errors.map((issue, idx) => (
            <QaIssueItem key={`err-${idx}`} issue={issue} />
          ))}
        </div>
      )}

      {!isRunning && warnings.length > 0 && (
        <div className="qa-section">
          <div className="qa-section-header qa-section-warning">
            경고 ({warningCount()})
          </div>
          {warnings.map((issue, idx) => (
            <QaIssueItem key={`warn-${idx}`} issue={issue} />
          ))}
        </div>
      )}
    </div>
  );
}
