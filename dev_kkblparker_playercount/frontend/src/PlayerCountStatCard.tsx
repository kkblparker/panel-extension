import { faUsers } from '@fortawesome/free-solid-svg-icons';
import { useQuery } from '@tanstack/react-query';
import StatCard from '@/elements/data-display/StatCard.tsx';
import { useServerStore } from '@/stores/server.ts';
import getPlayerCount from './api/getPlayerCount.ts';

const POLL_INTERVAL_MS = 20_000;

export default function PlayerCountStatCard() {
  const server = useServerStore((state) => state.server);

  const { data } = useQuery({
    queryKey: ['dev.kkblparker.playercount', server.uuid],
    queryFn: () => getPlayerCount(server.uuid),
    refetchInterval: POLL_INTERVAL_MS,
  });

  if (!data?.supported) return null;

  return (
    <StatCard
      icon={faUsers}
      label='Players'
      order={45}
      value={data.online ? `${data.players ?? 0} / ${data.maxPlayers ?? '?'}` : 'Offline'}
      details={data.online ? data.map : null}
    />
  );
}
