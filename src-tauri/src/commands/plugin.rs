//! Tauri commands for plugin management.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::State;

use crate::plugin::{PluginEntry, PluginRegistry, PluginRuntime};
use plugin_api::{MtRequest, MtResponse, PluginKind, QaRequest, QaResponse};

// ──────────────────────────────────────────────
// State
// ──────────────────────────────────────────────

pub struct PluginState(pub Mutex<PluginRegistry>);

// ──────────────────────────────────────────────
// Response types
// ──────────────────────────────────────────────

#[derive(Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub kind: String,
    pub enabled: bool,
    pub wasm_path: String,
}

impl From<&PluginEntry> for PluginInfo {
    fn from(e: &PluginEntry) -> Self {
        PluginInfo {
            id: e.manifest.id.clone(),
            name: e.manifest.name.clone(),
            version: e.manifest.version.clone(),
            author: e.manifest.author.clone(),
            description: e.manifest.description.clone(),
            kind: match e.manifest.kind {
                PluginKind::MtProvider => "mt_provider".into(),
                PluginKind::FileParser => "file_parser".into(),
                PluginKind::QaRule => "qa_rule".into(),
            },
            enabled: e.manifest.enabled,
            wasm_path: e.wasm_path.to_string_lossy().into(),
        }
    }
}

// ──────────────────────────────────────────────
// Commands
// ──────────────────────────────────────────────

/// List all installed plugins.
#[tauri::command]
pub fn plugin_list(state: State<PluginState>) -> Result<Vec<PluginInfo>, String> {
    let reg = state.0.lock().map_err(|e| e.to_string())?;
    Ok(reg.all().iter().map(|e| PluginInfo::from(*e)).collect())
}

/// Scan (or re-scan) the plugins directory and refresh the registry.
#[tauri::command]
pub fn plugin_scan(
    state: State<PluginState>,
    plugins_dir: String,
) -> Result<Vec<PluginInfo>, String> {
    let mut reg = state.0.lock().map_err(|e| e.to_string())?;
    let path = PathBuf::from(&plugins_dir);
    reg.scan(&path).map_err(|e| e.to_string())?;
    Ok(reg.all().iter().map(|e| PluginInfo::from(*e)).collect())
}

/// Enable or disable a plugin.
#[tauri::command]
pub fn plugin_set_enabled(
    state: State<PluginState>,
    plugin_id: String,
    enabled: bool,
) -> Result<(), String> {
    let mut reg = state.0.lock().map_err(|e| e.to_string())?;
    reg.set_enabled(&plugin_id, enabled)
        .map_err(|e| e.to_string())
}

/// Run an MT translation through the named plugin.
#[tauri::command]
pub fn plugin_mt_translate(
    state: State<PluginState>,
    plugin_id: String,
    source_lang: String,
    target_lang: String,
    segments: Vec<String>,
) -> Result<MtResponse, String> {
    let wasm_path = {
        let reg = state.0.lock().map_err(|e| e.to_string())?;
        let entry = reg
            .get(&plugin_id)
            .ok_or_else(|| format!("plugin '{}' not found", plugin_id))?;
        if !entry.manifest.enabled {
            return Err(format!("plugin '{}' is disabled", plugin_id));
        }
        entry.wasm_path.clone()
    };

    let rt = PluginRuntime::from_file(&wasm_path).map_err(|e| e.to_string())?;
    let req = MtRequest {
        source_lang,
        target_lang,
        segments,
    };
    rt.mt_translate(req).map_err(|e| e.to_string())
}

/// Run a QA check through the named plugin.
#[tauri::command]
pub fn plugin_qa_check(
    state: State<PluginState>,
    plugin_id: String,
    segments: Vec<plugin_api::QaSegment>,
) -> Result<QaResponse, String> {
    let wasm_path = {
        let reg = state.0.lock().map_err(|e| e.to_string())?;
        let entry = reg
            .get(&plugin_id)
            .ok_or_else(|| format!("plugin '{}' not found", plugin_id))?;
        if !entry.manifest.enabled {
            return Err(format!("plugin '{}' is disabled", plugin_id));
        }
        entry.wasm_path.clone()
    };

    let rt = PluginRuntime::from_file(&wasm_path).map_err(|e| e.to_string())?;
    let req = QaRequest { segments };
    rt.qa_check(req).map_err(|e| e.to_string())
}
