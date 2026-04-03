import { useEffect, useState } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { useTmStore } from "../../stores/tmStore";
import { searchTm } from "../../tauri/commands";
import type { TmMatch } from "../../types";

export function TmPanel() {
  const { project, currentSegmentIndex, updateSegment } = useProjectStore();
  const { activeTmId } = useTmStore();
  const [matches, setMatches] = useState<TmMatch[]>([]);
  const currentSegment = project?.segments[currentSegmentIndex];

  useEffect(() => {
    if (!activeTmId || !currentSegment || !project) { setMatches([]); return; }
    searchTm({ tmId: activeTmId, query: currentSegment.source, sourceLang: project.sourceLang, targetLang: project.targetLang, minScore: 0.5 }).then(setMatches);
  }, [activeTmId, currentSegment?.id, project?.id]);

  const applyMatch = (match: TmMatch) => {
    if (!currentSegment) return;
    updateSegment(currentSegment.id, { target: match.target, status: "draft" });
  };

  return (
    <div className="tm-panel">
      <h3>Translation Memory</h3>
      {matches.length === 0 ? <p className="no-matches">매치 없음</p> : matches.map((m, i) => (
        <div key={i} className="tm-match" onClick={() => applyMatch(m)}>
          <div className="match-score">{Math.round(m.score * 100)}%</div>
          <div className="match-source">{m.source}</div>
          <div className="match-target">{m.target}</div>
        </div>
      ))}
    </div>
  );
}
