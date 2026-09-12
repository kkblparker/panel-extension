import { z } from 'zod';

// Modrinth's shape is external and not ours to version - kept as opaque json (see
// api-transform's isOpaqueJson) and typed loosely below for rendering, the same way the panel's
// activity log `data` field is treated. Notably, Modrinth itself is inconsistent between
// endpoints: search hits use `project_id`/`follows`, while GET /project/{id} uses `id`/
// `followers` for the same concepts - both are optional here, resolved via modId()/modFollowers().
export const workshopSearchResponseSchema = z.object({
  result: z.json(),
});

export const workshopModResponseSchema = z.object({
  result: z.json(),
});

export const workshopInstalledResponseSchema = z.object({
  installed: z.array(z.json()),
});

export interface WorkshopMod {
  id?: string;
  project_id?: string;
  slug?: string;
  title: string;
  description?: string;
  body?: string;
  author?: string;
  icon_url?: string;
  downloads?: number;
  follows?: number;
  followers?: number;
  categories?: string[];
  project_type?: string;
}

export function modId(mod: WorkshopMod): string {
  return mod.project_id ?? mod.id ?? '';
}

export function modFollowers(mod: WorkshopMod): number | undefined {
  return mod.follows ?? mod.followers;
}

export interface WorkshopSearchResult {
  hits: WorkshopMod[];
  total_hits: number;
}

export interface InstalledWorkshopMod {
  project_id: string;
  version_id?: string;
  file_name?: string;
  name?: string;
  icon_url?: string;
}
