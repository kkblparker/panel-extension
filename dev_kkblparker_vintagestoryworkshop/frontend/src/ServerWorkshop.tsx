import { Badge, Image, Text } from '@mantine/core';
import debounce from 'debounce';
import { useEffect, useMemo, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import Card from '@/elements/data-display/Card.tsx';
import { Pagination } from '@/elements/data-display/Table.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Group from '@/elements/layout/Group.tsx';
import SegmentedControl from '@/elements/layout/SegmentedControl.tsx';
import Button from '@/elements/buttons/Button.tsx';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faTrash } from '@fortawesome/free-solid-svg-icons';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useServerStore } from '@/stores/server.ts';
import getInstalledMods from './api/getInstalledMods.ts';
import installMod from './api/installMod.ts';
import searchMods from './api/searchMods.ts';
import uninstallMod from './api/uninstallMod.ts';
import { InstalledWorkshopMod, WorkshopMod, WorkshopModRelease } from './schemas.ts';
import WorkshopModDetailsModal from './WorkshopModDetailsModal.tsx';

export default function ServerWorkshop() {
  const { addToast } = useToast();
  const server = useServerStore((state) => state.server);
  const canManage = useServerCan('workshop.manage');

  const [view, setView] = useState<'browse' | 'installed'>('browse');

  const [searchInput, setSearchInput] = useState('');
  const [search, setSearch] = useState('');
  const [page, setPage] = useState(1);

  const [mods, setMods] = useState<WorkshopMod[]>([]);
  const [total, setTotal] = useState(0);
  const [perPage, setPerPage] = useState(16);
  const [loading, setLoading] = useState(true);

  const [installedMods, setInstalledMods] = useState<InstalledWorkshopMod[]>([]);
  const [installedLoading, setInstalledLoading] = useState(true);
  const [pendingFileName, setPendingFileName] = useState<string | null>(null);
  const [detailsModId, setDetailsModId] = useState<number | null>(null);

  const installedFileNames = useMemo(() => new Set(installedMods.map((mod) => mod.fileName)), [installedMods]);

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

    searchMods(server.uuid, { search: search || undefined, page })
      .then((result) => {
        setMods(result.mods ?? []);
        setTotal(result.total ?? 0);
        setPerPage(result.perPage ?? 16);
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setLoading(false));
  }, [search, page]);

  const setDebouncedSearch = useMemo(
    () =>
      debounce((value: string) => {
        setPage(1);
        setSearch(value);
      }, 400),
    [],
  );

  const doInstall = (modId: number, release: WorkshopModRelease) => {
    setPendingFileName(release.fileName);
    installMod(server.uuid, modId, { fileName: release.fileName, downloadUrl: release.downloadUrl })
      .then(() => {
        addToast(`${release.fileName} has been installed.`, 'success');
        setInstalledMods((prev) => [
          ...prev.filter((installed) => installed.fileName !== release.fileName),
          { fileName: release.fileName, size: 0, modified: new Date().toISOString() },
        ]);
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setPendingFileName(null));
  };

  const doUninstall = (fileName: string) => {
    setPendingFileName(fileName);
    uninstallMod(server.uuid, fileName)
      .then(() => {
        addToast(`${fileName} has been removed.`, 'success');
        setInstalledMods((prev) => prev.filter((installed) => installed.fileName !== fileName));
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setPendingFileName(null));
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
        <SegmentedControl
          value={view}
          onChange={(value) => setView(value as 'browse' | 'installed')}
          data={[
            { label: 'Browse', value: 'browse' },
            { label: `Installed (${installedMods.length})`, value: 'installed' },
          ]}
        />
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
              <Card key={mod.fileName} p='sm'>
                <Group justify='space-between'>
                  <div>
                    <Text fw={600}>{mod.fileName}</Text>
                    <Text size='xs' c='dimmed'>
                      {(mod.size / 1024).toFixed(1)} KB · {new Date(mod.modified).toLocaleDateString()}
                    </Text>
                  </div>
                  <Button
                    size='xs'
                    color='red'
                    variant='light'
                    leftSection={<FontAwesomeIcon icon={faTrash} />}
                    disabled={!canManage}
                    loading={pendingFileName === mod.fileName}
                    onClick={() => doUninstall(mod.fileName)}
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
            <div className='grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3'>
              {mods.map((mod) => (
                <Card key={mod.id} p='sm' hoverable className='relative' onClick={() => setDetailsModId(mod.id)}>
                  {mod.imageUrl && (
                    <Image src={mod.imageUrl} w='100%' h={120} fit='cover' radius='sm' mb='xs' alt={mod.name} />
                  )}
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
                  <Text size='xs' c='dimmed' mt='xs'>
                    {mod.downloads !== undefined ? `${mod.downloads.toLocaleString()} downloads` : ''}
                  </Text>
                </Card>
              ))}
            </div>
          )}

          <Pagination data={{ total, perPage, page, data: mods }} onPageSelect={setPage} mt='lg' />
        </>
      )}

      <WorkshopModDetailsModal
        serverUuid={server.uuid}
        modId={detailsModId}
        installedFileNames={installedFileNames}
        canManage={canManage}
        pendingFileName={pendingFileName}
        onClose={() => setDetailsModId(null)}
        onInstall={doInstall}
        onUninstall={doUninstall}
      />
    </ServerContentContainer>
  );
}
