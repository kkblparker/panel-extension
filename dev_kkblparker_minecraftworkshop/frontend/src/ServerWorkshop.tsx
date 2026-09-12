import { faDownload, faTrash } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Image, Text } from '@mantine/core';
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
import { InstalledWorkshopMod, modFollowers, modId, WorkshopMod } from './schemas.ts';
import WorkshopModDetailsModal from './WorkshopModDetailsModal.tsx';

const SORT_OPTIONS = [
  { value: 'relevance', label: 'Relevance' },
  { value: 'downloads', label: 'Downloads' },
  { value: 'follows', label: 'Follows' },
  { value: 'newest', label: 'Newest' },
  { value: 'updated', label: 'Recently Updated' },
];

export default function ServerWorkshop() {
  const { addToast } = useToast();
  const server = useServerStore((state) => state.server);
  const canManage = useServerCan('minecraft_workshop.manage');

  const [view, setView] = useState<'browse' | 'installed'>('browse');

  const [searchInput, setSearchInput] = useState('');
  const [search, setSearch] = useState('');
  const [sort, setSort] = useState<string | null>('relevance');
  const [page, setPage] = useState(1);

  const [mods, setMods] = useState<WorkshopMod[]>([]);
  const [totalHits, setTotalHits] = useState(0);
  const [loading, setLoading] = useState(true);
  const [installedMods, setInstalledMods] = useState<InstalledWorkshopMod[]>([]);
  const [installedLoading, setInstalledLoading] = useState(true);
  const [pendingModId, setPendingModId] = useState<string | null>(null);
  const [detailsModId, setDetailsModId] = useState<string | null>(null);

  const installedModIds = useMemo(() => new Set(installedMods.map((mod) => mod.project_id)), [installedMods]);

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
        setMods(result.hits ?? []);
        setTotalHits(result.total_hits ?? 0);
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
    const id = modId(mod);
    setPendingModId(id);
    installMod(server.uuid, id)
      .then(() => {
        addToast(`${mod.title} has been installed.`, 'success');
        loadInstalled();
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setPendingModId(null));
  };

  const doUninstall = (id: string, name?: string) => {
    setPendingModId(id);
    uninstallMod(server.uuid, id)
      .then(() => {
        addToast(`${name ?? id} has been removed.`, 'success');
        setInstalledMods((prev) => prev.filter((installed) => installed.project_id !== id));
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setPendingModId(null));
  };

  const hasNextPage = page * 20 < totalHits;

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
          <Text c='dimmed'>No plugins/mods installed on this server yet.</Text>
        ) : (
          <div className='flex flex-col gap-2'>
            {installedMods.map((mod) => (
              <Card key={mod.project_id} p='sm' hoverable onClick={() => setDetailsModId(mod.project_id)}>
                <Group justify='space-between'>
                  <div>
                    <Text fw={600}>{mod.name ?? mod.project_id}</Text>
                    <Text size='xs' c='dimmed'>
                      {mod.file_name}
                    </Text>
                  </div>
                  <Button
                    size='xs'
                    color='red'
                    variant='light'
                    leftSection={<FontAwesomeIcon icon={faTrash} />}
                    disabled={!canManage}
                    loading={pendingModId === mod.project_id}
                    onClick={(e) => {
                      e.stopPropagation();
                      doUninstall(mod.project_id, mod.name);
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
            <Text c='dimmed'>No results found.</Text>
          ) : (
            <div className='grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3'>
              {mods.map((mod) => {
                const id = modId(mod);
                const installed = installedModIds.has(id);
                const followers = modFollowers(mod);

                return (
                  <Card key={id} p='sm' hoverable onClick={() => setDetailsModId(id)}>
                    {mod.icon_url && <Image src={mod.icon_url} height={90} radius='sm' mb='xs' alt={mod.title} />}
                    <Text fw={600} size='sm' lineClamp={1}>
                      {mod.title}
                    </Text>
                    {mod.description && (
                      <Text size='xs' c='dimmed' lineClamp={2}>
                        {mod.description}
                      </Text>
                    )}
                    <Group justify='space-between' mt='xs' wrap='nowrap'>
                      <Text size='xs' c='dimmed'>
                        {mod.downloads !== undefined ? `${mod.downloads.toLocaleString()} downloads` : ''}
                        {followers !== undefined ? ` · ${followers.toLocaleString()} follows` : ''}
                      </Text>
                      {installed ? (
                        <Button
                          size='xs'
                          color='red'
                          variant='light'
                          disabled={!canManage}
                          loading={pendingModId === id}
                          onClick={(e) => {
                            e.stopPropagation();
                            doUninstall(id, mod.title);
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
                          loading={pendingModId === id}
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
