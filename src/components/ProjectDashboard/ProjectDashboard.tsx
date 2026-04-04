import { useState, useCallback } from "react";
import { adapter, fileRefFromDrop } from "../../adapters";
import { useProjectStore } from "../../stores/projectStore";
import { ProjectSettingsModal } from "./ProjectSettingsModal";
import type { Segment } from "../../types";

function computeStats(segments: Segment[]) {
  const total = segments.length;
  if (total === 0) return { total: 0, translated: 0, confirmed: 0, rate: 0 };
  const translated = segments.filter(
    (s) => s.status === "translated" || s.status === "confirmed",
  ).length;
  const confirmed = segments.filter((s) => s.status === "confirmed").length;
  return { total, translated, confirmed, rate: Math.round((translated / total) * 100) };
}

export function ProjectDashboard() {
  const { project, closeProject, setCurrentSegmentIndex } = useProjectStore();
  const openEditor = useProjectStore((s) => s.openEditor);
  const [isDragOver, setIsDragOver] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [saving, setSaving] = useState(false);

  if (!project) return null;

  const stats = computeStats(project.segments);
  const fileName = project.sourcePath.split(/[\\/]/).pop() ?? project.name;

  const handleOpenInEditor = () => {
    setCurrentSegmentIndex(0);
    openEditor();
  };

  const handleAddFile = async () => {
    const fileRef = await adapter.openFileDialog({
      multiple: false,
      filters: [{ name: "Translation Files", extensions: ["xliff", "xlf", "docx"] }],
    });
    if (!fileRef) return;
    try {
      const updated = await adapter.addFileToProject(project, fileRef);
      useProjectStore.getState().setProject(updated);
    } catch (e) {
      alert(`파일 추가 실패: ${e}`);
    }
  };

  const handleDrop = useCallback(
    async (e: React.DragEvent<HTMLDivElement>) => {
      e.preventDefault();
      setIsDragOver(false);
      const files = Array.from(e.dataTransfer.files);
      const supported = files.filter((f) =>
        /\.(xliff|xlf|docx)$/i.test(f.name),
      );
      if (supported.length === 0) {
        alert("지원하지 않는 파일 형식입니다. XLIFF 또는 DOCX 파일을 사용하세요.");
        return;
      }
      let currentProject = useProjectStore.getState().project;
      if (!currentProject) return;
      for (const file of supported) {
        const fileRef = fileRefFromDrop(file);
        try {
          currentProject = await adapter.addFileToProject(currentProject, fileRef);
        } catch (err) {
          alert(`파일 추가 실패 (${file.name}): ${err}`);
        }
      }
      useProjectStore.getState().setProject(currentProject);
    },
    [],
  );

  const handleSaveProject = async () => {
    const outputPath = await adapter.saveFileDialog({
      filters: [{ name: "memoQ Clone Project", extensions: ["mqclone"] }],
      defaultPath: `${project.name}.mqclone`,
    });
    if (!outputPath) return;
    setSaving(true);
    try {
      await adapter.saveProject(project, outputPath);
      alert("프로젝트가 저장되었습니다.");
    } catch (e) {
      alert(`프로젝트 저장 실패: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="project-dashboard">
      {/* Dashboard Header */}
      <div className="pd-header">
        <div className="pd-header-left">
          <button className="pd-back-btn" onClick={closeProject} title="홈으로">
            ← 홈
          </button>
          <div className="pd-project-info">
            <h1 className="pd-project-name">{project.name}</h1>
            <span className="pd-lang-pair">
              {project.sourceLang} → {project.targetLang}
            </span>
          </div>
        </div>
        <div className="pd-header-right">
          <button
            className="pd-btn pd-btn-secondary"
            onClick={() => setShowSettings(true)}
          >
            ⚙ 설정
          </button>
          <button
            className="pd-btn pd-btn-secondary"
            onClick={handleSaveProject}
            disabled={saving}
          >
            💾 저장
          </button>
          <button
            className="pd-btn pd-btn-primary"
            data-testid="open-editor-btn"
            onClick={handleOpenInEditor}
          >
            ✏ 번역 편집기 열기
          </button>
        </div>
      </div>

      {/* Overall Progress */}
      <div className="pd-progress-section">
        <div className="pd-progress-header">
          <span className="pd-progress-label">전체 진행률</span>
          <span className="pd-progress-pct">{stats.rate}%</span>
          <span className="pd-progress-counts">
            {stats.translated} / {stats.total} 세그먼트 번역 완료 ({stats.confirmed} 확정)
          </span>
        </div>
        <div className="pd-progress-bar">
          <div
            className="pd-progress-fill"
            style={{ width: `${stats.rate}%` }}
          />
        </div>
      </div>

      {/* File List */}
      <div className="pd-file-section">
        <div className="pd-file-section-header">
          <h2 className="pd-section-title">파일 목록</h2>
          <button className="pd-btn pd-btn-secondary pd-btn-sm" onClick={handleAddFile}>
            + 파일 추가
          </button>
        </div>

        {/* Drop Zone + File Table */}
        <div
          className={`pd-file-table-wrapper ${isDragOver ? "drag-over" : ""}`}
          onDragOver={(e) => { e.preventDefault(); setIsDragOver(true); }}
          onDragLeave={() => setIsDragOver(false)}
          onDrop={handleDrop}
        >
          {isDragOver && (
            <div className="pd-drop-overlay">
              <span>파일을 여기에 놓으세요</span>
            </div>
          )}
          <table className="pd-file-table">
            <thead>
              <tr>
                <th>파일명</th>
                <th className="col-narrow">세그먼트</th>
                <th className="col-narrow">완료</th>
                <th className="col-wide">진행률</th>
                <th className="col-narrow">작업</th>
              </tr>
            </thead>
            <tbody>
              {(project.files ?? []).length > 0 ? (
                (project.files ?? []).map((file) => (
                  <FileRow
                    key={file.id}
                    fileName={file.path.split(/[\\/]/).pop() ?? file.path}
                    segments={file.segments}
                    onEdit={handleOpenInEditor}
                  />
                ))
              ) : (
                <FileRow
                  fileName={fileName}
                  segments={project.segments}
                  onEdit={handleOpenInEditor}
                />
              )}
            </tbody>
          </table>
          <div className="pd-drop-hint">
            XLIFF / DOCX 파일을 여기에 드래그&드롭하여 추가할 수 있습니다
          </div>
        </div>
      </div>

      {showSettings && (
        <ProjectSettingsModal onClose={() => setShowSettings(false)} />
      )}
    </div>
  );
}

interface FileRowProps {
  fileName: string;
  segments: Segment[];
  onEdit: () => void;
}

function FileRow({ fileName, segments, onEdit }: FileRowProps) {
  const stats = computeStats(segments);
  return (
    <tr className="pd-file-row">
      <td className="pd-file-name">
        <span className="pd-file-icon">📄</span>
        {fileName}
      </td>
      <td className="col-narrow pd-file-count">{stats.total}</td>
      <td className="col-narrow pd-file-done">{stats.translated}</td>
      <td className="col-wide">
        <div className="pd-file-progress-bar">
          <div
            className="pd-file-progress-fill"
            style={{ width: `${stats.rate}%` }}
          />
        </div>
        <span className="pd-file-pct">{stats.rate}%</span>
      </td>
      <td className="col-narrow">
        <button className="pd-btn pd-btn-sm pd-btn-primary" onClick={onEdit}>
          편집
        </button>
      </td>
    </tr>
  );
}
