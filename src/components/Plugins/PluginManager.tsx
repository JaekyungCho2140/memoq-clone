import { useEffect, useState } from "react";
import { usePluginStore } from "../../stores/pluginStore";
import type { Plugin } from "../../types";
import { PluginRow } from "./PluginRow";
import { PluginInstallPanel } from "./PluginInstallPanel";
import { PluginConfigModal } from "./PluginConfigModal";

export function PluginManager() {
  const {
    plugins,
    loading,
    error,
    fetchPlugins,
    installPlugin,
    togglePlugin,
    updateParams,
    removePlugin,
    clearError,
  } = usePluginStore();

  const [configTarget, setConfigTarget] = useState<Plugin | null>(null);

  useEffect(() => {
    fetchPlugins();
  }, []);

  const handleInstall = async (req: Parameters<typeof installPlugin>[0]) => {
    try {
      await installPlugin(req);
    } catch {
      // error already surfaced via store.error
    }
  };

  const handleConfigSave = async (paramValues: Record<string, string>) => {
    if (!configTarget) return;
    await updateParams(configTarget.id, paramValues);
    setConfigTarget(null);
  };

  const handleRemove = (id: string) => {
    if (window.confirm("플러그인을 제거하시겠습니까?")) {
      removePlugin(id);
    }
  };

  return (
    <div className="plugin-manager">
      <div className="plugin-manager-header">
        <h4 className="plugin-manager-title">플러그인 관리</h4>
        <button
          className="btn-small btn-outline"
          onClick={() => fetchPlugins()}
          disabled={loading}
          title="목록 새로고침"
        >
          {loading ? "로딩..." : "새로고침"}
        </button>
      </div>

      {error && (
        <div className="plugin-error-banner" role="alert">
          {error}
          <button className="plugin-error-dismiss" onClick={clearError}>×</button>
        </div>
      )}

      <PluginInstallPanel
        installing={loading}
        onInstall={handleInstall}
      />

      <div className="plugin-list">
        {plugins.length === 0 && !loading && (
          <p className="plugin-empty">설치된 플러그인이 없습니다.</p>
        )}
        {plugins.map((plugin) => (
          <PluginRow
            key={plugin.id}
            plugin={plugin}
            onToggle={togglePlugin}
            onConfigure={setConfigTarget}
            onRemove={handleRemove}
          />
        ))}
      </div>

      {configTarget && (
        <PluginConfigModal
          plugin={configTarget}
          onSave={handleConfigSave}
          onClose={() => setConfigTarget(null)}
        />
      )}
    </div>
  );
}
