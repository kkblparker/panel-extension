import { faChevronDown, faDownload } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import Button from '@/elements/buttons/Button.tsx';
import Menu from '@/elements/overlays/Menu.tsx';
import { MOD_LIST_KIND_LABELS, ModListKind } from './schemas.ts';

interface Props {
  availableKinds: ModListKind[];
  size?: 'xs' | 'sm';
  disabled?: boolean;
  loading?: boolean;
  onAdd: (kind: ModListKind) => void;
}

// A plain "Add" button when an egg only has one mod list (the common case), or an "Add as..."
// dropdown when it splits mods by kind the way Arma 3 does (client/server-only/optional).
export default function AddModButton({ availableKinds, size = 'xs', disabled, loading, onAdd }: Props) {
  if (availableKinds.length <= 1) {
    return (
      <Button
        size={size}
        color='blue'
        leftSection={<FontAwesomeIcon icon={faDownload} />}
        disabled={disabled}
        loading={loading}
        onClick={(e) => {
          e.stopPropagation();
          onAdd(availableKinds[0] ?? 'client');
        }}
      >
        Add
      </Button>
    );
  }

  return (
    <Menu shadow='md' position='bottom-end'>
      <Menu.Target>
        <Button
          size={size}
          color='blue'
          leftSection={<FontAwesomeIcon icon={faDownload} />}
          rightSection={<FontAwesomeIcon icon={faChevronDown} size='xs' />}
          disabled={disabled}
          loading={loading}
          onClick={(e) => e.stopPropagation()}
        >
          Add as...
        </Button>
      </Menu.Target>
      <Menu.Dropdown onClick={(e) => e.stopPropagation()}>
        {availableKinds.map((kind) => (
          <Menu.Item key={kind} onClick={() => onAdd(kind)}>
            {MOD_LIST_KIND_LABELS[kind]}
          </Menu.Item>
        ))}
      </Menu.Dropdown>
    </Menu>
  );
}
