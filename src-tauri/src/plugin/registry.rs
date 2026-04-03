//! Plugin registry — discovers, loads and tracks installed plugins.
//!
//! Plugins live in `<app_data_dir>/plugins/<plugin-id>/`:
//!
//! ```text
//! plugins/
//!   com.example.deepl-mt/
//!     manifest.json
//!     plugin.wasm
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use plugin_api::{PluginKind, PluginMetadata};

/// Contents of a plugin's `manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub kind: PluginKind,
    /// Relative path to the WASM binary (default: `"plugin.wasm"`).
    #[serde(default = "default_wasm_file")]
    pub wasm_file: String,
    #[serde(default)]
    pub enabled: bool,
}

fn default_wasm_file() -> String {
    "plugin.wasm".into()
}

impl From<&PluginManifest> for PluginMetadata {
    fn from(m: &PluginManifest) -> Self {
        PluginMetadata {
            id: m.id.clone(),
            name: m.name.clone(),
            version: m.version.clone(),
            author: m.author.clone(),
            description: m.description.clone(),
            kind: m.kind,
        }
    }
}

/// A discovered plugin entry.
#[derive(Debug, Clone)]
pub struct PluginEntry {
    pub manifest: PluginManifest,
    pub wasm_path: PathBuf,
}

/// In-memory plugin registry.
#[derive(Debug, Default)]
pub struct PluginRegistry {
    entries: HashMap<String, PluginEntry>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Scan `plugins_dir` for installed plugins and populate the registry.
    pub fn scan(&mut self, plugins_dir: &Path) -> Result<()> {
        if !plugins_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(plugins_dir)
            .with_context(|| format!("reading plugins dir {:?}", plugins_dir))?
        {
            let entry = entry?;
            let plugin_dir = entry.path();
            if !plugin_dir.is_dir() {
                continue;
            }

            match self.load_plugin_dir(&plugin_dir) {
                Ok(plugin) => {
                    self.entries.insert(plugin.manifest.id.clone(), plugin);
                }
                Err(e) => {
                    log::warn!("skipping {:?}: {}", plugin_dir, e);
                }
            }
        }

        Ok(())
    }

    fn load_plugin_dir(&self, dir: &Path) -> Result<PluginEntry> {
        let manifest_path = dir.join("manifest.json");
        let raw = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("reading {:?}", manifest_path))?;
        let manifest: PluginManifest =
            serde_json::from_str(&raw).with_context(|| "parsing manifest.json")?;
        let wasm_path = dir.join(&manifest.wasm_file);
        if !wasm_path.exists() {
            anyhow::bail!("WASM file {:?} not found", wasm_path);
        }
        Ok(PluginEntry {
            manifest,
            wasm_path,
        })
    }

    /// Register a plugin entry directly (useful for testing).
    pub fn insert(&mut self, entry: PluginEntry) {
        self.entries.insert(entry.manifest.id.clone(), entry);
    }

    /// Enable or disable a plugin.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<()> {
        let entry = self
            .entries
            .get_mut(id)
            .with_context(|| format!("plugin '{}' not found", id))?;
        entry.manifest.enabled = enabled;
        Ok(())
    }

    /// Return all registered plugins (enabled and disabled).
    pub fn all(&self) -> Vec<&PluginEntry> {
        self.entries.values().collect()
    }

    /// Return only enabled plugins of the given kind.
    pub fn enabled_of_kind(&self, kind: PluginKind) -> Vec<&PluginEntry> {
        self.entries
            .values()
            .filter(|e| e.manifest.enabled && e.manifest.kind == kind)
            .collect()
    }

    /// Look up a plugin by id.
    pub fn get(&self, id: &str) -> Option<&PluginEntry> {
        self.entries.get(id)
    }

    /// Remove a plugin from the registry (does not delete files).
    pub fn remove(&mut self, id: &str) -> Option<PluginEntry> {
        self.entries.remove(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_plugin_dir(parent: &Path, id: &str, kind: PluginKind, enabled: bool) -> PathBuf {
        let dir = parent.join(id);
        std::fs::create_dir_all(&dir).unwrap();

        let manifest = PluginManifest {
            id: id.into(),
            name: format!("{} Plugin", id),
            version: "0.1.0".into(),
            author: "Test".into(),
            description: "desc".into(),
            kind,
            wasm_file: "plugin.wasm".into(),
            enabled,
        };
        let json = serde_json::to_string(&manifest).unwrap();
        std::fs::write(dir.join("manifest.json"), json).unwrap();
        // create dummy wasm file
        std::fs::write(dir.join("plugin.wasm"), b"\0asm\x01\0\0\0").unwrap();
        dir
    }

    #[test]
    fn scan_finds_plugins() {
        let tmp = TempDir::new().unwrap();
        make_plugin_dir(tmp.path(), "com.test.mt1", PluginKind::MtProvider, true);
        make_plugin_dir(tmp.path(), "com.test.qa1", PluginKind::QaRule, false);

        let mut reg = PluginRegistry::new();
        reg.scan(tmp.path()).unwrap();

        assert_eq!(reg.all().len(), 2);
    }

    #[test]
    fn enabled_of_kind_filters_correctly() {
        let tmp = TempDir::new().unwrap();
        make_plugin_dir(tmp.path(), "com.test.mt1", PluginKind::MtProvider, true);
        make_plugin_dir(tmp.path(), "com.test.mt2", PluginKind::MtProvider, false);
        make_plugin_dir(tmp.path(), "com.test.qa1", PluginKind::QaRule, true);

        let mut reg = PluginRegistry::new();
        reg.scan(tmp.path()).unwrap();

        let mt = reg.enabled_of_kind(PluginKind::MtProvider);
        assert_eq!(mt.len(), 1);
        assert_eq!(mt[0].manifest.id, "com.test.mt1");

        let qa = reg.enabled_of_kind(PluginKind::QaRule);
        assert_eq!(qa.len(), 1);
    }

    #[test]
    fn set_enabled_toggles_state() {
        let tmp = TempDir::new().unwrap();
        make_plugin_dir(tmp.path(), "com.test.p1", PluginKind::MtProvider, false);

        let mut reg = PluginRegistry::new();
        reg.scan(tmp.path()).unwrap();

        assert_eq!(reg.enabled_of_kind(PluginKind::MtProvider).len(), 0);
        reg.set_enabled("com.test.p1", true).unwrap();
        assert_eq!(reg.enabled_of_kind(PluginKind::MtProvider).len(), 1);
    }

    #[test]
    fn scan_skips_missing_wasm() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("bad-plugin");
        std::fs::create_dir_all(&dir).unwrap();
        // manifest points to non-existent wasm
        let manifest = PluginManifest {
            id: "bad".into(),
            name: "Bad".into(),
            version: "0.1.0".into(),
            author: "x".into(),
            description: "d".into(),
            kind: PluginKind::MtProvider,
            wasm_file: "missing.wasm".into(),
            enabled: true,
        };
        std::fs::write(
            dir.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();

        let mut reg = PluginRegistry::new();
        reg.scan(tmp.path()).unwrap();
        assert_eq!(reg.all().len(), 0);
    }

    #[test]
    fn remove_plugin() {
        let tmp = TempDir::new().unwrap();
        make_plugin_dir(tmp.path(), "com.test.p1", PluginKind::MtProvider, true);

        let mut reg = PluginRegistry::new();
        reg.scan(tmp.path()).unwrap();
        assert_eq!(reg.all().len(), 1);

        let removed = reg.remove("com.test.p1");
        assert!(removed.is_some());
        assert_eq!(reg.all().len(), 0);
    }
}
