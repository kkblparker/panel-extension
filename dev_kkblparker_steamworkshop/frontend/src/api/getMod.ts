import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { ModListKind, toWorkshopMods, WorkshopMod, workshopModResponseSchema } from '../schemas.ts';

export default async (
  uuid: string,
  modId: string,
): Promise<{ mod: WorkshopMod | null; availableKinds: ModListKind[] }> => {
  const { data } = await axiosInstance.get(`/api/client/servers/${uuid}/workshop/mods/${modId}`);
  const parsed = parseFromApi(workshopModResponseSchema, data);

  return {
    mod: toWorkshopMods(parsed.result)[0] ?? null,
    availableKinds: parsed.available_kinds as ModListKind[],
  };
};
