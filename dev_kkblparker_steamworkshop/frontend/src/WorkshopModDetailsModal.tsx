import { faPlus, faTrash } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Alert, Badge, Image, Text } from '@mantine/core';
import { useEffect, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Group from '@/elements/layout/Group.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import { Modal, ModalFooter } from '@/elements/modals/Modal.tsx';
import { openUrl } from '@/lib/network/url.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import AddModButton from './AddModButton.tsx';
import getMod from './api/getMod.ts';
import { MOD_LIST_KIND_LABELS, ModListKind, WorkshopMod } from './schemas.ts';

interface Props {
  serverUuid: string;
  modId: string | null;
  inList: boolean;
  /** Which list(s) this mod is currently in, if `inList` - for the kind badge(s). */
  kinds: ModListKind[];
  canManage: boolean;
  pending: boolean;
  onClose: () => void;
  onAdd: (mod: WorkshopMod, kind: ModListKind) => void;
  onRemove: (modId: string, name?: string) => void;
  /** Only passed when the egg supports a separate load-order list (see ServerWorkshop.tsx). */
  onAppendToLoadOrder?: (modId: string) => void;
}

export default function WorkshopModDetailsModal({
  serverUuid,
  modId,
  inList,
  kinds,
  canManage,
  pending,
  onClose,
  onAdd,
  onRemove,
  onAppendToLoadOrder,
}: Props) {
  const { addToast } = useToast();
  const [mod, setMod] = useState<WorkshopMod | null>(null);
  const [availableKinds, setAvailableKinds] = useState<ModListKind[]>(['client']);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!modId) {
      setMod(null);
      return;
    }

    setLoading(true);
    getMod(serverUuid, modId)
      .then((result) => {
        setMod(result.mod);
        setAvailableKinds(result.availableKinds);
      })
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setLoading(false));
  }, [modId]);

  return (
    <Modal title={mod?.name ?? 'Workshop Item'} opened={modId !== null} onClose={onClose} size='lg'>
      {loading || !mod ? (
        <Spinner.Centered />
      ) : (
        <Stack gap='sm'>
          {mod.banned && (
            <Alert color='red'>
              This item has been banned from the Workshop{mod.banReason ? `: ${mod.banReason}` : '.'}
            </Alert>
          )}

          {mod.imageUrl && <Image src={mod.imageUrl} radius='sm' mah={280} fit='contain' alt={mod.name} />}

          <Group justify='space-between'>
            <Text size='xs' c='dimmed'>
              {mod.id}
            </Text>
            <Group gap='lg'>
              {mod.sizeFormatted && (
                <div>
                  <Text size='xs' c='dimmed'>
                    Size
                  </Text>
                  <Text size='sm'>{mod.sizeFormatted}</Text>
                </div>
              )}
              {mod.rating !== undefined && (
                <div>
                  <Text size='xs' c='dimmed'>
                    Rating
                  </Text>
                  <Text size='sm'>
                    {Math.round(mod.rating * 100)}%{mod.ratingCount ? ` (${mod.ratingCount})` : ''}
                  </Text>
                </div>
              )}
              {mod.subscriberCount !== undefined && (
                <div>
                  <Text size='xs' c='dimmed'>
                    Subscribers
                  </Text>
                  <Text size='sm'>{mod.subscriberCount.toLocaleString()}</Text>
                </div>
              )}
            </Group>
          </Group>

          {(inList || mod.tags.length > 0) && (
            <Group gap='xs'>
              {inList &&
                kinds.map((kind) => (
                  <Badge key={kind} size='sm' color='blue'>
                    {MOD_LIST_KIND_LABELS[kind]}
                  </Badge>
                ))}
              {mod.tags.map((tag) => (
                <Badge key={tag} size='sm' variant='light'>
                  {tag}
                </Badge>
              ))}
            </Group>
          )}

          {onAppendToLoadOrder && mod.modIdHint && (
            <Alert color='blue' variant='light'>
              <Group justify='space-between' align='center'>
                <Text size='sm'>
                  Detected Mod ID:{' '}
                  <Text span fw={600}>
                    {mod.modIdHint}
                  </Text>{' '}
                  (scraped from this item's description - double check it's correct)
                </Text>
                <Button
                  size='xs'
                  variant='light'
                  leftSection={<FontAwesomeIcon icon={faPlus} />}
                  disabled={!canManage}
                  onClick={() => onAppendToLoadOrder(mod.modIdHint!)}
                >
                  Add to Load Order
                </Button>
              </Group>
            </Alert>
          )}

          {mod.description && (
            <Text size='sm' style={{ whiteSpace: 'pre-wrap' }}>
              {mod.description}
            </Text>
          )}

          <ModalFooter>
            <Button variant='default' onClick={() => openUrl(mod.workshopUrl)}>
              View on Steam
            </Button>
            {inList ? (
              <Button
                color='red'
                variant='light'
                leftSection={<FontAwesomeIcon icon={faTrash} />}
                disabled={!canManage}
                loading={pending}
                onClick={() => onRemove(mod.id, mod.name)}
              >
                Remove
              </Button>
            ) : (
              <AddModButton
                availableKinds={availableKinds}
                size='sm'
                disabled={!canManage}
                loading={pending}
                onAdd={(kind) => onAdd(mod, kind)}
              />
            )}
            <Button variant='default' onClick={onClose}>
              Close
            </Button>
          </ModalFooter>
        </Stack>
      )}
    </Modal>
  );
}
