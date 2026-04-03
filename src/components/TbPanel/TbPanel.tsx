import { useEffect, useState } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { useTbStore } from "../../stores/tbStore";
import { lookupTb } from "../../tauri/commands";
import type { TbEntry } from "../../types";

export function TbPanel() {
  const { project, currentSegmentIndex } = useProjectStore();
  const { activeTbId } = useTbStore();
  const [entries, setEntries] = useState<TbEntry[]>([]);
  const currentSegment = project?.segments[currentSegmentIndex];

  useEffect(() => {
    if (!activeTbId || !currentSegment || !project) { setEntries([]); return; }
    lookupTb({ tbId: activeTbId, term: currentSegment.source, sourceLang: project.sourceLang }).then(setEntries);
  }, [activeTbId, currentSegment?.id, project?.id]);

  return (
    <div className="tb-panel">
      <h3>Term Base</h3>
      {entries.length === 0 ? <p className="no-terms">용어 없음</p> : entries.map((e) => (
        <div key={e.id} className={`tb-entry ${e.forbidden ? "forbidden" : ""}`}>
          <span className="source-term">{e.sourceTerm}</span>
          <span className="arrow">→</span>
          <span className="target-term">{e.targetTerm}</span>
          {e.forbidden && <span className="forbidden-badge">금지어</span>}
        </div>
      ))}
    </div>
  );
}
