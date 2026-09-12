import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { WorkshopSearchResult, workshopSearchResponseSchema } from '../schemas.ts';

export default async (
  uuid: string,
  params: { search?: string; sort?: string; page?: number },
): Promise<WorkshopSearchResult> => {
  const { data } = await axiosInstance.get(`/api/client/servers/${uuid}/workshop/mods`, {
    params: {
      search: params.search || undefined,
      sort: params.sort || undefined,
      page: params.page ?? 1,
    },
  });

  return parseFromApi(workshopSearchResponseSchema, data).result as unknown as WorkshopSearchResult;
};
