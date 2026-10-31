import { axiosInstance } from '@/api/axios.ts';

export default async (uuid: string, fileName: string): Promise<void> => {
  await axiosInstance.delete(
    `/api/client/servers/${uuid}/vintagestory-workshop/installed/${encodeURIComponent(fileName)}`,
  );
};
