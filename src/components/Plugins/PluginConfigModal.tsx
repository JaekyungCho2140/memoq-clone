import { useState } from "react";
import type { Plugin } from "../../types";

interface PluginConfigModalProps {
  plugin: Plugin;
  onSave(paramValues: Record<string, string>): void;
  onClose(): void;
}

export function PluginConfigModal({ plugin, onSave, onClose }: PluginConfigModalProps) {
  const [values, setValues] = useState<Record<string, string>>(() => {
    return Object.fromEntries(plugin.params.map((p) => [p.key, p.value]));
  });

  const handleChange = (key: string, value: string) => {
    setValues((prev) => ({ ...prev, [key]: value }));
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSave(values);
  };

  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true">
      <div className="modal-box">
        <div className="modal-header">
          <h3 className="modal-title">{plugin.name} — 설정</h3>
          <button className="modal-close" onClick={onClose} aria-label="닫기">
            ×
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="modal-body">
            {plugin.params.length === 0 ? (
              <p className="plugin-no-params">설정 가능한 파라미터가 없습니다.</p>
            ) : (
              plugin.params.map((param) => (
                <div key={param.key} className="plugin-param-row">
                  <label className="plugin-param-label">{param.label}</label>
                  <input
                    className="settings-input"
                    type={param.secret ? "password" : "text"}
                    value={values[param.key] ?? ""}
                    onChange={(e) => handleChange(param.key, e.target.value)}
                    autoComplete={param.secret ? "off" : undefined}
                  />
                </div>
              ))
            )}
          </div>

          <div className="modal-footer">
            <button type="button" className="btn-small btn-outline" onClick={onClose}>
              취소
            </button>
            <button type="submit" className="btn-small">
              저장
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
