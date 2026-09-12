import { Alert, Badge, Button, Group, NumberInput, PasswordInput, Select, Stack, Switch, Table, Text, TextInput, Title } from '@mantine/core';
import { useEffect, useState } from 'react';
import { axiosInstance, httpErrorToHuman } from '@/api/axios.ts';
import { useToast } from '@/providers/ToastProvider.tsx';

const API_BASE = '/api/admin/pfsensesync';

interface SettingsResponse {
  enabled: boolean;
  base_url: string;
  api_key_set: boolean;
  insecure_tls: boolean;
  interface: string;
  protocol: string;
  source: string;
  destination: string;
  associated_rule: string;
  interval_secs: number;
  debug: boolean;
}

interface InterfaceOption {
  id: string;
  description: string;
  address: string | null;
}

interface VirtualIpOption {
  address: string;
  interface: string;
  description: string;
  mode: string;
}

// Always available regardless of what pfSense reports — these are the
// keyword destinations pfSense itself accepts, confirmed against a live
// error response. Deliberately NOT the interface name: pfSense resolves a
// bare interface name to "<interface> subnets" (its whole attached
// network), which is too broad for a NAT destination — use that
// interface's actual address (offered below once fetched) instead.
const STATIC_DESTINATION_OPTIONS = [{ value: 'any', label: 'any' }];

interface AllocationStatus {
  server_uuid: string;
  server_name: string;
  allocation_uuid: string;
  ip: string;
  port: number;
  server_running: boolean;
  rule_exists: boolean;
  rule_disabled: boolean | null;
  in_sync: boolean;
  conflicting_rule: string | null;
}

export default function PfSenseConfigPage() {
  const { addToast } = useToast();

  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const [enabled, setEnabled] = useState(false);
  const [baseUrl, setBaseUrl] = useState('');
  const [apiKeySet, setApiKeySet] = useState(false);
  const [apiKeyInput, setApiKeyInput] = useState('');
  const [insecureTls, setInsecureTls] = useState(false);
  const [iface, setIface] = useState('wan');
  const [protocol, setProtocol] = useState('tcp/udp');
  const [source, setSource] = useState('any');
  const [destination, setDestination] = useState('wan:ip');
  const [associatedRule, setAssociatedRule] = useState('pass');
  const [intervalSecs, setIntervalSecs] = useState(15);
  const [debug, setDebug] = useState(false);

  const [interfaces, setInterfaces] = useState<InterfaceOption[]>([]);
  const [virtualIps, setVirtualIps] = useState<VirtualIpOption[]>([]);
  const [interfacesLoading, setInterfacesLoading] = useState(false);
  const [interfacesError, setInterfacesError] = useState<string | null>(null);

  const [status, setStatus] = useState<AllocationStatus[]>([]);
  const [statusLoading, setStatusLoading] = useState(false);
  const [statusError, setStatusError] = useState<string | null>(null);

  const loadSettings = () => {
    setLoading(true);
    setLoadError(null);

    axiosInstance
      .get<SettingsResponse>(`${API_BASE}/settings`)
      .then(({ data }) => {
        setEnabled(data.enabled);
        setBaseUrl(data.base_url);
        setApiKeySet(data.api_key_set);
        setInsecureTls(data.insecure_tls);
        setIface(data.interface);
        setProtocol(data.protocol);
        setSource(data.source);
        setDestination(data.destination);
        setAssociatedRule(data.associated_rule);
        setIntervalSecs(data.interval_secs);
        setDebug(data.debug);
      })
      .catch((err) => setLoadError(httpErrorToHuman(err)))
      .finally(() => setLoading(false));
  };

  const loadInterfaces = () => {
    setInterfacesLoading(true);
    setInterfacesError(null);

    // Test against whatever's currently typed in the form, not whatever was
    // last saved — otherwise this would require saving an untested URL/key
    // first. Blank fields fall back to the saved/env value on the backend.
    axiosInstance
      .post<{ interfaces: InterfaceOption[]; virtual_ips: VirtualIpOption[] }>(`${API_BASE}/interfaces`, {
        base_url: baseUrl,
        api_key: apiKeyInput === '' ? null : apiKeyInput,
        insecure_tls: insecureTls,
        debug,
      })
      .then(({ data }) => {
        setInterfaces(data.interfaces);
        setVirtualIps(data.virtual_ips);
      })
      .catch((err) => setInterfacesError(httpErrorToHuman(err)))
      .finally(() => setInterfacesLoading(false));
  };

  const loadStatus = () => {
    setStatusLoading(true);
    setStatusError(null);

    axiosInstance
      .get<{ allocations: AllocationStatus[] }>(`${API_BASE}/status`, { params: { debug } })
      .then(({ data }) => setStatus(data.allocations))
      .catch((err) => setStatusError(httpErrorToHuman(err)))
      .finally(() => setStatusLoading(false));
  };

  useEffect(() => {
    loadSettings();
    loadStatus();
  }, []);

  const save = () => {
    setSaving(true);

    axiosInstance
      .put(`${API_BASE}/settings`, {
        enabled,
        base_url: baseUrl,
        api_key: apiKeyInput === '' ? null : apiKeyInput,
        insecure_tls: insecureTls,
        interface: iface,
        protocol,
        source,
        destination,
        associated_rule: associatedRule,
        interval_secs: intervalSecs,
        debug,
      })
      .then(() => {
        addToast('pfSense settings saved.', 'success');
        setApiKeyInput('');
        loadSettings();
        loadStatus();
      })
      .catch((err) => addToast(httpErrorToHuman(err), 'error'))
      .finally(() => setSaving(false));
  };

  // Always include the currently-selected interface so the dropdown has
  // something to show even before "Fetch from pfSense" has been used.
  const interfaceSelectData = interfaces.length
    ? interfaces.map((i) => ({ value: i.id, label: `${i.description} (${i.id})` }))
    : iface
      ? [{ value: iface, label: iface }]
      : [];

  // One "<name> address" / "<name> subnets" pair per fetched interface —
  // mirroring pfSense's own destination dropdown exactly (its GUI shows the
  // same split per interface). "address" uses the `:ip` modifier (resolves
  // dynamically to that interface's current IP, survives a DHCP change);
  // "subnets" is the bare interface id (its whole attached network — rarely
  // what you want for a single-server NAT destination, but offered for
  // parity with pfSense's own picker). This is a `Select`, not an
  // `Autocomplete`: Mantine's Autocomplete always inserts an option's
  // *label* on selection (confirmed in its source — `handleValueChange`
  // reads `optionsLockup[val].label`, unconditionally), so a friendly label
  // distinct from the real value silently saves the label text as your
  // destination, which pfSense then rejects. `Select` correctly keeps value
  // and label separate. Before you've fetched anything, this falls back to
  // just the currently-selected interface's own address/subnets.
  const interfaceDestinationOptions = (
    interfaces.length ? interfaces : iface ? [{ id: iface, description: iface, address: null }] : []
  ).flatMap((i) => [
    { value: `${i.id}:ip`, label: `${i.description} address` },
    { value: i.id, label: `${i.description} subnets` },
  ]);
  const destinationSelectData = [
    ...interfaceDestinationOptions,
    ...STATIC_DESTINATION_OPTIONS,
    ...virtualIps.map((vip) => ({
      value: vip.address,
      label: `${vip.description} — ${vip.address} (${vip.interface}, ${vip.mode})`,
    })),
    ...(destination && !interfaceDestinationOptions.some((o) => o.value === destination) &&
    !STATIC_DESTINATION_OPTIONS.some((o) => o.value === destination) &&
    !virtualIps.some((vip) => vip.address === destination)
      ? [{ value: destination, label: destination }]
      : []),
  ];

  if (loading) {
    return <Text>Loading…</Text>;
  }

  return (
    <Stack gap='xl' p='md'>
      <Stack gap='md'>
        <Title order={3}>pfSense Firewall Sync</Title>
        {loadError && <Alert color='red'>{loadError}</Alert>}

        <Switch
          label='Enabled'
          description='Turns the reconcile loop on or off. Takes effect within one reconcile interval — no panel restart needed.'
          checked={enabled}
          onChange={(e) => setEnabled(e.currentTarget.checked)}
        />

        <TextInput
          label='pfSense base URL'
          placeholder='https://192.168.1.1'
          value={baseUrl}
          onChange={(e) => setBaseUrl(e.currentTarget.value)}
        />

        <PasswordInput
          label='pfSense API key'
          placeholder={apiKeySet ? 'Key is set — leave blank to keep it' : 'No key set yet'}
          value={apiKeyInput}
          onChange={(e) => setApiKeyInput(e.currentTarget.value)}
        />

        <Switch
          label='Accept self-signed / invalid TLS certificates'
          checked={insecureTls}
          onChange={(e) => setInsecureTls(e.currentTarget.checked)}
        />

        <Group align='flex-end'>
          <Select
            label='WAN interface'
            data={interfaceSelectData}
            value={iface}
            onChange={(value) => setIface(value ?? iface)}
            searchable
            style={{ flex: 1 }}
          />
          <Button variant='default' onClick={loadInterfaces} loading={interfacesLoading}>
            Fetch from pfSense
          </Button>
        </Group>
        {interfacesError && <Alert color='red'>{interfacesError}</Alert>}

        <TextInput label='Protocol' value={protocol} onChange={(e) => setProtocol(e.currentTarget.value)} />
        <TextInput label='Source' value={source} onChange={(e) => setSource(e.currentTarget.value)} />

        <Select
          label='Destination'
          description='Same split pfSense itself shows per interface: "address" is that interface’s own current IP (the default, resolved dynamically), "subnets" is its whole attached network. Click "Fetch from pfSense" above to list every interface and any extra WAN IPs.'
          data={destinationSelectData}
          value={destination}
          onChange={(value) => setDestination(value ?? destination)}
          searchable
        />
        <TextInput
          label='Or enter a custom destination'
          description='For a pfSense alias (e.g. a custom firewall alias name) or any other value not listed above. Overrides the dropdown once you type here.'
          value={destination}
          onChange={(e) => setDestination(e.currentTarget.value)}
        />

        <TextInput
          label='Associated rule mode'
          description='"pass" auto-allows traffic, "new" creates a linked firewall rule, empty means you add the rule yourself.'
          value={associatedRule}
          onChange={(e) => setAssociatedRule(e.currentTarget.value)}
        />
        <NumberInput
          label='Reconcile interval (seconds)'
          min={10}
          value={intervalSecs}
          onChange={(value) => setIntervalSecs(typeof value === 'number' ? value : 15)}
        />

        <Switch
          label='Debug mode'
          description='Show the real pfSense/connection error on "Fetch from pfSense" and the status table below, instead of a generic message. Turn off once you’re done troubleshooting.'
          checked={debug}
          onChange={(e) => setDebug(e.currentTarget.checked)}
        />

        <Group>
          <Button onClick={save} loading={saving}>
            Save
          </Button>
        </Group>
      </Stack>

      <Stack gap='md'>
        <Group justify='space-between'>
          <Title order={4}>Managed allocations</Title>
          <Button variant='default' size='xs' onClick={loadStatus} loading={statusLoading}>
            Refresh
          </Button>
        </Group>
        {statusError && <Alert color='red'>{statusError}</Alert>}
        {status.length === 0 ? (
          <Text c='dimmed'>No in-use allocations found.</Text>
        ) : (
          <Table striped withTableBorder>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Server</Table.Th>
                <Table.Th>Allocation</Table.Th>
                <Table.Th>Running</Table.Th>
                <Table.Th>Rule</Table.Th>
                <Table.Th>In sync</Table.Th>
                <Table.Th>Conflict</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {status.map((row) => (
                <Table.Tr key={row.allocation_uuid}>
                  <Table.Td>{row.server_name}</Table.Td>
                  <Table.Td>{`${row.ip}:${row.port}`}</Table.Td>
                  <Table.Td>
                    <Badge color={row.server_running ? 'green' : 'gray'}>
                      {row.server_running ? 'Running' : 'Off'}
                    </Badge>
                  </Table.Td>
                  <Table.Td>
                    {!row.rule_exists ? (
                      <Badge color='gray'>None</Badge>
                    ) : row.rule_disabled ? (
                      <Badge color='yellow'>Disabled</Badge>
                    ) : (
                      <Badge color='green'>Enabled</Badge>
                    )}
                  </Table.Td>
                  <Table.Td>
                    <Badge color={row.in_sync ? 'green' : 'yellow'}>{row.in_sync ? 'Yes' : 'Pending'}</Badge>
                  </Table.Td>
                  <Table.Td>
                    {row.conflicting_rule ? (
                      <span title={`An existing unmanaged rule already uses this port: "${row.conflicting_rule}". This allocation's forward will not be created/updated until that's resolved.`}>
                        <Badge color='red'>Port in use</Badge>
                      </span>
                    ) : (
                      <Text c='dimmed' size='sm'>
                        —
                      </Text>
                    )}
                  </Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Stack>
    </Stack>
  );
}
