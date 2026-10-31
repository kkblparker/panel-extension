import { axiosInstance } from '@/api/axios.ts';

export interface ServerPlayerCount {
  serverUuid: string;
  serverName: string;
  online: boolean;
  players: number | null;
  maxPlayers: number | null;
  map: string | null;
}

export default async (): Promise<ServerPlayerCount[]> => {
  const { data } = await axiosInstance.get('/api/client/player-counts');
  return (data.servers as Record<string, unknown>[]).map((s) => ({
    serverUuid: s.server_uuid as string,
    serverName: s.server_name as string,
    online: s.online as boolean,
    players: (s.players as number | null) ?? null,
    maxPlayers: (s.max_players as number | null) ?? null,
    map: (s.map as string | null) ?? null,
  }));
};
