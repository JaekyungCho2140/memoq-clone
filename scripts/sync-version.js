#!/usr/bin/env node
/**
 * sync-version.js — synchronise version across package.json and
 * src-tauri/Cargo.toml + src-tauri/tauri.conf.json.
 *
 * Usage:
 *   node scripts/sync-version.js [new-version]
 *
 * If [new-version] is omitted, reads the current version from package.json
 * and writes it to the Rust/Tauri manifests.
 *
 * Examples:
 *   node scripts/sync-version.js          # sync existing version
 *   node scripts/sync-version.js 1.2.3    # bump to 1.2.3 everywhere
 */

const fs = require("fs");
const path = require("path");

const ROOT = path.resolve(__dirname, "..");

// ���─ helpers ──────────────────────────────────────────────────────────────────

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf-8"));
}

function writeJson(filePath, obj) {
  fs.writeFileSync(filePath, JSON.stringify(obj, null, 2) + "\n", "utf-8");
  console.log(`  ✅ ${path.relative(ROOT, filePath)}`);
}

function patchCargoToml(filePath, version) {
  let content = fs.readFileSync(filePath, "utf-8");
  const versionPattern = /^(version\s*=\s*")([^"]+)(")/m;
  if (!versionPattern.test(content)) {
    console.log(
      `  ⚠️  ${path.relative(ROOT, filePath)} — version line not found, skipped`,
    );
    return;
  }
  const updated = content.replace(versionPattern, `$1${version}$3`);
  fs.writeFileSync(filePath, updated, "utf-8");
  console.log(`  ✅ ${path.relative(ROOT, filePath)}`);
}

// ── main ─────────────────────────────────────────────────────────────────────

const pkgPath = path.join(ROOT, "package.json");
const tauriConfPath = path.join(ROOT, "src-tauri", "tauri.conf.json");
const cargoTomlPath = path.join(ROOT, "src-tauri", "Cargo.toml");

const pkg = readJson(pkgPath);
const newVersion = process.argv[2] || pkg.version;

// Validate semver-ish
if (!/^\d+\.\d+\.\d+/.test(newVersion)) {
  console.error(`❌ Invalid version: "${newVersion}". Expected semver (e.g. 1.2.3).`);
  process.exit(1);
}

console.log(`\nSyncing version → ${newVersion}\n`);

// 1. package.json
pkg.version = newVersion;
writeJson(pkgPath, pkg);

// 2. tauri.conf.json
const tauriConf = readJson(tauriConfPath);
tauriConf.version = newVersion;
writeJson(tauriConfPath, tauriConf);

// 3. src-tauri/Cargo.toml
patchCargoToml(cargoTomlPath, newVersion);

console.log(`\nDone. All manifests now at v${newVersion}\n`);
