/**
 * Web adapter — runs in a plain browser without Tauri.
 *
 * File I/O:
 *  - File pickers use the HTML <input type="file"> API.
 *  - Files are registered in an in-memory map under a synthetic "web-file://<uuid>"
 *    URI so that the rest of the app can pass FileRefs around as opaque strings.
 *  - Uploads go to the REST server implemented in AFR-31.
 *
 * Persistence:
 *  - MT settings are stored in localStorage (no server round-trip needed).
 *  - Recent projects list is fetched from the server (returns [] until AFR-31 lands).
 *
 * Server base URL:
 *  - Defaults to the current origin (same host / port as the Vite dev server proxy
 *    or the production static file server).
 *  - Override by setting the global `window.__WEB_API_BASE__` before the app boots.
 */

import type { IAppAdapter, OpenFileDialogOptions, SaveFileDialogOptions, FileRef } from "./types";
import type {
  Project,
  Segment,
  TmEntry,
  TmMatch,
  TbEntry,
  TmSearchParams,
  TbLookupParams,
  QaIssue,
  MtTranslateParams,
  MtResult,
  MtSettings,
  ProjectStats,
  RecentProjects,
  LiveDocsLibrary,
  LiveDocsMatch,
  Plugin,
  PluginInstallRequest,
  PluginUpdateRequest,
  AlignmentRequest,
  AlignmentResult,
  AlignmentConfirmRequest,
} from "../types";

// ── File registry ─────────────────────────────────────────────────────────────

const WEB_FILE_PREFIX = "web-file://";
const fileRegistry = new Map<string, File>();

export function registerFile(file: File): FileRef {
  const id = `${WEB_FILE_PREFIX}${crypto.randomUUID()}`;
  fileRegistry.set(id, file);
  return id;
}

function resolveFile(fileRef: FileRef): File {
  if (!fileRef.startsWith(WEB_FILE_PREFIX)) {
    throw new Error(`[web adapter] Expected a web-file:// ref, got: ${fileRef}`);
  }
  const file = fileRegistry.get(fileRef);
  if (!file) throw new Error(`[web adapter] File not found in registry: ${fileRef}`);
  return file;
}

// ── Browser file picker helpers ───────────────────────────────────────────────

function buildAccept(filters?: { name: string; extensions: string[] }[]): string {
  if (!filters || filters.length === 0) return "";
  return filters.flatMap((f) => f.extensions.map((e) => `.${e}`)).join(",");
}

function pickFile(options: OpenFileDialogOptions): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.multiple = options.multiple ?? false;
    input.accept = buildAccept(options.filters);

    // Some browsers fire "cancel" on the input; others just never fire "change"
    input.addEventListener("cancel", () => resolve(null), { once: true });
    input.addEventListener(
      "change",
      () => {
        const file = input.files?.[0] ?? null;
        resolve(file);
        // Cleanup
        input.remove();
      },
      { once: true },
    );

    // Must be in the DOM for some browsers
    input.style.display = "none";
    document.body.appendChild(input);
    input.click();
  });
}

// ── Browser download helper ───────────────────────────────────────────────────

function downloadBlob(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

// ── Server API base URL ───────────────────────────────────────────────────────

function apiBase(): string {
  return ((window as unknown) as Record<string, unknown>).__WEB_API_BASE__ as string ?? "";
}

/** Return the current access token from localStorage (if present). */
function getAccessToken(): string | null {
  return localStorage.getItem("mq_access_token");
}

async function apiFetch(path: string, init?: RequestInit): Promise<Response> {
  const token = getAccessToken();
  const baseHeaders: Record<string, string> = { Accept: "application/json" };
  if (token) baseHeaders["Authorization"] = `Bearer ${token}`;
  const res = await fetch(`${apiBase()}${path}`, {
    headers: { ...baseHeaders, ...(init?.headers as Record<string, string> | undefined) },
    ...init,
  });
  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText);
    throw new Error(`[web adapter] ${init?.method ?? "GET"} ${path} → ${res.status}: ${text}`);
  }
  return res;
}

/** Build the WebSocket URL for a project (includes auth token as query param). */
export function projectWsUrl(projectId: string): string {
  const base = apiBase().replace(/^http/, "ws");
  const token = getAccessToken() ?? "";
  return `${base}/api/projects/${projectId}/ws?token=${encodeURIComponent(token)}`;
}

async function apiJson<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await apiFetch(path, init);
  return res.json() as Promise<T>;
}

// ── Adapter implementation ────────────────────────────────────────────────────

export const webAdapter: IAppAdapter = {
  // ── Dialog ─────────────────────────────────────────────────────────────────

  async openFileDialog(options: OpenFileDialogOptions = {}): Promise<FileRef | null> {
    const file = await pickFile(options);
    if (!file) return null;
    return registerFile(file);
  },

  async saveFileDialog(options: SaveFileDialogOptions = {}): Promise<string | null> {
    // In the browser there is no native save dialog.
    // We return the suggested filename; the actual download happens inside
    // exportFile / saveProject when they detect the web environment.
    const suggested =
      options.defaultPath?.split(/[\\/]/).pop() ??
      options.filters?.[0]?.extensions?.[0] ??
      "file";
    return suggested;
  },

  // ── Project ─────────────────────────────────────────────────────────────────

  async parseFile(fileRef: FileRef): Promise<Project> {
    const file = resolveFile(fileRef);
    const body = new FormData();
    body.append("file", file);
    return apiJson<Project>("/api/projects/parse", { method: "POST", body });
  },

  async exportFile(segments: Segment[], sourcePath: string, _outputPath: string): Promise<void> {
    const res = await apiFetch("/api/export", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ segments, sourcePath }),
    });
    const blob = await res.blob();
    const ext = sourcePath.endsWith(".docx") ? "docx" : "xliff";
    downloadBlob(blob, `translated.${ext}`);
  },

  async saveProject(project: Project, savePath: string): Promise<void> {
    const res = await apiFetch("/api/projects/export", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(project),
    });
    const blob = await res.blob();
    const filename = savePath.endsWith(".mqclone") ? savePath : `${savePath}.mqclone`;
    downloadBlob(blob, filename.split(/[\\/]/).pop() ?? "project.mqclone");
  },

  async loadProject(fileRef: FileRef): Promise<Project> {
    const file = resolveFile(fileRef);
    const body = new FormData();
    body.append("file", file);
    return apiJson<Project>("/api/projects/load", { method: "POST", body });
  },

  async addFileToProject(project: Project, fileRef: FileRef): Promise<Project> {
    const file = resolveFile(fileRef);
    const body = new FormData();
    body.append("project", JSON.stringify(project));
    body.append("file", file);
    return apiJson<Project>("/api/projects/add-file", { method: "POST", body });
  },

  async removeFileFromProject(project: Project, fileId: string): Promise<Project> {
    return apiJson<Project>("/api/projects/remove-file", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ project, fileId }),
    });
  },

  async getProjectStats(project: Project): Promise<ProjectStats> {
    return apiJson<ProjectStats>("/api/projects/stats", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(project),
    });
  },

  async getRecentProjects(): Promise<RecentProjects> {
    // Server not yet available (AFR-31). Return empty list gracefully.
    try {
      return await apiJson<RecentProjects>("/api/projects/recent");
    } catch {
      return [];
    }
  },

  // ── Translation Memory ──────────────────────────────────────────────────────

  async createTm(name: string, sourceLang: string, targetLang: string): Promise<string> {
    const res = await apiJson<{ id: string }>("/api/tm", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name, sourceLang, targetLang }),
    });
    return res.id;
  },

  async addToTm(
    tmId: string,
    source: string,
    target: string,
    sourceLang: string,
    targetLang: string,
  ): Promise<TmEntry> {
    return apiJson<TmEntry>(`/api/tm/${tmId}/entries`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ source, target, sourceLang, targetLang }),
    });
  },

  async searchTm(params: TmSearchParams): Promise<TmMatch[]> {
    return apiJson<TmMatch[]>(`/api/tm/${params.tmId}/search`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(params),
    });
  },

  // ── Term Base ───────────────────────────────────────────────────────────────

  async createTb(name: string): Promise<string> {
    const res = await apiJson<{ id: string }>("/api/tb", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name }),
    });
    return res.id;
  },

  async lookupTb(params: TbLookupParams): Promise<TbEntry[]> {
    return apiJson<TbEntry[]>(`/api/tb/${params.tbId}/lookup`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(params),
    });
  },

  async addToTb(
    tbId: string,
    sourceTerm: string,
    targetTerm: string,
    sourceLang: string,
    targetLang: string,
    notes: string,
    forbidden: boolean,
  ): Promise<TbEntry> {
    return apiJson<TbEntry>(`/api/tb/${tbId}/terms`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ sourceTerm, targetTerm, sourceLang, targetLang, notes, forbidden }),
    });
  },

  // ── Segment ──────────────────────────────────────────────────────────────────

  async saveSegment(
    projectId: string,
    segmentId: string,
    source: string,
    target: string,
    status: string,
    order: number,
  ): Promise<Segment> {
    return apiJson<Segment>(`/api/projects/${projectId}/segments/${segmentId}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ source, target, status, order }),
    });
  },

  // ── QA ──────────────────────────────────────────────────────────────────────

  async runQaCheck(projectId: string, tbId?: string): Promise<QaIssue[]> {
    return apiJson<QaIssue[]>("/api/qa/check", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ projectId, tbId: tbId ?? null }),
    });
  },

  // ── Machine Translation ──────────────────────────────────────────────────────

  async mtTranslate(params: MtTranslateParams): Promise<MtResult> {
    return apiJson<MtResult>("/api/mt/translate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(params),
    });
  },

  async mtSaveSettings(settings: MtSettings): Promise<void> {
    // Store in localStorage in web mode (no server secret storage needed for MVP)
    localStorage.setItem("mt_settings", JSON.stringify(settings));
  },

  async mtLoadSettings(): Promise<MtSettings | null> {
    const raw = localStorage.getItem("mt_settings");
    if (!raw) return null;
    try {
      return JSON.parse(raw) as MtSettings;
    } catch {
      return null;
    }
  },

  // ── LiveDocs ──────────────────────────────────────────────────────────────────

  async liveDocsCreateLibrary(name: string): Promise<LiveDocsLibrary> {
    return apiJson<LiveDocsLibrary>("/api/livedocs/libraries", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name }),
    });
  },

  async liveDocsListLibraries(): Promise<LiveDocsLibrary[]> {
    try {
      return await apiJson<LiveDocsLibrary[]>("/api/livedocs/libraries");
    } catch {
      return [];
    }
  },

  async liveDocsAddDocument(libId: string, fileRef: FileRef): Promise<LiveDocsLibrary> {
    const file = resolveFile(fileRef);
    const body = new FormData();
    body.append("file", file);
    return apiJson<LiveDocsLibrary>(`/api/livedocs/libraries/${libId}/documents`, {
      method: "POST",
      body,
    });
  },

  async liveDocsSearch(query: string, libId: string, minScore: number): Promise<LiveDocsMatch[]> {
    return apiJson<LiveDocsMatch[]>("/api/livedocs/search", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ query, libId, minScore }),
    });
  },

  // ── Plugins ───────────────────────────────────────────────────────────────

  async pluginList(): Promise<Plugin[]> {
    try {
      return await apiJson<Plugin[]>("/api/plugins");
    } catch {
      return [];
    }
  },

  async pluginInstall(req: PluginInstallRequest): Promise<Plugin> {
    const file = resolveFile(req.wasmFile);
    const body = new FormData();
    body.append("file", file);
    if (req.paramValues) {
      body.append("params", JSON.stringify(req.paramValues));
    }
    return apiJson<Plugin>("/api/plugins", { method: "POST", body });
  },

  async pluginUpdate(id: string, req: PluginUpdateRequest): Promise<Plugin> {
    return apiJson<Plugin>(`/api/plugins/${id}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(req),
    });
  },

  async pluginRemove(id: string): Promise<void> {
    await apiJson<unknown>(`/api/plugins/${id}`, { method: "DELETE" });
  },

  // ── TM Alignment (Phase 4, AFR-47) ───────────────────────────────────────

  async alignmentAlign(req: AlignmentRequest): Promise<AlignmentResult> {
    const sourceFile = resolveFile(req.sourceFileRef);
    const targetFile = resolveFile(req.targetFileRef);
    const body = new FormData();
    body.append("sourceFile", sourceFile);
    body.append("targetFile", targetFile);
    body.append("sourceLang", req.sourceLang);
    body.append("targetLang", req.targetLang);
    body.append("tmId", req.tmId);
    return apiJson<AlignmentResult>("/api/alignment/align", { method: "POST", body });
  },

  async alignmentConfirm(req: AlignmentConfirmRequest): Promise<void> {
    await apiJson<unknown>("/api/alignment/confirm", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(req),
    });
  },
};
