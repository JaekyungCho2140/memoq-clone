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

// Feature 13 — Plugin Ecosystem (Phase 4)

export type PluginKind = "MtProvider" | "FileParser" | "QaRule";

export interface PluginParam {
  key: string;
  label: string;
  value: string;
  /** true = render as password field */
  secret?: boolean;
}

export interface Plugin {
  id: string;
  name: string;
  version: string;
  kind: PluginKind;
  enabled: boolean;
  /** User-configurable parameters defined in the manifest */
  params: PluginParam[];
  /** Non-null when the runtime reported a load/execution error */
  error: string | null;
  installedAt: string;
}

export interface PluginInstallRequest {
  /** FileRef pointing to the .wasm binary */
  wasmFile: string;
  /** Optional initial param values keyed by param.key */
  paramValues?: Record<string, string>;
}

export interface PluginUpdateRequest {
  enabled?: boolean;
  paramValues?: Record<string, string>;
}

// Feature — Vendor Portal (Phase 4)

export type VendorAssignmentStatus =
  | "pending"
  | "in_progress"
  | "delivered"
  | "accepted"
  | "rejected";

export interface VendorAssignment {
  id: string;
  projectId: string;
  projectName: string;
  fileName: string;
  sourceLang: string;
  targetLang: string;
  deadline: string;
  status: VendorAssignmentStatus;
  /** Vendor's progress (0–100) */
  progressPct: number;
  totalSegments: number;
  translatedSegments: number;
  vendorId: string;
  vendorName: string;
  /** ISO string when delivered */
  deliveredAt: string | null;
  /** Admin note on rejection */
  rejectionNote: string | null;
}

export interface VendorInfo {
  id: string;
  username: string;
  displayName: string;
  email: string;
  langPairs: string[]; // e.g. ["en→ko", "ja→ko"]
  activeAssignments: number;
  totalDelivered: number;
}

// Feature — TM Alignment (Phase 4, AFR-47)

export type AlignmentPhase = "upload" | "processing" | "review" | "saving" | "done";

/** A single aligned sentence pair, ready for user review. */
export interface AlignedPair {
  id: string;
  source: string;
  target: string;
  /** Confidence score 0.0–1.0 returned by the alignment engine */
  score: number;
  /** Whether the user has confirmed this pair for TM import */
  confirmed: boolean;
  /** Whether the user has manually edited either side */
  modified: boolean;
}

export interface AlignmentRequest {
  sourceFileRef: string;
  targetFileRef: string;
  sourceLang: string;
  targetLang: string;
  tmId: string;
}

export interface AlignmentResult {
  pairs: AlignedPair[];
}

export interface AlignmentConfirmRequest {
  tmId: string;
  sourceLang: string;
  targetLang: string;
  pairs: Array<{ source: string; target: string }>;
}
