import { useState } from "react";
import { useVendorStore } from "../../stores/vendorStore";
import type { VendorAssignment } from "../../types";

interface Props {
  assignment: VendorAssignment;
  onClose: () => void;
}

export function DeliveryReviewModal({ assignment, onClose }: Props) {
  const { acceptDelivery, rejectDelivery, isLoading, error } = useVendorStore();
  const [rejecting, setRejecting] = useState(false);
  const [rejectNote, setRejectNote] = useState("");

  const handleAccept = async () => {
    await acceptDelivery(assignment.id);
    onClose();
  };

  const handleReject = async () => {
    if (!rejectNote.trim()) return;
    await rejectDelivery(assignment.id, rejectNote.trim());
    onClose();
  };

  return (
    <div
      className="modal-overlay"
      role="dialog"
      aria-modal="true"
      aria-labelledby="review-modal-title"
    >
      <div className="modal-box delivery-modal">
        <h2 id="review-modal-title" className="modal-title">
          납품 검토
        </h2>

        <div className="delivery-info">
          <div className="delivery-row">
            <span className="delivery-label">벤더</span>
            <span className="delivery-value">{assignment.vendorName}</span>
          </div>
          <div className="delivery-row">
            <span className="delivery-label">프로젝트</span>
            <span className="delivery-value">{assignment.projectName}</span>
          </div>
          <div className="delivery-row">
            <span className="delivery-label">파일</span>
            <span className="delivery-value">{assignment.fileName}</span>
          </div>
          <div className="delivery-row">
            <span className="delivery-label">납품일</span>
            <span className="delivery-value">
              {assignment.deliveredAt
                ? new Date(assignment.deliveredAt).toLocaleString("ko-KR")
                : "—"}
            </span>
          </div>
          <div className="delivery-row">
            <span className="delivery-label">완료율</span>
            <span className="delivery-value">
              {assignment.translatedSegments}/{assignment.totalSegments} ({assignment.progressPct}%)
            </span>
          </div>
        </div>

        {error && (
          <div className="delivery-warning" role="alert">
            {error}
          </div>
        )}

        {rejecting ? (
          <div className="reject-section">
            <label htmlFor="reject-note" className="reject-label">
              반려 사유 <span aria-hidden="true">*</span>
            </label>
            <textarea
              id="reject-note"
              className="reject-textarea"
              value={rejectNote}
              onChange={(e) => setRejectNote(e.target.value)}
              placeholder="반려 사유를 입력하세요"
              rows={3}
            />
          </div>
        ) : null}

        <div className="modal-actions">
          {!rejecting ? (
            <>
              <button
                className="btn-delivery btn-accept"
                onClick={handleAccept}
                disabled={isLoading}
                data-testid="accept-delivery-btn"
              >
                ✓ 수락
              </button>
              <button
                className="btn-delivery btn-reject"
                onClick={() => setRejecting(true)}
                disabled={isLoading}
              >
                ✗ 반려
              </button>
              <button
                className="btn-delivery btn-cancel"
                onClick={onClose}
                disabled={isLoading}
              >
                취소
              </button>
            </>
          ) : (
            <>
              <button
                className="btn-delivery btn-reject"
                onClick={handleReject}
                disabled={isLoading || !rejectNote.trim()}
                data-testid="confirm-reject-btn"
              >
                반려 확정
              </button>
              <button
                className="btn-delivery btn-cancel"
                onClick={() => setRejecting(false)}
                disabled={isLoading}
              >
                돌아가기
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
