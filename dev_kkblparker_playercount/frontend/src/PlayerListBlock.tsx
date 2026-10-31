import { faUser, faUsers } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useQuery } from '@tanstack/react-query';
import ChartBlock from '@/elements/charts/ChartBlock.tsx';
import { useServerStore } from '@/stores/server.ts';
import getPlayerCount from './api/getPlayerCount.ts';

const POLL_INTERVAL_MS = 20_000;

export default function PlayerListBlock() {
  const server = useServerStore((state) => state.server);

  const { data } = useQuery({
    queryKey: ['dev.kkblparker.playercount', server.uuid],
    queryFn: () => getPlayerCount(server.uuid),
    refetchInterval: POLL_INTERVAL_MS,
  });

  if (!data?.supported) return null;

  const overlayIcon = <FontAwesomeIcon icon={faUsers} className='text-2xl' />;
  const overlayLabel = !data.online ? 'Offline' : undefined;

  return (
    <ChartBlock
      icon={<FontAwesomeIcon icon={faUsers} />}
      title='Online Players'
      value={data.online ? `${data.players ?? 0} / ${data.maxPlayers ?? '?'}` : undefined}
      overlayIcon={overlayIcon}
      overlayLabel={overlayLabel}
    >
      {data.playerNames === null ? (
        <div className='flex h-full flex-col items-center justify-center gap-2 text-(--mantine-color-dimmed)'>
          <FontAwesomeIcon icon={faUsers} className='text-2xl' />
          <span className='text-sm'>Player names unavailable for this server</span>
        </div>
      ) : data.playerNames.length === 0 ? (
        <div className='flex h-full flex-col items-center justify-center gap-2 text-(--mantine-color-dimmed)'>
          <FontAwesomeIcon icon={faUsers} className='text-2xl' />
          <span className='text-sm'>No players connected</span>
          <span className='text-xs'>Players appear here as they join</span>
        </div>
      ) : (
        <div className='flex max-h-60 flex-col gap-2 overflow-y-auto'>
          {data.playerNames.map((name) => (
            <div key={name} className='flex items-center gap-2 text-sm'>
              <FontAwesomeIcon icon={faUser} className='text-(--mantine-color-dimmed)' />
              <span className='truncate'>{name}</span>
            </div>
          ))}
        </div>
      )}
    </ChartBlock>
  );
}
