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
import { WorkshopMod, WorkshopModRelease } from './schemas.ts';

interface Props {
  serverUuid: string;
  modId: number | null;
  installedFileNames: Set<string>;
  canManage: boolean;
  pendingFileName: string | null;
  onClose: () => void;
  onInstall: (modId: number, release: WorkshopModRelease) => void;
  onUninstall: (fileName: string) => void;
}

export default function WorkshopModDetailsModal({
  serverUuid,
  modId,
  installedFileNames,
  canManage,
  pendingFileName,
  onClose,
  onInstall,
  onUninstall,
}: Props) {
  const { addToast } = useToast();
  const [mod, setMod] = useState<WorkshopMod | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (modId === null) {
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
                </Text>
              )}
              <Text size='xs' c='dimmed'>
                #{mod.id}
              </Text>
            </div>
            {mod.downloads !== undefined && (
              <div>
                <Text size='xs' c='dimmed'>
                  Downloads
                </Text>
                <Text size='sm'>{mod.downloads.toLocaleString()}</Text>
              </div>
            )}
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

          {mod.summary && <Text size='sm'>{mod.summary}</Text>}

          <div>
            <Text size='sm' fw={600} mb={4}>
              Releases
            </Text>
            <Stack gap={4}>
              {(mod.releases ?? []).map((release) => {
                const installed = installedFileNames.has(release.fileName);
                const isPending = pendingFileName === release.fileName;

                return (
                  <Group key={release.releaseId} justify='space-between' wrap='nowrap'>
                    <div>
                      <Text size='sm'>v{release.version}</Text>
                      {release.gameVersions && release.gameVersions.length > 0 && (
                        <Text size='xs' c='dimmed'>
                          {release.gameVersions.join(', ')}
                        </Text>
                      )}
                    </div>
                    {installed ? (
                      <Button
                        size='xs'
                        color='red'
                        variant='light'
                        leftSection={<FontAwesomeIcon icon={faTrash} />}
                        disabled={!canManage}
                        loading={isPending}
                        onClick={() => onUninstall(release.fileName)}
                      >
                        Uninstall
                      </Button>
                    ) : (
                      <Button
                        size='xs'
                        color='blue'
                        leftSection={<FontAwesomeIcon icon={faDownload} />}
                        disabled={!canManage}
                        loading={isPending}
                        onClick={() => onInstall(mod.id, release)}
                      >
                        Install
                      </Button>
                    )}
                  </Group>
                );
              })}
              {(mod.releases ?? []).length === 0 && (
                <Text size='sm' c='dimmed'>
                  No releases available for this mod.
                </Text>
              )}
            </Stack>
          </div>

          <ModalFooter>
            {mod.workshopUrl && (
              <Button variant='default' onClick={() => openUrl(mod.workshopUrl!)}>
                View on Mod DB
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
