import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { BanEntry, bansResponseSchema } from '../schemas.ts';

export default async (uuid: string, username: string, reason?: string): Promise<BanEntry[]> => {
  const { data } = await axiosInstance.post(`/api/client/servers/${uuid}/minecraft-whitelist/bans`, {
    username,
    reason: reason || undefined,
  });
  const parsed = parseFromApi(bansResponseSchema, data);

  return parsed.entries as unknown as BanEntry[];
};
