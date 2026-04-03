import { useProjectStore } from "../../stores/projectStore";

const STATUS_COLORS: Record<string, string> = {
  untranslated: "#e0e0e0",
  draft: "#fff9c4",
  translated: "#c8e6c9",
  confirmed: "#1b5e20",
};

export function SegmentList() {
  const { project, currentSegmentIndex, setCurrentSegmentIndex } = useProjectStore();
  if (!project) return null;
  return (
    <div className="segment-list">
      {project.segments.map((seg, idx) => (
        <div key={seg.id} className={`segment-row ${idx === currentSegmentIndex ? "active" : ""}`}
          onClick={() => setCurrentSegmentIndex(idx)}
          style={{ borderLeft: `4px solid ${STATUS_COLORS[seg.status]}` }}>
          <span className="seg-number">{seg.order + 1}</span>
          <span className="seg-source">{seg.source.slice(0, 80)}</span>
        </div>
      ))}
    </div>
  );
}
