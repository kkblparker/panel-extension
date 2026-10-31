import { z } from 'zod';

export interface WhitelistEntry {
  uuid: string;
  name: string;
}

export interface BanEntry {
  uuid: string;
  name: string;
  created?: string | null;
  source?: string | null;
  expires?: string | null;
  reason?: string | null;
}

export const whitelistResponseSchema = z.object({
  entries: z.array(z.json()),
});

export const bansResponseSchema = z.object({
  entries: z.array(z.json()),
});
