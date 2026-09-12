import { axiosInstance } from '@/api/axios.ts';
import { ModListKind } from '../schemas.ts';

export default async (uuid: string, modId: string, kind: ModListKind): Promise<void> => {
  await axiosInstance.post(`/api/client/servers/${uuid}/workshop/mods/${modId}`, { kind });
};
