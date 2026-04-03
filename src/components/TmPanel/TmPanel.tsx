import { useEffect, useState } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { useTmStore } from "../../stores/tmStore";
import { createTm, searchTm, addToTm } from "../../tauri/commands";
import type { TmMatch } from "../../types";

export function TmPanel() {
  const { project, currentSegmentIndex, updateSegment } = useProjectStore();
  const { activeTmId, setActiveTmId } = useTmStore();
  const [matches, setMatches] = useState<TmMatch[]>([]);
  const [creating, setCreating] = useState(false);
  const [tmName, setTmName] = useState("");
  const [adding, setAdding] = useState(false);

  const currentSegment = project?.segments[currentSegmentIndex];

  // Auto-create a TM for the project's language pair if none exists
  useEffect(() => {
    if (!activeTmId && project) {
      const defaultName = `${project.sourceLang} → ${project.targetLang}`;
      createTm(defaultName, project.sourceLang, project.targetLang)
        .then((id) => setActiveTmId(id))
        .catch(() => {/* ignore */});
    }
  }, [project?.id]);

  // Search TM when current segment changes
  useEffect(() => {
    if (!activeTmId || !currentSegment || !project) {
      setMatches([]);
      return;
    }
    searchTm({
      tmId: activeTmId,
      query: currentSegment.source,
      sourceLang: project.sourceLang,
      targetLang: project.targetLang,
      minScore: 0.5,
    })
      .then(setMatches)
      .catch(() => setMatches([]));
  }, [activeTmId, currentSegment?.id, project?.id]);

  const applyMatch = (match: TmMatch) => {
    if (!currentSegment) return;
    updateSegment(currentSegment.id, { target: match.target, status: "draft" });
  };

  const handleCreateTm = async () => {
    if (!tmName.trim() || !project) return;
    try {
      const id = await createTm(tmName.trim(), project.sourceLang, project.targetLang);
      setActiveTmId(id);
      setTmName("");
      setCreating(false);
    } catch (e) {
      console.error("Failed to create TM:", e);
    }
  };

  const handleAddToTm = async () => {
    if (!activeTmId || !currentSegment || !project) return;
    if (!currentSegment.target.trim()) return;
    setAdding(true);
    try {
      await addToTm(
        activeTmId,
        currentSegment.source,
        currentSegment.target,
        project.sourceLang,
        project.targetLang,
      );
      // Refresh matches
      const fresh = await searchTm({
        tmId: activeTmId,
        query: currentSegment.source,
        sourceLang: project.sourceLang,
        targetLang: project.targetLang,
        minScore: 0.5,
      });
      setMatches(fresh);
    } catch (e) {
      console.error("Failed to add to TM:", e);
    } finally {
      setAdding(false);
    }
  };

  return (
    <div className="tm-panel">
      <div className="panel-header">
        <h3>Translation Memory</h3>
        <div className="panel-actions">
          {currentSegment?.target && (
            <button
              className="btn-small"
              onClick={handleAddToTm}
              disabled={adding}
              title="현재 세그먼트를 TM에 추가"
            >
              {adding ? "..." : "+ TM"}
            </button>
          )}
          <button
            className="btn-small btn-outline"
            onClick={() => setCreating((v) => !v)}
            title="새 TM 만들기"
          >
            새 TM
          </button>
        </div>
      </div>

      {creating && (
        <div className="create-form">
          <input
            type="text"
            value={tmName}
            onChange={(e) => setTmName(e.target.value)}
            placeholder="TM 이름"
            onKeyDown={(e) => e.key === "Enter" && handleCreateTm()}
            autoFocus
          />
          <button className="btn-small" onClick={handleCreateTm}>만들기</button>
          <button className="btn-small btn-outline" onClick={() => setCreating(false)}>취소</button>
        </div>
      )}

      {matches.length === 0 ? (
        <p className="no-matches">매치 없음</p>
      ) : (
        matches.map((m, i) => (
          <div key={i} className="tm-match" onClick={() => applyMatch(m)}>
            <div className="match-score">{Math.round(m.score * 100)}%</div>
            <div className="match-source">{m.source}</div>
            <div className="match-target">{m.target}</div>
          </div>
        ))
      )}
    </div>
  );
}
