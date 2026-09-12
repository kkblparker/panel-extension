import { z } from 'zod';

// reforgermods.net's mod shape is external, camelCase already and not ours to version - kept as
// opaque json (see api-transform's isOpaqueJson) and typed loosely below for rendering, the same
// way the panel's activity log `data` field is treated.
export const workshopSearchResponseSchema = z.object({
  result: z.json(),
});

export const workshopModResponseSchema = z.object({
  result: z.json(),
});

export const workshopInstalledModsResponseSchema = z.object({
  mods: z.array(z.json()),
});

export interface WorkshopMod {
  id: string;
  name: string;
  type?: string;
  summary?: string;
  author?: string;
  version?: string;
  sizeFormatted?: string;
  rating?: number;
  ratingCount?: number;
  subscriberCount?: number;
  tags?: string[];
  imageUrl?: string;
  workshopUrl?: string;

  // Only present on the single-mod details endpoint (GET /workshop/mods/{id})
  description?: string;
  downloadCount?: number;
  dependencies?: WorkshopModDependency[];
}

export interface WorkshopModDependency {
  id?: string;
  name?: string;
  version?: string;
}

export interface WorkshopSearchResult {
  status: string;
  data: WorkshopMod[];
}

export interface InstalledWorkshopMod {
  modId: string;
  name?: string;
  version?: string;
}
