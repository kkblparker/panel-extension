import { Extension, ExtensionContext } from 'shared';
import PfSenseConfigPage from './ConfigPage.tsx';

/**
 * pfSense Firewall Sync
 *
 * The heavy lifting lives in the backend crate (a reconciling background task
 * that mirrors server allocations to pfSense NAT port-forwards, plus an
 * immediate pass on server install completion). This file wires up the admin
 * settings page at /admin/extensions/dev.kkblparker.pfsensesync, which reads
 * and writes settings via /api/admin/pfsensesync/* and lets an operator
 * configure everything that was previously environment-variable only.
 */
class PfSenseFirewallSyncExtension extends Extension {
  public packageName = 'dev.kkblparker.pfsensesync';
  public cardConfigurationPage = PfSenseConfigPage;

  public initialize(_ctx: ExtensionContext): void {}
}

export default new PfSenseFirewallSyncExtension();
