import { useEffect, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "../../stores/projectStore";
import { useQaStore } from "../../stores/qaStore";
import { exportFile, runQaCheck } from "../../tauri/commands";

export function Toolbar() {
  const { project, closeProject } = useProjectStore();
  const { setIssues, setRunning, errorCount } = useQaStore();
  const [showExportWarning, setShowExportWarning] = useState(false);
  const [pendingExportPath, setPendingExportPath] = useState<string | null>(null);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "F9" && project) {
        e.preventDefault();
        handleRunQa();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [project]);

  if (!project) return null;

  const translated = project.segments.filter((s) => s.status !== "untranslated").length;
  const total = project.segments.length;
  const progress = total > 0 ? Math.round((translated / total) * 100) : 0;
  const errors = errorCount();

  const handleRunQa = async () => {
    setRunning(true);
    try {
      const issues = await runQaCheck(project.id);
      setIssues(issues);
    } catch {
      setIssues([]);
    } finally {
      setRunning(false);
    }
  };

  const doExport = async (outputPath: string) => {
    try {
      await exportFile(project.segments, project.sourcePath, outputPath);
      alert(`내보내기 완료: ${outputPath}`);
    } catch (e) {
      alert(`내보내기 실패: ${e}`);
    }
  };

  const handleExport = async () => {
    const ext = project.sourcePath.endsWith(".docx") ? "docx" : "xliff";
    const outputPath = await save({
      defaultPath: `translated.${ext}`,
      filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
    });
    if (!outputPath) return;

    if (errors > 0) {
      setPendingExportPath(outputPath);
      setShowExportWarning(true);
    } else {
      await doExport(outputPath);
    }
  };

  const handleExportConfirm = async () => {
    setShowExportWarning(false);
    if (pendingExportPath) {
      await doExport(pendingExportPath);
      setPendingExportPath(null);
    }
  };

  const handleExportCancel = () => {
    setShowExportWarning(false);
    setPendingExportPath(null);
  };

  return (
    <>
      <div className="toolbar">
        <div className="toolbar-left">
          <span className="project-name" title={project.sourcePath}>{project.name}</span>
          <span className="lang-pair">{project.sourceLang} → {project.targetLang}</span>
          <span className="progress-badge">{translated}/{total} ({progress}%)</span>
          {errors > 0 && (
            <span className="qa-error-badge">{errors}개 오류</span>
          )}
        </div>
        <div className="toolbar-right">
          <button className="btn-toolbar btn-qa" onClick={handleRunQa} title="QA 체크 실행 (F9)">
            QA 체크 (F9)
          </button>
          <button className="btn-toolbar" onClick={handleExport}>내보내기</button>
          <button className="btn-toolbar btn-danger" onClick={closeProject}>닫기</button>
        </div>
      </div>

      {showExportWarning && (
        <div className="modal-overlay">
          <div className="modal-dialog">
            <h3 className="modal-title">⚠ QA 오류가 있습니다</h3>
            <p className="modal-body">
              {errors}개의 QA 오류가 있습니다. 그래도 내보내시겠습니까?
            </p>
            <div className="modal-actions">
              <button className="btn-toolbar btn-danger" onClick={handleExportConfirm}>
                계속 내보내기
              </button>
              <button className="btn-toolbar btn-outline" onClick={handleExportCancel}>
                취소
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
