import { useState } from "react";
import { useProjectStore } from "../../stores/projectStore";
import { PluginManager } from "../Plugins/PluginManager";

const LANGUAGES = [
  { code: "ko", label: "한국어" },
  { code: "en", label: "영어" },
  { code: "ja", label: "일본어" },
  { code: "zh", label: "중국어" },
  { code: "de", label: "독일어" },
  { code: "fr", label: "프랑스어" },
  { code: "es", label: "스페인어" },
];

interface Props {
  onClose: () => void;
}

export function ProjectSettingsModal({ onClose }: Props) {
  const project = useProjectStore((s) => s.project);

  const [sourceLang, setSourceLang] = useState(project?.sourceLang ?? "en");
  const [targetLang, setTargetLang] = useState(project?.targetLang ?? "ko");
  const [projectName, setProjectName] = useState(project?.name ?? "");

  if (!project) return null;

  const handleSave = () => {
    // Language pair and name updates will be handled by the store/backend in AFR-24.
    // For now, this modal is display-only for language pair inspection.
    onClose();
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2 className="modal-title">프로젝트 설정</h2>
          <button className="modal-close-btn" onClick={onClose}>✕</button>
        </div>

        <div className="modal-body">
          <section className="settings-section">
            <h3 className="settings-section-title">기본 정보</h3>
            <label className="settings-label">
              프로젝트 이름
              <input
                className="settings-input"
                type="text"
                value={projectName}
                onChange={(e) => setProjectName(e.target.value)}
              />
            </label>
          </section>

          <section className="settings-section">
            <h3 className="settings-section-title">언어 쌍</h3>
            <div className="settings-lang-row">
              <label className="settings-label">
                원본 언어
                <select
                  className="settings-select"
                  value={sourceLang}
                  onChange={(e) => setSourceLang(e.target.value)}
                >
                  {LANGUAGES.map((l) => (
                    <option key={l.code} value={l.code}>
                      {l.label} ({l.code})
                    </option>
                  ))}
                </select>
              </label>
              <span className="settings-arrow">→</span>
              <label className="settings-label">
                번역 언어
                <select
                  className="settings-select"
                  value={targetLang}
                  onChange={(e) => setTargetLang(e.target.value)}
                >
                  {LANGUAGES.map((l) => (
                    <option key={l.code} value={l.code}>
                      {l.label} ({l.code})
                    </option>
                  ))}
                </select>
              </label>
            </div>
          </section>

          <section className="settings-section">
            <h3 className="settings-section-title">리소스 연결</h3>
            <p className="settings-desc">
              TM (번역 메모리) 및 TB (용어집) 연결은 AFR-24 완료 후 활성화됩니다.
            </p>
            <div className="settings-resource-placeholder">
              <span>🔗 TM/TB 연결 설정 (준비 중)</span>
            </div>
          </section>

          <section className="settings-section">
            <PluginManager />
          </section>
        </div>

        <div className="modal-footer">
          <button className="pd-btn pd-btn-secondary" onClick={onClose}>
            취소
          </button>
          <button className="pd-btn pd-btn-primary" onClick={handleSave}>
            저장
          </button>
        </div>
      </div>
    </div>
  );
}
