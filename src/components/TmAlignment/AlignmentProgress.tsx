import { useAlignmentStore } from "../../stores/alignmentStore";

export function AlignmentProgress() {
  const progress = useAlignmentStore((s) => s.progress);

  return (
    <div className="alignment-progress-page">
      <h2 className="alignment-section-title">정렬 처리 중...</h2>
      <p className="alignment-description">
        소스·타겟 문서를 분석하고 문장을 정렬하고 있습니다. 잠시 기다려 주세요.
      </p>
      <div className="alignment-progress-bar-wrap">
        <div
          className="alignment-progress-bar"
          style={{ width: `${progress}%` }}
          role="progressbar"
          aria-valuenow={progress}
          aria-valuemin={0}
          aria-valuemax={100}
        />
      </div>
      <p className="alignment-progress-pct">{progress}%</p>
    </div>
  );
}
