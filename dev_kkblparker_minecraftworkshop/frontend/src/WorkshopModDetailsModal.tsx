import { faDownload, faTrash } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Badge, Image, Text } from '@mantine/core';
import { useEffect, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Group from '@/elements/layout/Group.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import { Modal, ModalFooter } from '@/elements/modals/Modal.tsx';
import { useToast } from '@/providers/ToastProvider.tsx';
import getMod from './api/getMod.ts';
import { modId, WorkshopMod } from './schemas.ts';

interface Props {
  serverUuid: string;
  modId: string | null;
  installed: boolean;
  canManage: boolean;
  pending: boolean;
  onClose: () => void;
  onInstall: (mod: WorkshopMod) => void;
  onUninstall: (modId: string, name?: string) => void;
}

export default function WorkshopModDetailsModal({
  serverUuid,
  modId: activeModId,
  installed,
  canManage,
  pending,
  onClose,
  onInstall,
  onUninstall,
}: Props) {
  const { addToast } = useToast();
  const [mod, setMod] = useState<WorkshopMod | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!activeModId) {
      setMod(null);
      return;
    }

    setLoading(true);
    getMod(serverUuid, activeModId)
      .then(setMod)
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setLoading(false));
  }, [activeModId]);

  return (
    <Modal title={mod?.title ?? 'Mod Details'} opened={activeModId !== null} onClose={onClose} size='lg'>
      {loading || !mod ? (
        <Spinner.Centered />
      ) : (
        <Stack gap='sm'>
          {mod.icon_url && <Image src={mod.icon_url} radius='sm' mah={200} w={200} fit='contain' alt={mod.title} />}

          <Group justify='space-between'>
            <div>
              {mod.author && (
                <Text size='sm' c='dimmed'>
                  by {mod.author}
                </Text>
              )}
            </div>
            <Group gap='lg'>
              {mod.downloads !== undefined && (
                <div>
                  <Text size='xs' c='dimmed'>
                    Downloads
                  </Text>
                  <Text size='sm'>{mod.downloads.toLocaleString()}</Text>
                </div>
              )}
              {(mod.follows ?? mod.followers) !== undefined && (
                <div>
                  <Text size='xs' c='dimmed'>
                    Follows
                  </Text>
                  <Text size='sm'>{(mod.follows ?? mod.followers)!.toLocaleString()}</Text>
                </div>
              )}
            </Group>
          </Group>

          {mod.categories && mod.categories.length > 0 && (
            <Group gap='xs'>
              {mod.categories.map((category) => (
                <Badge key={category} size='sm' variant='light'>
                  {category}
                </Badge>
              ))}
            </Group>
          )}

          {(mod.body ?? mod.description) && (
            <Text size='sm' style={{ whiteSpace: 'pre-wrap' }}>
              {mod.body ?? mod.description}
            </Text>
          )}

          <ModalFooter>
            {installed ? (
              <Button
                color='red'
                variant='light'
                leftSection={<FontAwesomeIcon icon={faTrash} />}
                disabled={!canManage}
                loading={pending}
                onClick={() => onUninstall(modId(mod), mod.title)}
              >
                Uninstall
              </Button>
            ) : (
              <Button
                color='blue'
                leftSection={<FontAwesomeIcon icon={faDownload} />}
                disabled={!canManage}
                loading={pending}
                onClick={() => onInstall(mod)}
              >
                Install
              </Button>
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
