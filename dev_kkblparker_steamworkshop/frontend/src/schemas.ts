import { z } from 'zod';

// Mirrors the backend's `ModListKind` (variables.rs). Most eggs only ever use `client` - `server`
// and `optional` only show up for eggs that split their mod lists the way Arma 3 does
// (MODIFICATIONS/SERVERMODS/OPTIONALMODS).
export const modListKindSchema = z.enum(['client', 'server', 'optional']);
export type ModListKind = z.infer<typeof modListKindSchema>;

export const MOD_LIST_KIND_LABELS: Record<ModListKind, string> = {
  client: 'Client Mod',
  server: 'Server-Only Mod',
  optional: 'Optional Mod',
};

// The Steam Web API's raw JSON (QueryFiles/GetPublishedFileDetails) is external, snake_case, and
// not ours to version - kept as opaque json (see api-transform's isOpaqueJson) and normalized into
// WorkshopMod below for rendering, the same way the Reforger/Minecraft Workshop extensions treat
// their upstream APIs.
export const workshopSearchResponseSchema = z.object({
  result: z.json(),
  available_kinds: z.array(modListKindSchema),
});

export const workshopModResponseSchema = z.object({
  result: z.json(),
  available_kinds: z.array(modListKindSchema),
});

export const workshopInstalledResponseSchema = z.object({
  result: z.json(),
  mods: z.array(z.object({ id: z.string(), kinds: z.array(modListKindSchema) })),
  available_kinds: z.array(modListKindSchema),
  load_order_supported: z.boolean(),
  load_order: z.string().nullable(),
});

export const workshopLoadOrderResponseSchema = z.object({
  supported: z.boolean(),
  value: z.string().nullable(),
});

export interface WorkshopMod {
  id: string;
  name: string;
  summary?: string;
  description?: string;
  imageUrl?: string;
  tags: string[];
  rating?: number;
  ratingCount?: number;
  subscriberCount?: number;
  sizeFormatted?: string;
  workshopUrl: string;
  banned: boolean;
  banReason?: string;
  // Best-effort: some games (Project Zomboid) split "Workshop ID" from a separate "Mod ID" that
  // has to be listed elsewhere to actually load the mod, and have no API to look one up from the
  // other. Community convention on those games is to print "Mod ID: <id>" in the item's own
  // description, so this is a plain text scrape of that - a hint to review, not an authoritative
  // value, since not every mod page follows the convention or follows it correctly.
  modIdHint?: string;
}

interface SteamTag {
  tag?: string;
}

interface SteamVoteData {
  score?: number;
  votes_up?: number;
  votes_down?: number;
}

// Loosely typed - only the fields this extension actually reads. Both QueryFiles (search) and
// GetPublishedFileDetails (single item / batch "installed" lookup) return this same shape, just
// with different completeness depending on which return_* flags were requested.
interface SteamPublishedFileDetails {
  publishedfileid?: string;
  result?: number;
  title?: string;
  short_description?: string;
  file_description?: string;
  preview_url?: string;
  file_size?: string | number;
  subscriptions?: number;
  tags?: SteamTag[];
  vote_data?: SteamVoteData;
  banned?: boolean;
  ban_reason?: string;
}

// Matches the "Mod ID: <id>" line most current Project Zomboid Workshop pages print in their
// description (verified against several real, popular mods) - takes the first match, since a
// "Requires Mod ID: ..." dependency line can appear separately and shouldn't be confused for it.
const MOD_ID_HINT_PATTERN = /(?<!requires\s)mod\s*id:\s*([^\r\n]+)/i;

function extractModIdHint(description?: string): string | undefined {
  const match = description?.match(MOD_ID_HINT_PATTERN);
  return match?.[1]?.trim() || undefined;
}

function formatSize(bytes?: string | number): string | undefined {
  const value = typeof bytes === 'string' ? Number(bytes) : bytes;
  if (!value || Number.isNaN(value) || value <= 0) return undefined;

  const units = ['B', 'KB', 'MB', 'GB'];
  let size = value;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit += 1;
  }

  return `${size.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

export function toWorkshopMod(raw: unknown): WorkshopMod | null {
  const details = raw as SteamPublishedFileDetails;
  if (!details?.publishedfileid) return null;

  const votesUp = details.vote_data?.votes_up ?? 0;
  const votesDown = details.vote_data?.votes_down ?? 0;
  const description = details.file_description || details.short_description || undefined;

  return {
    id: details.publishedfileid,
    name: details.title || details.publishedfileid,
    summary: details.short_description || undefined,
    description,
    imageUrl: details.preview_url || undefined,
    tags: (details.tags ?? []).map((tag) => tag.tag).filter((tag): tag is string => !!tag),
    rating: details.vote_data?.score,
    ratingCount: votesUp + votesDown > 0 ? votesUp + votesDown : undefined,
    subscriberCount: details.subscriptions,
    sizeFormatted: formatSize(details.file_size),
    workshopUrl: `https://steamcommunity.com/sharedfiles/filedetails/?id=${details.publishedfileid}`,
    banned: details.banned ?? false,
    banReason: details.ban_reason || undefined,
    modIdHint: extractModIdHint(description),
  };
}

export function toWorkshopMods(raw: unknown): WorkshopMod[] {
  const list = (raw as { response?: { publishedfiledetails?: unknown[] } })?.response?.publishedfiledetails ?? [];
  return list.map(toWorkshopMod).filter((mod): mod is WorkshopMod => mod !== null);
}

export function totalResults(raw: unknown): number {
  return (raw as { response?: { total?: number } })?.response?.total ?? 0;
}
