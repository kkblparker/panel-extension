import { z } from 'zod';

// The Vintage Story Mod DB's shape is external and not ours to version - kept as opaque json
// (see api-transform's isOpaqueJson) and typed loosely below for rendering. Normalized
// backend-side (see vsmoddb.rs's normalize_summary/normalize_detail) into this same flat shape
// regardless of whether it came from a search result row or the single-mod detail page.
export const workshopSearchResponseSchema = z.object({
  mods: z.array(z.json()),
  total: z.number(),
  perPage: z.number(),
});

export const workshopModResponseSchema = z.object({
  result: z.json(),
});

export interface WorkshopMod {
  id: number;
  name: string;
  summary?: string;
  author?: string;
  imageUrl?: string;
  tags?: string[];
  downloads?: number;
  side?: string;
  lastReleased?: string;
  workshopUrl?: string;

  // Only present on the single-mod details endpoint.
  description?: string;
  releases?: WorkshopModRelease[];
}

export interface WorkshopModRelease {
  releaseId: number;
  version: string;
  fileName: string;
  downloadUrl: string;
  gameVersions?: string[];
  created?: string;
  changelog?: string;
}

export interface WorkshopSearchResult {
  mods: WorkshopMod[];
  total: number;
  perPage: number;
}

export const workshopInstalledModsResponseSchema = z.object({
  mods: z.array(
    z.object({
      fileName: z.string(),
      size: z.number(),
      modified: z.string(),
    }),
  ),
});

export interface InstalledWorkshopMod {
  fileName: string;
  size: number;
  modified: string;
}
