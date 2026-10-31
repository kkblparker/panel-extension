import { faPlus, faTrash, faUserSlash } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Text } from '@mantine/core';
import { useEffect, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import Card from '@/elements/data-display/Card.tsx';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Group from '@/elements/layout/Group.tsx';
import SegmentedControl from '@/elements/layout/SegmentedControl.tsx';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useServerStore } from '@/stores/server.ts';
import addBan from './api/addBan.ts';
import addWhitelistEntry from './api/addWhitelistEntry.ts';
import getBans from './api/getBans.ts';
import getWhitelist from './api/getWhitelist.ts';
import removeBan from './api/removeBan.ts';
import removeWhitelistEntry from './api/removeWhitelistEntry.ts';
import { BanEntry, WhitelistEntry } from './schemas.ts';

export default function ServerWhitelist() {
  const { addToast } = useToast();
  const server = useServerStore((state) => state.server);
  const canManage = useServerCan('minecraft_whitelist.manage');

  const [view, setView] = useState<'whitelist' | 'bans'>('whitelist');

  const [whitelist, setWhitelist] = useState<WhitelistEntry[]>([]);
  const [bans, setBans] = useState<BanEntry[]>([]);
  const [loading, setLoading] = useState(true);

  const [username, setUsername] = useState('');
  const [reason, setReason] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [pendingUuid, setPendingUuid] = useState<string | null>(null);

  const load = () => {
    setLoading(true);
    Promise.all([getWhitelist(server.uuid), getBans(server.uuid)])
      .then(([whitelistEntries, banEntries]) => {
        setWhitelist(whitelistEntries);
        setBans(banEntries);
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    load();
  }, []);

  const doAdd = () => {
    const name = username.trim();
    if (!name) return;

    setSubmitting(true);

    const request =
      view === 'whitelist' ? addWhitelistEntry(server.uuid, name) : addBan(server.uuid, name, reason.trim());

    request
      .then((entries) => {
        if (view === 'whitelist') {
          setWhitelist(entries as WhitelistEntry[]);
        } else {
          setBans(entries as BanEntry[]);
        }
        addToast(
          view === 'whitelist' ? `${name} has been added to the whitelist.` : `${name} has been banned.`,
          'success',
        );
        setUsername('');
        setReason('');
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setSubmitting(false));
  };

  const doRemove = (entryUuid: string, name: string) => {
    setPendingUuid(entryUuid);

    const request =
      view === 'whitelist' ? removeWhitelistEntry(server.uuid, entryUuid) : removeBan(server.uuid, entryUuid);

    request
      .then((entries) => {
        if (view === 'whitelist') {
          setWhitelist(entries as WhitelistEntry[]);
        } else {
          setBans(entries as BanEntry[]);
        }
        addToast(view === 'whitelist' ? `${name} has been removed.` : `${name} has been unbanned.`, 'success');
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setPendingUuid(null));
  };

  return (
    <ServerContentContainer
      title='Whitelist & Bans'
      contentRight={
        <SegmentedControl
          value={view}
          onChange={(value) => setView(value as 'whitelist' | 'bans')}
          data={[
            { label: `Whitelist (${whitelist.length})`, value: 'whitelist' },
            { label: `Bans (${bans.length})`, value: 'bans' },
          ]}
        />
      }
    >
      {canManage && (
        <Card p='sm' className='mb-4'>
          <Group gap='sm' align='flex-end' wrap='wrap'>
            <TextInput
              label='Minecraft username'
              value={username}
              onChange={(e) => setUsername(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') doAdd();
              }}
              className='flex-1 min-w-[200px]'
            />
            {view === 'bans' && (
              <TextInput
                label='Reason (optional)'
                value={reason}
                onChange={(e) => setReason(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') doAdd();
                }}
                className='flex-1 min-w-[200px]'
              />
            )}
            <Button
              leftSection={<FontAwesomeIcon icon={view === 'whitelist' ? faPlus : faUserSlash} />}
              color={view === 'whitelist' ? 'blue' : 'red'}
              loading={submitting}
              disabled={!username.trim()}
              onClick={doAdd}
            >
              {view === 'whitelist' ? 'Add' : 'Ban'}
            </Button>
          </Group>
        </Card>
      )}

      {loading ? (
        <Spinner.Centered />
      ) : view === 'whitelist' ? (
        whitelist.length === 0 ? (
          <Text c='dimmed'>No players are whitelisted yet.</Text>
        ) : (
          <div className='flex flex-col gap-2'>
            {whitelist.map((entry) => (
              <Card key={entry.uuid} p='sm'>
                <Group justify='space-between'>
                  <div>
                    <Text fw={600}>{entry.name}</Text>
                    <Text size='xs' c='dimmed'>
                      {entry.uuid}
                    </Text>
                  </div>
                  <Button
                    size='xs'
                    color='red'
                    variant='light'
                    leftSection={<FontAwesomeIcon icon={faTrash} />}
                    disabled={!canManage}
                    loading={pendingUuid === entry.uuid}
                    onClick={() => doRemove(entry.uuid, entry.name)}
                  >
                    Remove
                  </Button>
                </Group>
              </Card>
            ))}
          </div>
        )
      ) : bans.length === 0 ? (
        <Text c='dimmed'>No players are banned.</Text>
      ) : (
        <div className='flex flex-col gap-2'>
          {bans.map((entry) => (
            <Card key={entry.uuid} p='sm'>
              <Group justify='space-between'>
                <div>
                  <Text fw={600}>{entry.name}</Text>
                  <Text size='xs' c='dimmed'>
                    {entry.uuid}
                  </Text>
                  {entry.reason && (
                    <Text size='xs' c='dimmed'>
                      Reason: {entry.reason}
                    </Text>
                  )}
                </div>
                <Button
                  size='xs'
                  color='blue'
                  variant='light'
                  disabled={!canManage}
                  loading={pendingUuid === entry.uuid}
                  onClick={() => doRemove(entry.uuid, entry.name)}
                >
                  Unban
                </Button>
              </Group>
            </Card>
          ))}
        </div>
      )}
    </ServerContentContainer>
  );
}
