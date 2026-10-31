import { faUsers } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useQuery } from '@tanstack/react-query';
import Card from '@/elements/data-display/Card.tsx';
import getPlayerCounts from './api/getPlayerCounts.ts';

const POLL_INTERVAL_MS = 20_000;

export default function DashboardPlayerCounts() {
  const { data } = useQuery({
    queryKey: ['dev.kkblparker.playercount', 'all'],
    queryFn: getPlayerCounts,
    refetchInterval: POLL_INTERVAL_MS,
  });

  if (!data || data.length === 0) return null;

  return (
    <Card className='mb-4'>
      <div className='flex items-center gap-2 mb-3'>
        <FontAwesomeIcon icon={faUsers} />
        <span className='font-bold'>Player Counts</span>
      </div>
      <div className='flex flex-col gap-1'>
        {data.map((server) => (
          <div key={server.serverUuid} className='flex justify-between text-sm'>
            <span className='text-(--mantine-color-dimmed)'>
              {server.serverName}
              {server.online && server.map && <span className='ml-1 text-xs'>({server.map})</span>}
            </span>
            <span>{server.online ? `${server.players ?? 0} / ${server.maxPlayers ?? '?'}` : 'Offline'}</span>
          </div>
        ))}
      </div>
    </Card>
  );
}
