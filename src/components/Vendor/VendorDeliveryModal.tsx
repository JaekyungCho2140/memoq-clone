import { useState } from "react";
import { useVendorStore } from "../../stores/vendorStore";
import type { VendorAssignment } from "../../types";

interface Props {
  assignment: VendorAssignment;
  onCancel: () => void;
}

export function VendorDeliveryModal({ assignment, onCancel }: Props) {
  const { deliver, isLoading, error } = useVendorStore();
  const [confirmed, setConfirmed] = useState(false);

  const incompleteCount = assignment.totalSegments - assignment.translatedSegments;
  const hasIncomplete = incompleteCount > 0;

  const handleDeliver = async () => {
    if (!confirmed) return;
    try {
      await deliver(assignment.id);
    } catch {
      // error shown from store
    }
  };

  return (
    <div
      className="modal-overlay"
      role="dialog"
      aria-modal="true"
      aria-labelledby="delivery-modal-title"
    >
      <div className="modal-box delivery-modal">
        <h2 id="delivery-modal-title" className="modal-title">
          납품 제출
        </h2>

        <div className="delivery-info">
          <div className="delivery-row">
            <span className="delivery-label">프로젝트</span>
            <span className="delivery-value">{assignment.projectName}</span>
          </div>
          <div className="delivery-row">
            <span className="delivery-label">파일</span>
            <span className="delivery-value">{assignment.fileName}</span>
          </div>
          <div className="delivery-row">
            <span className="delivery-label">진행률</span>
            <span className="delivery-value">
              {assignment.translatedSegments}/{assignment.totalSegments} (
              {assignment.progressPct}%)
            </span>
          </div>
        </div>

        {hasIncomplete && (
          <div className="delivery-warning" role="alert">
            ⚠ 미번역 세그먼트가 <strong>{incompleteCount}개</strong> 남아 있습니다.
            미완성 상태로 제출하시겠습니까?
          </div>
        )}

        <label className="delivery-confirm-label">
          <input
            type="checkbox"
            checked={confirmed}
            onChange={(e) => setConfirmed(e.target.checked)}
          />
          <span>납품 내용을 확인했으며, 제출에 동의합니다.</span>
        </label>

        {error && (
          <div className="delivery-error" role="alert">
            {error}
          </div>
        )}

        <div className="modal-actions">
          <button
            className="btn btn-outline"
            onClick={onCancel}
            disabled={isLoading}
          >
            취소
          </button>
          <button
            className="btn btn-primary"
            onClick={handleDeliver}
            disabled={!confirmed || isLoading}
          >
            {isLoading ? "제출 중..." : "납품 제출"}
          </button>
        </div>
      </div>
    </div>
  );
}
