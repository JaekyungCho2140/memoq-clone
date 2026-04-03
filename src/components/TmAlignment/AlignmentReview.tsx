import { useState } from "react";
import { useAlignmentStore } from "../../stores/alignmentStore";
import { adapter } from "../../adapters";
import type { AlignedPair } from "../../types";

function PairRow({
  pair,
  onConfirm,
  onUnconfirm,
  onDelete,
  onEdit,
}: {
  pair: AlignedPair;
  onConfirm: () => void;
  onUnconfirm: () => void;
  onDelete: () => void;
  onEdit: (source: string, target: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [editSource, setEditSource] = useState(pair.source);
  const [editTarget, setEditTarget] = useState(pair.target);

  const handleSave = () => {
    onEdit(editSource, editTarget);
    setEditing(false);
  };

  const handleCancel = () => {
    setEditSource(pair.source);
    setEditTarget(pair.target);
    setEditing(false);
  };

  return (
    <div className={`alignment-pair-row${pair.confirmed ? " confirmed" : ""}${pair.modified ? " modified" : ""}`}>
      <div className="pair-score" title="정렬 신뢰도">
        {Math.round(pair.score * 100)}%
      </div>

      <div className="pair-texts">
        {editing ? (
          <>
            <textarea
              className="pair-edit-source"
              value={editSource}
              onChange={(e) => setEditSource(e.target.value)}
              rows={2}
            />
            <textarea
              className="pair-edit-target"
              value={editTarget}
              onChange={(e) => setEditTarget(e.target.value)}
              rows={2}
            />
          </>
        ) : (
          <>
            <div className="pair-source">{pair.source}</div>
            <div className="pair-target">{pair.target}</div>
          </>
        )}
      </div>

      <div className="pair-actions">
        {editing ? (
          <>
            <button className="btn-small" onClick={handleSave} title="저장">✓</button>
            <button className="btn-small btn-outline" onClick={handleCancel} title="취소">✕</button>
          </>
        ) : (
          <>
            {pair.confirmed ? (
              <button className="btn-small btn-outline" onClick={onUnconfirm} title="확정 취소">
                확정됨
              </button>
            ) : (
              <button className="btn-small btn-primary" onClick={onConfirm} title="TM에 저장">
                확정
              </button>
            )}
            <button className="btn-small btn-outline" onClick={() => setEditing(true)} title="편집">
              편집
            </button>
            <button className="btn-small btn-danger" onClick={onDelete} title="삭제">
              삭제
            </button>
          </>
        )}
      </div>
    </div>
  );
}

export function AlignmentReview() {
  const {
    pairs,
    sourceLang,
    targetLang,
    tmId,
    confirmPair,
    unconfirmPair,
    deletePair,
    editPair,
    confirmAll,
    setPhase,
    setError,
    reset,
  } = useAlignmentStore();

  const [saving, setSaving] = useState(false);

  const confirmedCount = pairs.filter((p) => p.confirmed).length;
  const totalCount = pairs.length;

  const handleSaveToTm = async () => {
    if (!tmId || saving || confirmedCount === 0) return;
    setSaving(true);
    try {
      const confirmedPairs = pairs
        .filter((p) => p.confirmed)
        .map((p) => ({ source: p.source, target: p.target }));

      await adapter.alignmentConfirm({
        tmId,
        sourceLang,
        targetLang,
        pairs: confirmedPairs,
      });
      setPhase("done");
    } catch (err) {
      const msg = err instanceof Error ? err.message : "TM 저장 중 오류가 발생했습니다.";
      setError(msg);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="alignment-review">
      <div className="alignment-review-header">
        <div>
          <h2 className="alignment-section-title">정렬 결과 검토</h2>
          <p className="alignment-review-stats">
            {totalCount}개 항목 중 {confirmedCount}개 확정됨
          </p>
        </div>
        <div className="alignment-review-actions">
          <button className="btn btn-outline" onClick={confirmAll} disabled={saving}>
            전체 확정
          </button>
          <button
            className="btn btn-primary"
            onClick={handleSaveToTm}
            disabled={saving || confirmedCount === 0}
          >
            {saving ? "저장 중..." : `TM에 저장 (${confirmedCount})`}
          </button>
          <button className="btn btn-outline" onClick={reset} disabled={saving}>
            다시 시작
          </button>
        </div>
      </div>

      {pairs.length === 0 ? (
        <p className="alignment-empty">정렬 결과가 없습니다.</p>
      ) : (
        <div className="alignment-pair-list">
          <div className="alignment-pair-list-header">
            <span className="pair-header-score">신뢰도</span>
            <span className="pair-header-source">소스</span>
            <span className="pair-header-target">타겟</span>
            <span className="pair-header-actions">작업</span>
          </div>
          {pairs.map((pair) => (
            <PairRow
              key={pair.id}
              pair={pair}
              onConfirm={() => confirmPair(pair.id)}
              onUnconfirm={() => unconfirmPair(pair.id)}
              onDelete={() => deletePair(pair.id)}
              onEdit={(src, tgt) => editPair(pair.id, src, tgt)}
            />
          ))}
        </div>
      )}
    </div>
  );
}
