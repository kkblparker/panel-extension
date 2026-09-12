import { Alert, Anchor, Button, Group, PasswordInput, Stack, Text, Title } from '@mantine/core';
import { useEffect, useState } from 'react';
import { axiosInstance, httpErrorToHuman } from '@/api/axios.ts';
import { useToast } from '@/providers/ToastProvider.tsx';

const API_BASE = '/api/admin/steamworkshop';

interface SettingsResponse {
  api_key_set: boolean;
}

export default function SteamWorkshopConfigPage() {
  const { addToast } = useToast();

  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const [apiKeySet, setApiKeySet] = useState(false);
  const [apiKeyInput, setApiKeyInput] = useState('');

  const loadSettings = () => {
    setLoading(true);
    setLoadError(null);

    axiosInstance
      .get<SettingsResponse>(`${API_BASE}/settings`)
      .then(({ data }) => setApiKeySet(data.api_key_set))
      .catch((err) => setLoadError(httpErrorToHuman(err)))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    loadSettings();
  }, []);

  const save = () => {
    setSaving(true);

    axiosInstance
      .put(`${API_BASE}/settings`, { api_key: apiKeyInput === '' ? null : apiKeyInput })
      .then(() => {
        addToast('Steam Workshop settings saved.', 'success');
        setApiKeyInput('');
        loadSettings();
      })
      .catch((err) => addToast(httpErrorToHuman(err), 'error'))
      .finally(() => setSaving(false));
  };

  if (loading) {
    return <Text>Loading…</Text>;
  }

  return (
    <Stack gap='md' p='md'>
      <Title order={3}>Steam Workshop</Title>
      {loadError && <Alert color='red'>{loadError}</Alert>}

      <Text size='sm' c='dimmed'>
        A Steam Web API key is needed to search the Workshop from a server's Workshop tab. Fetching details for a mod
        already in a server's mod list works without a key. Get a free key at{' '}
        <Anchor href='https://steamcommunity.com/dev/apikey' target='_blank' rel='noreferrer'>
          steamcommunity.com/dev/apikey
        </Anchor>{' '}
        — a standard key, not a restricted publisher key.
      </Text>

      <PasswordInput
        label='Steam Web API key'
        placeholder={apiKeySet ? 'Key is set — leave blank to keep it' : 'No key set yet'}
        value={apiKeyInput}
        onChange={(e) => setApiKeyInput(e.currentTarget.value)}
      />

      <Group>
        <Button onClick={save} loading={saving}>
          Save
        </Button>
      </Group>
    </Stack>
  );
}
