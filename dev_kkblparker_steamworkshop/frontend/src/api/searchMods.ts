import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { ModListKind, totalResults, toWorkshopMods, workshopSearchResponseSchema } from '../schemas.ts';

export default async (uuid: string, params: { search?: string; sort?: string; page?: number }) => {
  const { data } = await axiosInstance.get(`/api/client/servers/${uuid}/workshop/mods`, {
    params: {
      search: params.search || undefined,
      sort: params.sort || undefined,
      page: params.page ?? 1,
    },
  });

  const parsed = parseFromApi(workshopSearchResponseSchema, data);

  return {
    mods: toWorkshopMods(parsed.result),
    total: totalResults(parsed.result),
    availableKinds: parsed.available_kinds as ModListKind[],
  };
};
