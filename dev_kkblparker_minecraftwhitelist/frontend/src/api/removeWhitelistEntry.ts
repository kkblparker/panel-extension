import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { WhitelistEntry, whitelistResponseSchema } from '../schemas.ts';

export default async (uuid: string, entryUuid: string): Promise<WhitelistEntry[]> => {
  const { data } = await axiosInstance.delete(
    `/api/client/servers/${uuid}/minecraft-whitelist/whitelist/${entryUuid}`,
  );
  const parsed = parseFromApi(whitelistResponseSchema, data);

  return parsed.entries as unknown as WhitelistEntry[];
};
