import { invoke } from "@tauri-apps/api/core";
import type { Project, Segment, TmEntry, TmMatch, TbEntry, TmSearchParams, TbLookupParams } from "../types";

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
