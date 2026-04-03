import { save } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "../../stores/projectStore";
import { exportFile } from "../../tauri/commands";

export function Toolbar() {
  const { project, closeProject } = useProjectStore();
  if (!project) return null;

  const translated = project.segments.filter((s) => s.status !== "untranslated").length;
  const total = project.segments.length;
  const progress = total > 0 ? Math.round((translated / total) * 100) : 0;

  const handleExport = async () => {
    const ext = project.sourcePath.endsWith(".docx") ? "docx" : "xliff";
    const outputPath = await save({
      defaultPath: `translated.${ext}`,
      filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
    });
    if (!outputPath) return;
    try {
      await exportFile(project.segments, project.sourcePath, outputPath);
      alert(`내보내기 완료: ${outputPath}`);
    } catch (e) {
      alert(`내보내기 실패: ${e}`);
    }
  };

  return (
    <div className="toolbar">
      <div className="toolbar-left">
        <span className="project-name" title={project.sourcePath}>{project.name}</span>
        <span className="lang-pair">{project.sourceLang} → {project.targetLang}</span>
        <span className="progress-badge">{translated}/{total} ({progress}%)</span>
      </div>
      <div className="toolbar-right">
        <button className="btn-toolbar" onClick={handleExport}>내보내기</button>
        <button className="btn-toolbar btn-danger" onClick={closeProject}>닫기</button>
      </div>
    </div>
  );
}
