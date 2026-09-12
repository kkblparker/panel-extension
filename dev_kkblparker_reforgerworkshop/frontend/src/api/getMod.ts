import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { WorkshopMod, workshopModResponseSchema } from '../schemas.ts';

export default async (uuid: string, modId: string): Promise<WorkshopMod> => {
  const { data } = await axiosInstance.get(`/api/client/servers/${uuid}/workshop/mods/${modId}`);
  return parseFromApi(workshopModResponseSchema, data).result as unknown as WorkshopMod;
};
