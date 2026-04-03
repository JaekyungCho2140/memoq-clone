export type SegmentStatus = "untranslated" | "draft" | "translated" | "confirmed";
export type MatchType = "exact" | "fuzzy" | "context";

export interface TmMatch {
  source: string;
  target: string;
  /** Similarity score 0.0 – 1.0 */
  score: number;
  matchType: MatchType;
}

export interface Segment {
  id: string;
  source: string;
  target: string;
  status: SegmentStatus;
  tmMatches: TmMatch[];
  order: number;
}

export interface ProjectFile {
  id: string;
  path: string;
  segments: Segment[];
}

export interface Project {
  id: string;
  name: string;
  /** Absolute path to the original source file (used for export) */
  sourcePath: string;
  sourceLang: string;
  targetLang: string;
  createdAt: string;
  /** Multi-file support (default empty, matches Rust serde default) */
  files?: ProjectFile[];
  segments: Segment[];
}

export interface TmEntry {
  id: string;
  source: string;
  target: string;
  sourceLang: string;
  targetLang: string;
  createdAt: string;
  metadata: Record<string, string>;
}

export interface TbEntry {
  id: string;
  sourceTerm: string;
  targetTerm: string;
  sourceLang: string;
  targetLang: string;
  notes: string;
  forbidden: boolean;
}

export interface TmSearchParams {
  tmId: string;
  query: string;
  sourceLang: string;
  targetLang: string;
  minScore: number;
}

export interface TbLookupParams {
  tbId: string;
  term: string;
  sourceLang: string;
}

export type MtProvider = "deepl" | "google";

export interface MtSettings {
  provider: MtProvider;
  apiKey: string;
}

export interface MtResult {
  source: string;
  target: string;
  provider: MtProvider;
}

export interface MtTranslateParams {
  source: string;
  sourceLang: string;
  targetLang: string;
  provider: MtProvider;
  apiKey: string;
}

export type QaSeverity = "Error" | "Warning";

export type QaCheckType =
  | "TagMismatch"
  | "NumberMismatch"
  | "Untranslated"
  | "ForbiddenTerm"
  | "SourceEqualsTarget";

export interface QaIssue {
  segment_id: string;
  check_type: QaCheckType;
  severity: QaSeverity;
  message: string;
}

// Feature 8 — Project Management Enhancement

/** Stats for a single file or the entire project */
export interface ProjectStats {
  totalSegments: number;
  translated: number;
  confirmed: number;
  completionPct: number; // 0–100
}

/** Recently opened projects — Rust returns Vec<String> (paths only) */
export type RecentProjects = string[];

// Feature — LiveDocs (Phase 2)

export interface LiveDocsDocument {
  id: string;
  path: string;
  sentences: string[];
}

export interface LiveDocsLibrary {
  id: string;
  name: string;
  documents: LiveDocsDocument[];
}

export interface LiveDocsMatch {
  sentence: string;
  docPath: string;
  score: number;
}
