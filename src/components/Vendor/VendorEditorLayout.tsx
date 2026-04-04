import { useState } from "react";
import { useVendorStore } from "../../stores/vendorStore";
import { SegmentEditor } from "../Editor/SegmentEditor";
import { SegmentList } from "../Editor/SegmentList";
import { TmPanel } from "../TmPanel/TmPanel";
import { QaPanel } from "../QaPanel/QaPanel";
import { VendorDeliveryModal } from "./VendorDeliveryModal";
import { useProjectStore } from "../../stores/projectStore";

/**
 * Simplified editor for vendors.
 * Differs from the full EditorLayout:
 * - No TbPanel (term base management)
 * - No MtSettings (machine translation admin)
 * - No LiveDocs manager
 * - No project open/save controls
 * - Delivery submit button always visible in toolbar
 */
export function VendorEditorLayout() {
  const { activeAssignment, closeAssignment } = useVendorStore();
  const { backToDashboard } = useProjectStore();
  const [showDelivery, setShowDelivery] = useState(false);

  const handleBack = () => {
    backToDashboard();
    closeAssignment();
  };

  return (
    <div className="editor-root vendor-editor">
      {/* Simplified vendor toolbar */}
      <div className="toolbar vendor-toolbar">
        <button className="toolbar-btn" onClick={handleBack}>
          ← 대시보드
        </button>
        {activeAssignment && (
          <span className="vendor-toolbar-title">
            {activeAssignment.projectName} — {activeAssignment.fileName}
          </span>
        )}
        <div className="toolbar-spacer" />
        {activeAssignment && (
          <button
            className="btn-primary toolbar-btn-deliver"
            onClick={() => setShowDelivery(true)}
          >
            납품 제출
          </button>
        )}
      </div>

      <div className="editor-layout">
        <aside className="segment-list-pane">
          <SegmentList />
        </aside>
        <main className="editor-pane">
          <SegmentEditor />
        </main>
        <aside className="panel-pane vendor-panel-pane">
          <TmPanel />
          <QaPanel />
        </aside>
      </div>

      {showDelivery && activeAssignment && (
        <VendorDeliveryModal
          assignment={activeAssignment}
          onCancel={() => setShowDelivery(false)}
        />
      )}
    </div>
  );
}
