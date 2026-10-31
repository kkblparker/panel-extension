import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { BanEntry, bansResponseSchema } from '../schemas.ts';

export default async (uuid: string): Promise<BanEntry[]> => {
  const { data } = await axiosInstance.get(`/api/client/servers/${uuid}/minecraft-whitelist/bans`);
  const parsed = parseFromApi(bansResponseSchema, data);

  return parsed.entries as unknown as BanEntry[];
};
