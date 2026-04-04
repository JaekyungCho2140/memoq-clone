import { useEffect, useState } from "react";
import { useVendorStore } from "../../stores/vendorStore";
import type { VendorAssignment, VendorAssignmentStatus, VendorInfo } from "../../types";
import { DeliveryReviewModal } from "./DeliveryReviewModal";

const STATUS_LABEL: Record<VendorAssignmentStatus, string> = {
  pending: "대기",
  in_progress: "진행 중",
  delivered: "납품 완료",
  accepted: "승인됨",
  rejected: "반려됨",
};

function formatDate(iso: string | null): string {
  if (!iso) return "—";
  return new Date(iso).toLocaleDateString("ko-KR", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

interface VendorCardProps {
  vendor: VendorInfo;
  assignments: VendorAssignment[];
}

function VendorCard({ vendor, assignments }: VendorCardProps) {
  const [expanded, setExpanded] = useState(false);
  const [reviewAssignment, setReviewAssignment] = useState<VendorAssignment | null>(null);
  const active = assignments.filter(
    (a) => a.status === "pending" || a.status === "in_progress",
  );
  const avgProgress =
    active.length > 0
      ? Math.round(active.reduce((s, a) => s + a.progressPct, 0) / active.length)
      : null;

  return (
    <div className="vendor-card">
      <div
        className="vendor-card-header"
        onClick={() => setExpanded((v) => !v)}
        role="button"
        tabIndex={0}
        aria-expanded={expanded}
        onKeyDown={(e) => e.key === "Enter" && setExpanded((v) => !v)}
      >
        <div className="vendor-card-meta">
          <span className="vendor-card-name">{vendor.displayName}</span>
          <span className="vendor-card-langs">{vendor.langPairs.join(", ")}</span>
        </div>
        <div className="vendor-card-stats">
          <span>활성 {active.length}건</span>
          <span>총 납품 {vendor.totalDelivered}건</span>
          {avgProgress !== null && (
            <span>평균 진행 {avgProgress}%</span>
          )}
        </div>
        <span className="vendor-card-toggle">{expanded ? "▲" : "▼"}</span>
      </div>

      {expanded && (
        <div className="vendor-card-body">
          {assignments.length === 0 ? (
            <p className="vendor-empty">할당된 작업 없음</p>
          ) : (
            <table className="admin-assignment-table">
              <thead>
                <tr>
                  <th>파일</th>
                  <th>언어</th>
                  <th>진행률</th>
                  <th>마감일</th>
                  <th>납품일</th>
                  <th>상태</th>
                  <th>작업</th>
                </tr>
              </thead>
              <tbody>
                {assignments.map((a) => (
                  <tr key={a.id}>
                    <td>
                      <div className="admin-project-name">{a.projectName}</div>
                      <div className="admin-file-name">{a.fileName}</div>
                    </td>
                    <td>{a.sourceLang.toUpperCase()} → {a.targetLang.toUpperCase()}</td>
                    <td>
                      <div className="progress-bar-wrap">
                        <div
                          className="progress-bar-fill"
                          style={{ width: `${a.progressPct}%` }}
                          role="progressbar"
                          aria-valuenow={a.progressPct}
                          aria-valuemin={0}
                          aria-valuemax={100}
                        />
                      </div>
                      <span className="progress-label">{a.progressPct}%</span>
                    </td>
                    <td>{formatDate(a.deadline)}</td>
                    <td>{formatDate(a.deliveredAt)}</td>
                    <td>
                      <span className={`status-badge status-${a.status}`}>
                        {STATUS_LABEL[a.status]}
                      </span>
                    </td>
                    <td>
                      {a.status === "delivered" && (
                        <button
                          className="btn-small btn-primary"
                          onClick={() => setReviewAssignment(a)}
                        >
                          검토
                        </button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}
      {reviewAssignment && (
        <DeliveryReviewModal
          assignment={reviewAssignment}
          onClose={() => setReviewAssignment(null)}
        />
      )}
    </div>
  );
}

interface AdminVendorDashboardProps {
  onClose?: () => void;
}

export function AdminVendorDashboard({ onClose }: AdminVendorDashboardProps) {
  const {
    allAssignments,
    vendors,
    isLoading,
    error,
    fetchAllAssignments,
    fetchVendors,
  } = useVendorStore();

  useEffect(() => {
    fetchVendors();
    fetchAllAssignments();
  }, [fetchVendors, fetchAllAssignments]);

  const totalActive = allAssignments.filter(
    (a) => a.status === "pending" || a.status === "in_progress",
  ).length;
  const totalDelivered = allAssignments.filter(
    (a) => a.status === "delivered" || a.status === "accepted",
  ).length;

  const assignmentsByVendor = (vendorId: string) =>
    allAssignments.filter((a) => a.vendorId === vendorId);

  return (
    <div className="admin-vendor-dashboard">
      <div className="admin-vendor-header">
        <h2 className="admin-section-title">벤더 현황 대시보드</h2>
        {onClose && (
          <button className="btn-small btn-outline" onClick={onClose}>
            ← 돌아가기
          </button>
        )}
      </div>

      <div className="admin-summary-bar">
        <div className="summary-card">
          <span className="summary-number">{vendors.length}</span>
          <span className="summary-label">총 벤더 수</span>
        </div>
        <div className="summary-card summary-card--active">
          <span className="summary-number">{totalActive}</span>
          <span className="summary-label">진행 중 작업</span>
        </div>
        <div className="summary-card">
          <span className="summary-number">{totalDelivered}</span>
          <span className="summary-label">납품 완료</span>
        </div>
      </div>

      {error && (
        <div className="admin-error" role="alert">
          {error}
        </div>
      )}

      {isLoading ? (
        <div className="admin-loading">불러오는 중...</div>
      ) : vendors.length === 0 ? (
        <div className="admin-empty">등록된 벤더가 없습니다.</div>
      ) : (
        <div className="vendor-cards">
          {vendors.map((v) => (
            <VendorCard
              key={v.id}
              vendor={v}
              assignments={assignmentsByVendor(v.id)}
            />
          ))}
        </div>
      )}
    </div>
  );
}
