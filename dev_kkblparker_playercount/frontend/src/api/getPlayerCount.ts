import { axiosInstance } from '@/api/axios.ts';

export interface PlayerCountStatus {
  supported: boolean;
  online: boolean;
  players: number | null;
  maxPlayers: number | null;
  map: string | null;
  playerNames: string[] | null;
}

export default async (uuid: string): Promise<PlayerCountStatus> => {
  const { data } = await axiosInstance.get(`/api/client/servers/${uuid}/player-count`);
  return {
    supported: data.supported,
    online: data.online,
    players: data.players ?? null,
    maxPlayers: data.max_players ?? null,
    map: data.map ?? null,
    playerNames: data.player_names ?? null,
  };
};
