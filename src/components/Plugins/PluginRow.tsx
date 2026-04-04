import type { Plugin } from "../../types";

interface PluginRowProps {
  plugin: Plugin;
  onToggle(id: string, enabled: boolean): void;
  onConfigure(plugin: Plugin): void;
  onRemove(id: string): void;
}

const KIND_LABEL: Record<string, string> = {
  MtProvider: "MT 프로바이더",
  FileParser: "파일 파서",
  QaRule: "QA 규칙",
};

export function PluginRow({ plugin, onToggle, onConfigure, onRemove }: PluginRowProps) {
  return (
    <div className={`plugin-row${plugin.error ? " plugin-row--error" : ""}`}>
      <div className="plugin-row-info">
        <span className="plugin-row-name">{plugin.name}</span>
        <span className="plugin-row-version">v{plugin.version}</span>
        <span className="plugin-row-kind">{KIND_LABEL[plugin.kind] ?? plugin.kind}</span>
        {plugin.error && (
          <span className="plugin-row-errtext" title={plugin.error}>
            ⚠ {plugin.error}
          </span>
        )}
      </div>
      <div className="plugin-row-actions">
        <label className="plugin-toggle" title={plugin.enabled ? "비활성화" : "활성화"}>
          <input
            type="checkbox"
            checked={plugin.enabled}
            onChange={(e) => onToggle(plugin.id, e.target.checked)}
          />
          <span className="plugin-toggle-slider" />
        </label>
        <button
          className="btn-small btn-outline"
          onClick={() => onConfigure(plugin)}
          title="설정"
        >
          설정
        </button>
        <button
          className="btn-small btn-outline btn-danger"
          onClick={() => onRemove(plugin.id)}
          title="제거"
        >
          제거
        </button>
      </div>
    </div>
  );
}
