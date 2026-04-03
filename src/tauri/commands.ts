import { invoke } from "@tauri-apps/api/core";
import type { Project, Segment, TmEntry, TmMatch, TbEntry, TmSearchParams, TbLookupParams, QaIssue, MtTranslateParams, MtResult, MtSettings, ProjectStats, RecentProjects, LiveDocsLibrary, LiveDocsMatch } from "../types";

export async function parseFile(path: string): Promise<Project> {
  return invoke<Project>("parse_file", { path });
}

/**
 * Export translated segments back to the original format.
 * Uses the source file as a template and writes the result to outputPath.
 */
export async function exportFile(
  segments: Segment[],
  sourcePath: string,
  outputPath: string,
): Promise<void> {
  return invoke("export_file", { segments, sourcePath, outputPath });
}

export async function saveSegment(
  projectId: string,
  segmentId: string,
  source: string,
  target: string,
  status: string,
  order: number,
): Promise<Segment> {
  return invoke<Segment>("save_segment", { projectId, segmentId, source, target, status, order });
}

export async function createTm(name: string, sourceLang: string, targetLang: string): Promise<string> {
  return invoke<string>("tm_create", { name, sourceLang, targetLang });
}

export async function addToTm(
  tmId: string,
  source: string,
  target: string,
  sourceLang: string,
  targetLang: string,
): Promise<TmEntry> {
  return invoke<TmEntry>("tm_add", { tmId, source, target, sourceLang, targetLang });
}

export async function searchTm(params: TmSearchParams): Promise<TmMatch[]> {
  return invoke<TmMatch[]>("tm_search", params as unknown as Record<string, unknown>);
}

export async function createTb(name: string): Promise<string> {
  return invoke<string>("tb_create", { name });
}

export async function lookupTb(params: TbLookupParams): Promise<TbEntry[]> {
  return invoke<TbEntry[]>("tb_lookup", params as unknown as Record<string, unknown>);
}

export async function addToTb(
  tbId: string,
  sourceTerm: string,
  targetTerm: string,
  sourceLang: string,
  targetLang: string,
  notes: string,
  forbidden: boolean,
): Promise<TbEntry> {
  return invoke<TbEntry>("tb_add", { tbId, sourceTerm, targetTerm, sourceLang, targetLang, notes, forbidden });
}

export async function runQaCheck(projectId: string, tbId?: string): Promise<QaIssue[]> {
  return invoke<QaIssue[]>("run_qa_check", { projectId, tbId: tbId ?? null });
}

/**
 * Translate a segment using the configured MT provider (DeepL or Google).
 * Backend command: mt_translate — implemented in AFR-16.
 */
export async function mtTranslate(params: MtTranslateParams): Promise<MtResult> {
  return invoke<MtResult>("mt_translate", params as unknown as Record<string, unknown>);
}

/**
 * Persist MT provider settings (provider + API key) to the app config.
 * Backend command: mt_save_settings — implemented in AFR-16.
 */
export async function mtSaveSettings(settings: MtSettings): Promise<void> {
  return invoke("mt_save_settings", settings as unknown as Record<string, unknown>);
}

/**
 * Load previously saved MT provider settings from the app config.
 * Backend command: mt_load_settings — implemented in AFR-16.
 */
export async function mtLoadSettings(): Promise<MtSettings | null> {
  return invoke<MtSettings | null>("mt_load_settings");
}

// ── Feature 8: Project Management Enhancement (AFR-18) ──────────────────────

/**
 * Add a file to an existing project. Returns the updated project.
 * Backend command: add_file_to_project
 */
export async function addFileToProject(
  project: Project,
  filePath: string,
): Promise<Project> {
  return invoke<Project>("add_file_to_project", { project, filePath });
}

/**
 * Remove a file from a project by its id. Returns the updated project.
 * Backend command: remove_file_from_project
 */
export async function removeFileFromProject(
  project: Project,
  fileId: string,
): Promise<Project> {
  return invoke<Project>("remove_file_from_project", { project, fileId });
}

/**
 * Save a project to a .mqclone file at the given path.
 * Backend command: save_project
 */
export async function saveProject(
  project: Project,
  savePath: string,
): Promise<void> {
  return invoke("save_project", { project, savePath });
}

/**
 * Load a project from a .mqclone file.
 * Backend command: load_project
 */
export async function loadProject(loadPath: string): Promise<Project> {
  return invoke<Project>("load_project", { loadPath });
}

/**
 * Return aggregated stats for a project.
 * Backend command: get_project_stats
 */
export async function getProjectStats(project: Project): Promise<ProjectStats> {
  return invoke<ProjectStats>("get_project_stats", { project });
}

/**
 * Return the list of recently opened project paths (up to 5).
 * Backend command: get_recent_projects — returns Vec<String>
 */
export async function getRecentProjects(): Promise<RecentProjects> {
  return invoke<RecentProjects>("get_recent_projects");
}

// LiveDocs commands (Phase 2)

/**
 * Create a new LiveDocs library with the given name.
 * Backend command: livedocs_create_library
 */
export async function liveDocsCreateLibrary(name: string): Promise<LiveDocsLibrary> {
  return invoke<LiveDocsLibrary>("livedocs_create_library", { name });
}

/**
 * Add a document (file path) to an existing library.
 * Backend command: livedocs_add_document
 */
export async function liveDocsAddDocument(
  libId: string,
  path: string,
): Promise<LiveDocsLibrary> {
  return invoke<LiveDocsLibrary>("livedocs_add_document", { libId, path });
}

/**
 * List all LiveDocs libraries.
 * Backend command: livedocs_list_libraries
 */
export async function liveDocsListLibraries(): Promise<LiveDocsLibrary[]> {
  return invoke<LiveDocsLibrary[]>("livedocs_list_libraries");
}

/**
 * Search across a library (or all libraries if libId is null) for sentences
 * similar to the given query.
 * Backend command: livedocs_search
 */
export async function liveDocsSearch(
  query: string,
  libId: string,
  minScore: number,
): Promise<LiveDocsMatch[]> {
  return invoke<LiveDocsMatch[]>("livedocs_search", { query, libId, minScore });
}
