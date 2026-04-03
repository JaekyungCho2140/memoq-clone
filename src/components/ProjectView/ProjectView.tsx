import { open } from "@tauri-apps/plugin-dialog";
import { parseFile } from "../../tauri/commands";
import { useProjectStore } from "../../stores/projectStore";

export function ProjectView() {
  const setProject = useProjectStore((s) => s.setProject);

  const handleOpenFile = async () => {
    const selected = await open({ multiple: false, filters: [{ name: "Translation Files", extensions: ["xliff", "xlf", "docx"] }] });
    if (!selected || typeof selected !== "string") return;
    const project = await parseFile(selected);
    setProject(project);
  };

  return (
    <div className="project-view">
      <h1>memoQ Clone</h1>
      <p>번역 파일을 열어 작업을 시작하세요.</p>
      <button className="open-file-btn" onClick={handleOpenFile}>파일 열기 (XLIFF / DOCX)</button>
    </div>
  );
}
