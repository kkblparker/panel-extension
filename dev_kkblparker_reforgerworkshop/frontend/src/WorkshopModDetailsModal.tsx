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
import { openUrl } from '@/lib/network/url.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import getMod from './api/getMod.ts';
import { WorkshopMod } from './schemas.ts';

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
  modId,
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
    if (!modId) {
      setMod(null);
      return;
    }

    setLoading(true);
    getMod(serverUuid, modId)
      .then(setMod)
      .catch((error) => addToast(httpErrorToHuman(error), 'error'))
      .finally(() => setLoading(false));
  }, [modId]);

  return (
    <Modal title={mod?.name ?? 'Mod Details'} opened={modId !== null} onClose={onClose} size='lg'>
      {loading || !mod ? (
        <Spinner.Centered />
      ) : (
        <Stack gap='sm'>
          {mod.imageUrl && <Image src={mod.imageUrl} radius='sm' mah={280} fit='contain' alt={mod.name} />}

          <Group justify='space-between'>
            <div>
              {mod.author && (
                <Text size='sm' c='dimmed'>
                  by {mod.author}
                  {mod.version ? ` · v${mod.version}` : ''}
                </Text>
              )}
              <Text size='xs' c='dimmed'>
                {mod.id}
              </Text>
            </div>
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
              {mod.downloadCount !== undefined && (
                <div>
                  <Text size='xs' c='dimmed'>
                    Downloads
                  </Text>
                  <Text size='sm'>{mod.downloadCount.toLocaleString()}</Text>
                </div>
              )}
            </Group>
          </Group>

          {mod.tags && mod.tags.length > 0 && (
            <Group gap='xs'>
              {mod.tags.map((tag) => (
                <Badge key={tag} size='sm' variant='light'>
                  {tag}
                </Badge>
              ))}
            </Group>
          )}

          {(mod.description ?? mod.summary) && (
            <Text size='sm' style={{ whiteSpace: 'pre-wrap' }}>
              {mod.description ?? mod.summary}
            </Text>
          )}

          {mod.dependencies && mod.dependencies.length > 0 && (
            <div>
              <Text size='sm' fw={600} mb={4}>
                Dependencies
              </Text>
              <Stack gap={4}>
                {mod.dependencies.map((dependency, index) => (
                  <Text key={dependency.id ?? index} size='sm' c='dimmed'>
                    {dependency.name ?? dependency.id}
                    {dependency.version ? ` · v${dependency.version}` : ''}
                  </Text>
                ))}
              </Stack>
            </div>
          )}

          <ModalFooter>
            {mod.workshopUrl && (
              <Button variant='default' onClick={() => openUrl(mod.workshopUrl!)}>
                View on Workshop
              </Button>
            )}
            {installed ? (
              <Button
                color='red'
                variant='light'
                leftSection={<FontAwesomeIcon icon={faTrash} />}
                disabled={!canManage}
                loading={pending}
                onClick={() => onUninstall(mod.id, mod.name)}
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
