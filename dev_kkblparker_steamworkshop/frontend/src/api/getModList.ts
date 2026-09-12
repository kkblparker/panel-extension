import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { ModListKind, toWorkshopMods, WorkshopMod, workshopInstalledResponseSchema } from '../schemas.ts';

export default async (
  uuid: string,
): Promise<{
  mods: WorkshopMod[];
  kindsById: Record<string, ModListKind[]>;
  availableKinds: ModListKind[];
  loadOrderSupported: boolean;
  loadOrder: string | null;
}> => {
  const { data } = await axiosInstance.get(`/api/client/servers/${uuid}/workshop/installed`);
  const parsed = parseFromApi(workshopInstalledResponseSchema, data);

  const kindsById: Record<string, ModListKind[]> = {};
  for (const entry of parsed.mods) {
    kindsById[entry.id] = entry.kinds as ModListKind[];
  }

  return {
    mods: toWorkshopMods(parsed.result),
    kindsById,
    availableKinds: parsed.available_kinds as ModListKind[],
    loadOrderSupported: parsed.load_order_supported,
    loadOrder: parsed.load_order,
  };
};
