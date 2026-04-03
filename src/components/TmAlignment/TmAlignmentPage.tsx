import { useAlignmentStore } from "../../stores/alignmentStore";
import { AlignmentUpload } from "./AlignmentUpload";
import { AlignmentProgress } from "./AlignmentProgress";
import { AlignmentReview } from "./AlignmentReview";

interface TmAlignmentPageProps {
  onClose: () => void;
}

export function TmAlignmentPage({ onClose }: TmAlignmentPageProps) {
  const { phase, reset } = useAlignmentStore();

  const handleClose = () => {
    reset();
    onClose();
  };

  return (
    <div className="alignment-page">
      <div className="alignment-page-header">
        <h1 className="alignment-page-title">TM 정렬</h1>
        <button className="btn btn-outline alignment-close-btn" onClick={handleClose}>
          닫기
        </button>
      </div>

      <div className="alignment-phase-indicator">
        <span className={phase === "upload" ? "active" : "done"}>
          1. 파일 업로드
        </span>
        <span className="phase-sep">›</span>
        <span className={phase === "processing" ? "active" : phase === "review" || phase === "saving" || phase === "done" ? "done" : ""}>
          2. 정렬 처리
        </span>
        <span className="phase-sep">›</span>
        <span className={phase === "review" || phase === "saving" ? "active" : phase === "done" ? "done" : ""}>
          3. 검토 및 확정
        </span>
        <span className="phase-sep">›</span>
        <span className={phase === "done" ? "active done" : ""}>
          4. TM 저장 완료
        </span>
      </div>

      <div className="alignment-content">
        {phase === "upload" && <AlignmentUpload />}
        {phase === "processing" && <AlignmentProgress />}
        {(phase === "review" || phase === "saving") && <AlignmentReview />}
        {phase === "done" && (
          <div className="alignment-done">
            <div className="alignment-done-icon">✓</div>
            <h2>TM 저장 완료!</h2>
            <p>확정된 문장 쌍이 TM에 성공적으로 저장되었습니다.</p>
            <div className="alignment-done-actions">
              <button className="btn btn-primary" onClick={() => { reset(); }}>
                새 정렬 시작
              </button>
              <button className="btn btn-outline" onClick={handleClose}>
                닫기
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
