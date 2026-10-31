import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { WhitelistEntry, whitelistResponseSchema } from '../schemas.ts';

export default async (uuid: string): Promise<WhitelistEntry[]> => {
  const { data } = await axiosInstance.get(`/api/client/servers/${uuid}/minecraft-whitelist/whitelist`);
  const parsed = parseFromApi(whitelistResponseSchema, data);

  return parsed.entries as unknown as WhitelistEntry[];
};
