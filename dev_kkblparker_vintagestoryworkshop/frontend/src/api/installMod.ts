import { axiosInstance } from '@/api/axios.ts';

export default async (
  uuid: string,
  modId: number,
  data: { fileName: string; downloadUrl: string },
): Promise<void> => {
  await axiosInstance.post(`/api/client/servers/${uuid}/vintagestory-workshop/mods/${modId}/install`, {
    file_name: data.fileName,
    download_url: data.downloadUrl,
  });
};
