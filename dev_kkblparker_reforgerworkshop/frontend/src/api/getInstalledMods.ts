import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { InstalledWorkshopMod, workshopInstalledModsResponseSchema } from '../schemas.ts';

export default async (uuid: string): Promise<InstalledWorkshopMod[]> => {
  const { data } = await axiosInstance.get(`/api/client/servers/${uuid}/workshop/installed`);
  return parseFromApi(workshopInstalledModsResponseSchema, data).mods as unknown as InstalledWorkshopMod[];
};
