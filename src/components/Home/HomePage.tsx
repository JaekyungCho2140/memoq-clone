import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { parseFile, getRecentProjects, loadProject } from "../../tauri/commands";
import { useProjectStore } from "../../stores/projectStore";

export function HomePage() {
  const [recentProjects, setRecentProjects] = useState<string[]>([]);
  const [loadingRecent, setLoadingRecent] = useState(true);
  const setProject = useProjectStore((s) => s.setProject);

  useEffect(() => {
    getRecentProjects()
      .then(setRecentProjects)
      .catch(() => setRecentProjects([]))
      .finally(() => setLoadingRecent(false));
  }, []);

  const handleOpenFile = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Translation Files", extensions: ["xliff", "xlf", "docx"] }],
    });
    if (!selected || typeof selected !== "string") return;
    const project = await parseFile(selected);
    setProject(project);
  };

  const handleOpenProject = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "memoQ Clone Project", extensions: ["mqclone"] }],
    });
    if (!selected || typeof selected !== "string") return;
    try {
      const project = await loadProject(selected);
      setProject(project);
    } catch (e) {
      alert(`프로젝트 파일을 불러오는 데 실패했습니다: ${e}`);
    }
  };

  const handleOpenRecent = async (projectPath: string) => {
    try {
      const project = await loadProject(projectPath);
      setProject(project);
    } catch (e) {
      alert(`프로젝트를 불러오는 데 실패했습니다: ${e}`);
    }
  };

  return (
    <div className="home-page">
      <div className="home-header">
        <div className="home-logo">
          <span className="home-logo-icon">⚡</span>
          <span className="home-logo-text">memoQ Clone</span>
        </div>
        <p className="home-subtitle">전문 번역 도구</p>
      </div>

      <div className="home-actions">
        <button className="home-action-btn home-action-primary" onClick={handleOpenFile}>
          <span className="home-action-icon">📄</span>
          <span className="home-action-label">번역 파일 열기</span>
          <span className="home-action-desc">XLIFF, DOCX 파일</span>
        </button>

        <button className="home-action-btn home-action-secondary" onClick={handleOpenProject}>
          <span className="home-action-icon">📁</span>
          <span className="home-action-label">프로젝트 열기</span>
          <span className="home-action-desc">.mqclone 프로젝트 파일</span>
        </button>
      </div>

      <div className="home-recent">
        <h2 className="home-section-title">최근 프로젝트</h2>
        {loadingRecent ? (
          <p className="home-empty">불러오는 중...</p>
        ) : recentProjects.length === 0 ? (
          <p className="home-empty">최근 열었던 프로젝트가 없습니다.</p>
        ) : (
          <ul className="recent-project-list">
            {recentProjects.map((rp) => (
              <RecentProjectCard
                key={rp}
                path={rp}
                onOpen={() => handleOpenRecent(rp)}
              />
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

interface RecentProjectCardProps {
  path: string;
  onOpen: () => void;
}

function RecentProjectCard({ path, onOpen }: RecentProjectCardProps) {
  const fileName = path.split(/[\\/]/).pop() ?? path;

  return (
    <li className="recent-project-card" onClick={onOpen}>
      <div className="rpc-icon">📋</div>
      <div className="rpc-info">
        <span className="rpc-name">{fileName}</span>
        <span className="rpc-path">{path}</span>
      </div>
      <button className="rpc-open-btn" onClick={(e) => { e.stopPropagation(); onOpen(); }}>
        열기
      </button>
    </li>
  );
}
