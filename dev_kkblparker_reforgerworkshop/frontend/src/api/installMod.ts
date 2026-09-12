import { axiosInstance } from '@/api/axios.ts';

export default async (uuid: string, modId: string, data: { name?: string; version?: string }): Promise<void> => {
  await axiosInstance.post(`/api/client/servers/${uuid}/workshop/mods/${modId}/install`, {
    name: data.name,
    version: data.version,
  });
};
