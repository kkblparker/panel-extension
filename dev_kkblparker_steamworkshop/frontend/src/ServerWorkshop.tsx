import { faRotate, faTrash } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Badge, Image, Text } from '@mantine/core';
import debounce from 'debounce';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import Card from '@/elements/data-display/Card.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Select from '@/elements/input/Select.tsx';
import TextArea from '@/elements/input/TextArea.tsx';
import Group from '@/elements/layout/Group.tsx';
import SegmentedControl from '@/elements/layout/SegmentedControl.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import ConfirmationModal from '@/elements/modals/ConfirmationModal.tsx';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useServerStore } from '@/stores/server.ts';
import AddModButton from './AddModButton.tsx';
import addMod from './api/addMod.ts';
import applyChanges from './api/applyChanges.ts';
import getModList from './api/getModList.ts';
import removeMod from './api/removeMod.ts';
import searchMods from './api/searchMods.ts';
import setLoadOrder from './api/setLoadOrder.ts';
import { MOD_LIST_KIND_LABELS, ModListKind, WorkshopMod } from './schemas.ts';
import WorkshopModDetailsModal from './WorkshopModDetailsModal.tsx';

const RESULTS_PER_PAGE = 20;

const SORT_OPTIONS = [
  { value: 'popular', label: 'Most Popular' },
  { value: 'newest', label: 'Newest' },
  { value: 'recently-updated', label: 'Recently Updated' },
  { value: 'trending', label: 'Trending' },
];

export default function ServerWorkshop() {
  const { addToast } = useToast();
  const server = useServerStore((state) => state.server);
  const navigate = useNavigate();
  const canManage = useServerCan('steam_workshop.manage');

  const [view, setView] = useState<'browse' | 'list'>('browse');

  const [searchInput, setSearchInput] = useState('');
  const [search, setSearch] = useState('');
  const [sort, setSort] = useState<string | null>('popular');
  const [page, setPage] = useState(1);

  const [mods, setMods] = useState<WorkshopMod[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [modList, setModList] = useState<WorkshopMod[]>([]);
  const [modListLoading, setModListLoading] = useState(true);
  const [pendingModId, setPendingModId] = useState<string | null>(null);
  const [detailsModId, setDetailsModId] = useState<string | null>(null);
  const [applyModalOpen, setApplyModalOpen] = useState(false);

  const [loadOrderSupported, setLoadOrderSupported] = useState(false);
  const [loadOrder, setLoadOrderValue] = useState('');
  const [loadOrderInput, setLoadOrderInput] = useState('');
  const [savingLoadOrder, setSavingLoadOrder] = useState(false);

  const [availableKinds, setAvailableKinds] = useState<ModListKind[]>(['client']);
  const [kindsById, setKindsById] = useState<Record<string, ModListKind[]>>({});

  const modListIds = useMemo(() => new Set(modList.map((mod) => mod.id)), [modList]);

  const loadModList = () => {
    setModListLoading(true);
    getModList(server.uuid)
      .then((result) => {
        setModList(result.mods);
        setKindsById(result.kindsById);
        setAvailableKinds(result.availableKinds);
        setLoadOrderSupported(result.loadOrderSupported);
        setLoadOrderValue(result.loadOrder ?? '');
        setLoadOrderInput(result.loadOrder ?? '');
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setModListLoading(false));
  };

  const saveLoadOrder = (value: string) => {
    setSavingLoadOrder(true);
    setLoadOrder(server.uuid, value)
      .then(() => {
        setLoadOrderValue(value);
        setLoadOrderInput(value);
        addToast('Load order saved. Restart the server to apply it.', 'success');
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setSavingLoadOrder(false));
  };

  const appendToLoadOrder = (modId: string) => {
    const existing = loadOrder
      .split(';')
      .map((entry) => entry.trim())
      .filter(Boolean);

    if (existing.includes(modId)) {
      addToast(`${modId} is already in the load order.`, 'info');
      return;
    }

    saveLoadOrder([...existing, modId].join(';'));
  };

  useEffect(() => {
    loadModList();
  }, []);

  useEffect(() => {
    setLoading(true);

    searchMods(server.uuid, { search: search || undefined, sort: sort || undefined, page })
      .then((result) => {
        setMods(result.mods);
        setTotal(result.total);
        setAvailableKinds(result.availableKinds);
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

  const doAdd = (mod: WorkshopMod, kind: ModListKind) => {
    setPendingModId(mod.id);
    addMod(server.uuid, mod.id, kind)
      .then(() => {
        addToast(
          `${mod.name} was added as a ${MOD_LIST_KIND_LABELS[kind].toLowerCase()}. Use "Apply Changes" to download it.`,
          'success',
        );
        setModList((prev) => [...prev.filter((existing) => existing.id !== mod.id), mod]);
        setKindsById((prev) => ({
          ...prev,
          [mod.id]: prev[mod.id]?.includes(kind) ? prev[mod.id] : [...(prev[mod.id] ?? []), kind],
        }));
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setPendingModId(null));
  };

  const doRemove = (modId: string, name?: string) => {
    setPendingModId(modId);
    removeMod(server.uuid, modId)
      .then(() => {
        addToast(`${name ?? modId} was removed from the mod list.`, 'success');
        setModList((prev) => prev.filter((existing) => existing.id !== modId));
        setKindsById((prev) => {
          const { [modId]: _removed, ...rest } = prev;
          return rest;
        });
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setPendingModId(null));
  };

  const doApply = async () => {
    await applyChanges(server.uuid);
    addToast('Restarting the server to apply Workshop changes.', 'success');
    navigate(`/server/${server.uuidShort}`);
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
            onChange={(value) => setView(value as 'browse' | 'list')}
            data={[
              { label: 'Browse', value: 'browse' },
              { label: `Mod List (${modList.length})`, value: 'list' },
            ]}
          />
          {view === 'browse' && !search && (
            <Select
              data={SORT_OPTIONS}
              value={sort}
              onChange={(value) => {
                setPage(1);
                setSort(value);
              }}
              w={190}
            />
          )}
          <Button
            variant='light'
            color='orange'
            leftSection={<FontAwesomeIcon icon={faRotate} />}
            disabled={!canManage || modList.length === 0}
            onClick={() => setApplyModalOpen(true)}
          >
            Apply Changes
          </Button>
        </Group>
      }
    >
      {view === 'list' ? (
        <Stack gap='lg'>
          {modListLoading ? (
            <Spinner.Centered />
          ) : modList.length === 0 ? (
            <Text c='dimmed'>No mods in this server's Workshop list yet. Browse and add some, then Apply Changes.</Text>
          ) : (
            <div className='flex flex-col gap-2'>
              {modList.map((mod) => (
                <Card key={mod.id} p='sm' hoverable onClick={() => setDetailsModId(mod.id)}>
                  <Group justify='space-between'>
                    <Group gap='sm'>
                      {mod.imageUrl && (
                        <Image src={mod.imageUrl} h={40} w={40} radius='sm' fit='cover' alt={mod.name} />
                      )}
                      <div>
                        <Text fw={600}>{mod.name}</Text>
                        <Text size='xs' c='dimmed'>
                          {mod.id}
                        </Text>
                      </div>
                      {availableKinds.length > 1 &&
                        (kindsById[mod.id] ?? []).map((kind) => (
                          <Badge key={kind} size='xs' variant='light'>
                            {MOD_LIST_KIND_LABELS[kind]}
                          </Badge>
                        ))}
                      {mod.banned && (
                        <Badge color='red' size='xs'>
                          Banned
                        </Badge>
                      )}
                    </Group>
                    <Button
                      size='xs'
                      color='red'
                      variant='light'
                      leftSection={<FontAwesomeIcon icon={faTrash} />}
                      disabled={!canManage}
                      loading={pendingModId === mod.id}
                      onClick={(e) => {
                        e.stopPropagation();
                        doRemove(mod.id, mod.name);
                      }}
                    >
                      Remove
                    </Button>
                  </Group>
                </Card>
              ))}
            </div>
          )}

          {loadOrderSupported && (
            <div>
              <Text fw={600} size='sm'>
                Load Order
              </Text>
              <Text size='xs' c='dimmed' mb='xs'>
                This egg loads Workshop downloads and actual mod loading separately - this is the raw,
                semicolon-separated list of mod IDs (not Workshop IDs) that get loaded, in order. Check a mod's detail
                page for its Mod ID, or use the "Add to Load Order" shortcut there when one is detected.
              </Text>
              <TextArea
                value={loadOrderInput}
                onChange={(e) => setLoadOrderInput(e.currentTarget.value)}
                disabled={!canManage}
                autosize
                minRows={2}
              />
              <Group justify='flex-end' mt='xs'>
                <Button
                  size='xs'
                  variant='light'
                  disabled={!canManage || loadOrderInput === loadOrder}
                  loading={savingLoadOrder}
                  onClick={() => saveLoadOrder(loadOrderInput)}
                >
                  Save Load Order
                </Button>
              </Group>
            </div>
          )}
        </Stack>
      ) : (
        <>
          {loading ? (
            <Spinner.Centered />
          ) : mods.length === 0 ? (
            <Text c='dimmed'>No workshop items found.</Text>
          ) : (
            <div className='grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3'>
              {mods.map((mod) => {
                const inList = modListIds.has(mod.id);

                return (
                  <Card key={mod.id} p='sm' hoverable onClick={() => setDetailsModId(mod.id)}>
                    {mod.imageUrl && <Image src={mod.imageUrl} height={90} radius='sm' mb='xs' alt={mod.name} />}
                    <Text fw={600} size='sm' lineClamp={1}>
                      {mod.name}
                    </Text>
                    {mod.tags.length > 0 && (
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
                      {inList ? (
                        <Button
                          size='xs'
                          color='red'
                          variant='light'
                          disabled={!canManage}
                          loading={pendingModId === mod.id}
                          onClick={(e) => {
                            e.stopPropagation();
                            doRemove(mod.id, mod.name);
                          }}
                        >
                          Remove
                        </Button>
                      ) : (
                        <AddModButton
                          availableKinds={availableKinds}
                          disabled={!canManage}
                          loading={pendingModId === mod.id}
                          onAdd={(kind) => doAdd(mod, kind)}
                        />
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
            <Button variant='subtle' disabled={page * RESULTS_PER_PAGE >= total} onClick={() => setPage((p) => p + 1)}>
              Next
            </Button>
          </Group>
        </>
      )}

      <WorkshopModDetailsModal
        serverUuid={server.uuid}
        modId={detailsModId}
        inList={detailsModId !== null && modListIds.has(detailsModId)}
        kinds={detailsModId !== null ? (kindsById[detailsModId] ?? []) : []}
        canManage={canManage}
        pending={detailsModId !== null && pendingModId === detailsModId}
        onClose={() => setDetailsModId(null)}
        onAdd={doAdd}
        onRemove={doRemove}
        onAppendToLoadOrder={loadOrderSupported ? appendToLoadOrder : undefined}
      />

      <ConfirmationModal
        title='Apply Workshop Changes'
        confirm='Restart & Apply'
        confirmColor='orange'
        opened={applyModalOpen}
        onClose={() => setApplyModalOpen(false)}
        onConfirmed={doApply}
      >
        <Text size='sm'>
          This restarts the server so it checks the {modList.length} item{modList.length === 1 ? '' : 's'} currently in
          the mod list for downloads/updates via <code>steamcmd</code> on startup, same as it already does for server
          updates.
        </Text>
        <Text size='sm' mt='sm' c='dimmed'>
          The server will be briefly unavailable while it restarts. This uses the same Steam account already configured
          for this server — no separate login is needed.
        </Text>
      </ConfirmationModal>
    </ServerContentContainer>
  );
}
