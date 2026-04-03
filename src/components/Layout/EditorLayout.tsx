import { SegmentEditor } from "../Editor/SegmentEditor";
import { TmPanel } from "../TmPanel/TmPanel";
import { TbPanel } from "../TbPanel/TbPanel";
import { QaPanel } from "../QaPanel/QaPanel";
import { SegmentList } from "../Editor/SegmentList";
import { Toolbar } from "./Toolbar";

export function EditorLayout() {
  return (
    <div className="editor-root">
      <Toolbar />
      <div className="editor-layout">
        <aside className="segment-list-pane"><SegmentList /></aside>
        <main className="editor-pane"><SegmentEditor /></main>
        <aside className="panel-pane"><TmPanel /><TbPanel /><QaPanel /></aside>
      </div>
    </div>
  );
}
