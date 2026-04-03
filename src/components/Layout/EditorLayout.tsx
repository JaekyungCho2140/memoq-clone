import { SegmentEditor } from "../Editor/SegmentEditor";
import { TmPanel } from "../TmPanel/TmPanel";
import { TbPanel } from "../TbPanel/TbPanel";
import { SegmentList } from "../Editor/SegmentList";

export function EditorLayout() {
  return (
    <div className="editor-layout">
      <aside className="segment-list-pane"><SegmentList /></aside>
      <main className="editor-pane"><SegmentEditor /></main>
      <aside className="panel-pane"><TmPanel /><TbPanel /></aside>
    </div>
  );
}
