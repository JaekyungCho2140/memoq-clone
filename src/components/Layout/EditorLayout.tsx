import { useState } from "react";
import { SegmentEditor } from "../Editor/SegmentEditor";
import { TmPanel } from "../TmPanel/TmPanel";
import { TbPanel } from "../TbPanel/TbPanel";
import { QaPanel } from "../QaPanel/QaPanel";
import { SegmentList } from "../Editor/SegmentList";
import { Toolbar } from "./Toolbar";
import { MtSettings } from "../Settings/MtSettings";
import { LiveDocsManager } from "../LiveDocs/LiveDocsManager";

export function EditorLayout() {
  const [showLiveDocs, setShowLiveDocs] = useState(false);

  return (
    <div className="editor-root">
      <Toolbar />
      <div className="editor-layout">
        <aside className="segment-list-pane"><SegmentList /></aside>
        <main className="editor-pane"><SegmentEditor /></main>
        <aside className="panel-pane">
          <TmPanel />
          <TbPanel />
          <QaPanel />
          <MtSettings />
          <div className="livedocs-entry">
            <button
              className="btn-small btn-outline"
              onClick={() => setShowLiveDocs((v) => !v)}
            >
              {showLiveDocs ? "▲ LiveDocs 닫기" : "▼ LiveDocs 라이브러리"}
            </button>
            {showLiveDocs && <LiveDocsManager />}
          </div>
        </aside>
      </div>
    </div>
  );
}
