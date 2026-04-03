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

export interface Project {
  id: string;
  name: string;
  sourceLang: string;
  targetLang: string;
  createdAt: string;
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
