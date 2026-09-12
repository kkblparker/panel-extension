import { axiosInstance } from '@/api/axios.ts';

export default async (uuid: string, modId: string): Promise<void> => {
  await axiosInstance.delete(`/api/client/servers/${uuid}/workshop/mods/${modId}`);
};
