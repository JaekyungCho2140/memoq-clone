import { useEffect } from "react";
import { useVendorStore } from "../../stores/vendorStore";
import { useAuthStore } from "../../stores/authStore";
import type { VendorAssignment, VendorAssignmentStatus } from "../../types";

const STATUS_LABEL: Record<VendorAssignmentStatus, string> = {
  pending: "대기",
  in_progress: "진행 중",
  delivered: "납품 완료",
  accepted: "승인됨",
  rejected: "반려됨",
};

const STATUS_CLASS: Record<VendorAssignmentStatus, string> = {
  pending: "status-pending",
  in_progress: "status-in-progress",
  delivered: "status-delivered",
  accepted: "status-accepted",
  rejected: "status-rejected",
};

function formatDeadline(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleDateString("ko-KR", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function isOverdue(iso: string, status: VendorAssignmentStatus): boolean {
  if (status === "delivered" || status === "accepted") return false;
  return new Date(iso) < new Date();
}

interface AssignmentRowProps {
  assignment: VendorAssignment;
  onOpen: (a: VendorAssignment) => void;
}

function AssignmentRow({ assignment: a, onOpen }: AssignmentRowProps) {
  const overdue = isOverdue(a.deadline, a.status);
  return (
    <tr className={`vendor-assignment-row${overdue ? " overdue" : ""}`}>
      <td className="vendor-cell">
        <div className="vendor-project-name">{a.projectName}</div>
        <div className="vendor-file-name">{a.fileName}</div>
      </td>
      <td className="vendor-cell vendor-lang-pair">
        {a.sourceLang.toUpperCase()} → {a.targetLang.toUpperCase()}
      </td>
      <td className="vendor-cell">
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
        <span className="progress-label">
          {a.translatedSegments}/{a.totalSegments} ({a.progressPct}%)
        </span>
      </td>
      <td className={`vendor-cell deadline-cell${overdue ? " overdue-text" : ""}`}>
        {formatDeadline(a.deadline)}
        {overdue && <span className="overdue-badge">기한 초과</span>}
      </td>
      <td className="vendor-cell">
        <span className={`status-badge ${STATUS_CLASS[a.status]}`}>
          {STATUS_LABEL[a.status]}
        </span>
      </td>
      <td className="vendor-cell">
        {(a.status === "pending" || a.status === "in_progress") && (
          <button
            className="btn-small btn-primary"
            onClick={() => onOpen(a)}
          >
            편집 열기
          </button>
        )}
      </td>
    </tr>
  );
}

export function VendorDashboard() {
  const { myAssignments, isLoading, error, fetchMyAssignments, openAssignment } =
    useVendorStore();
  const { user, logout } = useAuthStore();

  useEffect(() => {
    fetchMyAssignments();
  }, [fetchMyAssignments]);

  const pending = myAssignments.filter((a) => a.status === "pending").length;
  const inProgress = myAssignments.filter((a) => a.status === "in_progress").length;
  const delivered = myAssignments.filter(
    (a) => a.status === "delivered" || a.status === "accepted",
  ).length;

  return (
    <div className="vendor-dashboard">
      <header className="vendor-header">
        <div className="vendor-header-left">
          <span className="vendor-logo-icon">⚡</span>
          <h1 className="vendor-title">번역 포탈</h1>
        </div>
        <div className="vendor-header-right">
          <span className="vendor-username">{user?.username}</span>
          <button className="btn-small btn-outline" onClick={logout}>
            로그아웃
          </button>
        </div>
      </header>

      <div className="vendor-summary-bar">
        <div className="summary-card">
          <span className="summary-number">{pending}</span>
          <span className="summary-label">대기 중</span>
        </div>
        <div className="summary-card summary-card--active">
          <span className="summary-number">{inProgress}</span>
          <span className="summary-label">진행 중</span>
        </div>
        <div className="summary-card">
          <span className="summary-number">{delivered}</span>
          <span className="summary-label">납품 완료</span>
        </div>
      </div>

      {error && (
        <div className="vendor-error" role="alert">
          {error}
        </div>
      )}

      {isLoading ? (
        <div className="vendor-loading">불러오는 중...</div>
      ) : (
        <div className="vendor-table-wrap">
          <table className="vendor-table">
            <thead>
              <tr>
                <th>프로젝트 / 파일</th>
                <th>언어</th>
                <th>진행률</th>
                <th>마감일</th>
                <th>상태</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {myAssignments.length === 0 ? (
                <tr>
                  <td colSpan={6} className="vendor-empty">
                    할당된 작업이 없습니다.
                  </td>
                </tr>
              ) : (
                myAssignments.map((a) => (
                  <AssignmentRow
                    key={a.id}
                    assignment={a}
                    onOpen={openAssignment}
                  />
                ))
              )}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
