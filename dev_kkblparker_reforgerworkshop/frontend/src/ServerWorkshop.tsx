import { faDownload, faTrash } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Badge, Image, Text } from '@mantine/core';
import debounce from 'debounce';
import { useEffect, useMemo, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import Card from '@/elements/data-display/Card.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Select from '@/elements/input/Select.tsx';
import Group from '@/elements/layout/Group.tsx';
import SegmentedControl from '@/elements/layout/SegmentedControl.tsx';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useServerStore } from '@/stores/server.ts';
import getInstalledMods from './api/getInstalledMods.ts';
import installMod from './api/installMod.ts';
import searchMods from './api/searchMods.ts';
import uninstallMod from './api/uninstallMod.ts';
import { InstalledWorkshopMod, WorkshopMod } from './schemas.ts';
import WorkshopModDetailsModal from './WorkshopModDetailsModal.tsx';

const RESULTS_PER_PAGE = 20;

const SORT_OPTIONS = [
  { value: 'popularity', label: 'Popularity' },
  { value: 'newest', label: 'Newest' },
  { value: 'recently-updated', label: 'Recently Updated' },
  { value: 'highest-rated', label: 'Highest Rated' },
  { value: 'subscribers', label: 'Subscribers' },
  { value: 'name', label: 'Name' },
];

export default function ServerWorkshop() {
  const { addToast } = useToast();
  const server = useServerStore((state) => state.server);
  const canManage = useServerCan('workshop.manage');

  const [view, setView] = useState<'browse' | 'installed'>('browse');

  const [searchInput, setSearchInput] = useState('');
  const [search, setSearch] = useState('');
  const [sort, setSort] = useState<string | null>('popularity');
  const [page, setPage] = useState(1);

  const [mods, setMods] = useState<WorkshopMod[]>([]);
  const [hasNextPage, setHasNextPage] = useState(false);
  const [loading, setLoading] = useState(true);
  const [installedMods, setInstalledMods] = useState<InstalledWorkshopMod[]>([]);
  const [installedLoading, setInstalledLoading] = useState(true);
  const [pendingModId, setPendingModId] = useState<string | null>(null);
  const [detailsModId, setDetailsModId] = useState<string | null>(null);

  const installedModIds = useMemo(() => new Set(installedMods.map((mod) => mod.modId)), [installedMods]);

  const loadInstalled = () => {
    setInstalledLoading(true);
    getInstalledMods(server.uuid)
      .then(setInstalledMods)
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setInstalledLoading(false));
  };

  useEffect(() => {
    loadInstalled();
  }, []);

  useEffect(() => {
    setLoading(true);

    searchMods(server.uuid, { search: search || undefined, sort: sort || undefined, page })
      .then((result) => {
        const data = result.data ?? [];
        setMods(data);
        setHasNextPage(data.length >= RESULTS_PER_PAGE);
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setLoading(false));
  }, [search, sort, page]);

  const setDebouncedSearch = useMemo(
    () =>
      debounce((value: string) => {
        setPage(1);
        setSearch(value);
      }, 400),
    [],
  );

  const doInstall = (mod: WorkshopMod) => {
    setPendingModId(mod.id);
    installMod(server.uuid, mod.id, { name: mod.name, version: mod.version })
      .then(() => {
        addToast(`${mod.name} has been installed.`, 'success');
        setInstalledMods((prev) => [
          ...prev.filter((installed) => installed.modId !== mod.id),
          { modId: mod.id, name: mod.name, version: mod.version },
        ]);
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setPendingModId(null));
  };

  const doUninstall = (modId: string, name?: string) => {
    setPendingModId(modId);
    uninstallMod(server.uuid, modId)
      .then(() => {
        addToast(`${name ?? modId} has been removed.`, 'success');
        setInstalledMods((prev) => prev.filter((installed) => installed.modId !== modId));
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setPendingModId(null));
  };

  return (
    <ServerContentContainer
      title='Workshop'
      search={view === 'browse' ? searchInput : undefined}
      setSearch={
        view === 'browse'
          ? (value) => {
              const resolved = typeof value === 'function' ? value(searchInput) : value;
              setSearchInput(resolved);
              setDebouncedSearch(resolved);
            }
          : undefined
      }
      contentRight={
        <Group gap='sm'>
          <SegmentedControl
            value={view}
            onChange={(value) => setView(value as 'browse' | 'installed')}
            data={[
              { label: 'Browse', value: 'browse' },
              { label: `Installed (${installedMods.length})`, value: 'installed' },
            ]}
          />
          {view === 'browse' && (
            <Select
              data={SORT_OPTIONS}
              value={sort}
              onChange={(value) => {
                setPage(1);
                setSort(value);
              }}
              w={200}
            />
          )}
        </Group>
      }
    >
      {view === 'installed' ? (
        installedLoading ? (
          <Spinner.Centered />
        ) : installedMods.length === 0 ? (
          <Text c='dimmed'>No mods installed on this server yet.</Text>
        ) : (
          <div className='flex flex-col gap-2'>
            {installedMods.map((mod) => (
              <Card key={mod.modId} p='sm' hoverable onClick={() => setDetailsModId(mod.modId)}>
                <Group justify='space-between'>
                  <div>
                    <Text fw={600}>{mod.name ?? mod.modId}</Text>
                    <Text size='xs' c='dimmed'>
                      {mod.modId}
                      {mod.version ? ` · v${mod.version}` : ''}
                    </Text>
                  </div>
                  <Button
                    size='xs'
                    color='red'
                    variant='light'
                    leftSection={<FontAwesomeIcon icon={faTrash} />}
                    disabled={!canManage}
                    loading={pendingModId === mod.modId}
                    onClick={(e) => {
                      e.stopPropagation();
                      doUninstall(mod.modId, mod.name);
                    }}
                  >
                    Uninstall
                  </Button>
                </Group>
              </Card>
            ))}
          </div>
        )
      ) : (
        <>
          {loading ? (
            <Spinner.Centered />
          ) : mods.length === 0 ? (
            <Text c='dimmed'>No workshop mods found.</Text>
          ) : (
            <div className='grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3'>
              {mods.map((mod) => {
                const installed = installedModIds.has(mod.id);

                return (
                  <Card key={mod.id} p='sm' hoverable onClick={() => setDetailsModId(mod.id)}>
                    {mod.imageUrl && <Image src={mod.imageUrl} height={90} radius='sm' mb='xs' alt={mod.name} />}
                    <Text fw={600} size='sm' lineClamp={1}>
                      {mod.name}
                    </Text>
                    {mod.author && (
                      <Text size='xs' c='dimmed' lineClamp={1}>
                        by {mod.author}
                      </Text>
                    )}
                    {mod.tags && mod.tags.length > 0 && (
                      <Group gap={4} mt={4}>
                        {mod.tags.slice(0, 2).map((tag) => (
                          <Badge key={tag} size='xs' variant='light'>
                            {tag}
                          </Badge>
                        ))}
                      </Group>
                    )}
                    <Group justify='space-between' mt='xs' wrap='nowrap'>
                      <Text size='xs' c='dimmed'>
                        {mod.sizeFormatted ?? ''}
                      </Text>
                      {installed ? (
                        <Button
                          size='xs'
                          color='red'
                          variant='light'
                          disabled={!canManage}
                          loading={pendingModId === mod.id}
                          onClick={(e) => {
                            e.stopPropagation();
                            doUninstall(mod.id, mod.name);
                          }}
                        >
                          Uninstall
                        </Button>
                      ) : (
                        <Button
                          size='xs'
                          color='blue'
                          leftSection={<FontAwesomeIcon icon={faDownload} />}
                          disabled={!canManage}
                          loading={pendingModId === mod.id}
                          onClick={(e) => {
                            e.stopPropagation();
                            doInstall(mod);
                          }}
                        >
                          Install
                        </Button>
                      )}
                    </Group>
                  </Card>
                );
              })}
            </div>
          )}

          <Group justify='center' mt='lg'>
            <Button variant='subtle' disabled={page <= 1} onClick={() => setPage((p) => p - 1)}>
              Previous
            </Button>
            <Text size='sm'>{page}</Text>
            <Button variant='subtle' disabled={!hasNextPage} onClick={() => setPage((p) => p + 1)}>
              Next
            </Button>
          </Group>
        </>
      )}

      <WorkshopModDetailsModal
        serverUuid={server.uuid}
        modId={detailsModId}
        installed={detailsModId !== null && installedModIds.has(detailsModId)}
        canManage={canManage}
        pending={detailsModId !== null && pendingModId === detailsModId}
        onClose={() => setDetailsModId(null)}
        onInstall={doInstall}
        onUninstall={doUninstall}
      />
    </ServerContentContainer>
  );
}
