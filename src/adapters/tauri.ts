/**
 * Tauri adapter — runs inside the Tauri desktop shell.
 *
 * Delegates to `@tauri-apps/plugin-dialog` for file pickers and
 * to the existing `src/tauri/commands.ts` for all other operations.
 */

import { open, save } from "@tauri-apps/plugin-dialog";
import * as cmds from "../tauri/commands";
import type { IAppAdapter, OpenFileDialogOptions, SaveFileDialogOptions, FileRef } from "./types";
import type { Project, Segment } from "../types";

export const tauriAdapter: IAppAdapter = {
  // ── Dialog ────────────────────────────────────────────────────────────────

  async openFileDialog(options: OpenFileDialogOptions = {}): Promise<FileRef | null> {
    const selected = await open({
      multiple: options.multiple ?? false,
      filters: options.filters,
    });
    if (!selected) return null;
    // `open` returns string | string[] | null; we only support single selection
    return typeof selected === "string" ? selected : selected[0] ?? null;
  },

  async saveFileDialog(options: SaveFileDialogOptions = {}): Promise<string | null> {
    const path = await save({
      defaultPath: options.defaultPath,
      filters: options.filters,
    });
    return path ?? null;
  },

  // ── Project ───────────────────────────────────────────────────────────────

  parseFile: (fileRef: FileRef) => cmds.parseFile(fileRef),

  exportFile: (segments: Segment[], sourcePath: string, outputPath: string) =>
    cmds.exportFile(segments, sourcePath, outputPath),

  saveProject: (project: Project, savePath: string) =>
    cmds.saveProject(project, savePath),

  loadProject: (fileRef: FileRef) => cmds.loadProject(fileRef),

  addFileToProject: (project: Project, fileRef: FileRef) =>
    cmds.addFileToProject(project, fileRef),

  removeFileFromProject: (project: Project, fileId: string) =>
    cmds.removeFileFromProject(project, fileId),

  getProjectStats: (project: Project) => cmds.getProjectStats(project),

  getRecentProjects: () => cmds.getRecentProjects(),

  // ── Translation Memory ────────────────────────────────────────────────────

  createTm: (name, sourceLang, targetLang) => cmds.createTm(name, sourceLang, targetLang),

  addToTm: (tmId, source, target, sourceLang, targetLang) =>
    cmds.addToTm(tmId, source, target, sourceLang, targetLang),

  searchTm: (params) => cmds.searchTm(params),

  // ── Term Base ─────────────────────────────────────────────────────────────

  createTb: (name) => cmds.createTb(name),

  lookupTb: (params) => cmds.lookupTb(params),

  addToTb: (tbId, sourceTerm, targetTerm, sourceLang, targetLang, notes, forbidden) =>
    cmds.addToTb(tbId, sourceTerm, targetTerm, sourceLang, targetLang, notes, forbidden),

  // ── Segment ───────────────────────────────────────────────────────────────

  saveSegment: (projectId, segmentId, source, target, status, order) =>
    cmds.saveSegment(projectId, segmentId, source, target, status, order),

  // ── QA ───────────────────────────────────────────────────────────────────

  runQaCheck: (projectId, tbId) => cmds.runQaCheck(projectId, tbId),

  // ── Machine Translation ───────────────────────────────────────────────────

  mtTranslate: (params) => cmds.mtTranslate(params),

  mtSaveSettings: (settings) => cmds.mtSaveSettings(settings),

  mtLoadSettings: () => cmds.mtLoadSettings(),

  // ── LiveDocs ──────────────────────────────────────────────────────────────

  liveDocsCreateLibrary: (name) => cmds.liveDocsCreateLibrary(name),

  liveDocsListLibraries: () => cmds.liveDocsListLibraries(),

  liveDocsAddDocument: (libId, fileRef) => cmds.liveDocsAddDocument(libId, fileRef),

  liveDocsSearch: (query, libId, minScore) => cmds.liveDocsSearch(query, libId, minScore),
};
