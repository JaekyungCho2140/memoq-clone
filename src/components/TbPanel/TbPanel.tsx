import { useEffect, useState } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { useTbStore } from "../../stores/tbStore";
import { adapter } from "../../adapters";
import type { TbEntry } from "../../types";

export function TbPanel() {
  const { project, currentSegmentIndex } = useProjectStore();
  const { activeTbId, setActiveTbId, setCurrentTbEntries } = useTbStore();
  const [entries, setEntries] = useState<TbEntry[]>([]);
  const [creating, setCreating] = useState(false);
  const [tbName, setTbName] = useState("");
  const [showAddTerm, setShowAddTerm] = useState(false);
  const [newSource, setNewSource] = useState("");
  const [newTarget, setNewTarget] = useState("");
  const [newNotes, setNewNotes] = useState("");
  const [newForbidden, setNewForbidden] = useState(false);

  const currentSegment = project?.segments[currentSegmentIndex];

  // Auto-create TB for the project if none exists
  useEffect(() => {
    if (!activeTbId && project) {
      adapter.createTb(`${project.name} TB`)
        .then((id) => setActiveTbId(id))
        .catch(() => {/* ignore */});
    }
  }, [project?.id]);

  // Look up terms in current segment source
  useEffect(() => {
    if (!activeTbId || !currentSegment || !project) {
      setEntries([]);
      return;
    }
    adapter.lookupTb({
      tbId: activeTbId,
      term: currentSegment.source,
      sourceLang: project.sourceLang,
    })
      .then((results) => {
        setEntries(results);
        setCurrentTbEntries(results);
      })
      .catch(() => {
        setEntries([]);
        setCurrentTbEntries([]);
      });
  }, [activeTbId, currentSegment?.id, project?.id]);

  const handleCreateTb = async () => {
    if (!tbName.trim()) return;
    try {
      const id = await adapter.createTb(tbName.trim());
      setActiveTbId(id);
      setTbName("");
      setCreating(false);
    } catch (e) {
      console.error("Failed to create TB:", e);
    }
  };

  const handleAddTerm = async () => {
    if (!activeTbId || !project || !newSource.trim() || !newTarget.trim()) return;
    try {
      await adapter.addToTb(
        activeTbId,
        newSource.trim(),
        newTarget.trim(),
        project.sourceLang,
        project.targetLang,
        newNotes,
        newForbidden,
      );
      setNewSource("");
      setNewTarget("");
      setNewNotes("");
      setNewForbidden(false);
      setShowAddTerm(false);
      // Refresh
      if (currentSegment) {
        const fresh = await adapter.lookupTb({
          tbId: activeTbId,
          term: currentSegment.source,
          sourceLang: project.sourceLang,
        });
        setEntries(fresh);
      }
    } catch (e) {
      console.error("Failed to add term:", e);
    }
  };

  return (
    <div className="tb-panel">
      <div className="panel-header">
        <h3>Term Base</h3>
        <div className="panel-actions">
          <button
            className="btn-small"
            onClick={() => setShowAddTerm((v) => !v)}
            title="새 용어 추가"
          >
            + 용어
          </button>
          <button
            className="btn-small btn-outline"
            onClick={() => setCreating((v) => !v)}
            title="새 TB 만들기"
          >
            새 TB
          </button>
        </div>
      </div>

      {creating && (
        <div className="create-form">
          <input
            type="text"
            value={tbName}
            onChange={(e) => setTbName(e.target.value)}
            placeholder="TB 이름"
            onKeyDown={(e) => e.key === "Enter" && handleCreateTb()}
            autoFocus
          />
          <button className="btn-small" onClick={handleCreateTb}>만들기</button>
          <button className="btn-small btn-outline" onClick={() => setCreating(false)}>취소</button>
        </div>
      )}

      {showAddTerm && (
        <div className="add-term-form">
          <input
            type="text"
            value={newSource}
            onChange={(e) => setNewSource(e.target.value)}
            placeholder="소스 용어"
          />
          <input
            type="text"
            value={newTarget}
            onChange={(e) => setNewTarget(e.target.value)}
            placeholder="타겟 용어"
          />
          <input
            type="text"
            value={newNotes}
            onChange={(e) => setNewNotes(e.target.value)}
            placeholder="메모 (선택)"
          />
          <label className="forbidden-label">
            <input
              type="checkbox"
              checked={newForbidden}
              onChange={(e) => setNewForbidden(e.target.checked)}
            />
            금지어
          </label>
          <div className="form-actions">
            <button className="btn-small" onClick={handleAddTerm}>추가</button>
            <button className="btn-small btn-outline" onClick={() => setShowAddTerm(false)}>취소</button>
          </div>
        </div>
      )}

      {entries.length === 0 ? (
        <p className="no-terms">용어 없음</p>
      ) : (
        entries.map((e) => (
          <div key={e.id} className={`tb-entry ${e.forbidden ? "forbidden" : ""}`}>
            <span className="source-term">{e.sourceTerm}</span>
            <span className="arrow">→</span>
            <span className="target-term">{e.targetTerm}</span>
            {e.forbidden && <span className="forbidden-badge">금지어</span>}
            {e.notes && <span className="term-notes">{e.notes}</span>}
          </div>
        ))
      )}
    </div>
  );
}
