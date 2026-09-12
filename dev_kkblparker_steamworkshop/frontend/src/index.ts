import { faSteam } from '@fortawesome/free-brands-svg-icons';
import { lazy } from 'react';
import { Extension, ExtensionContext } from 'shared';

const ServerWorkshop = lazy(() => import('./ServerWorkshop.tsx'));
const SteamWorkshopConfigPage = lazy(() => import('./ConfigPage.tsx'));

/**
 * Steam Workshop
 *
 * Unlike the Arma Reforger and Minecraft Workshop extensions, this one is a
 * mod LIST manager, not an instant installer: Steam Workshop content has no
 * direct download URL (confirmed live against Steam's own API), so getting
 * it onto a server requires `steamcmd +workshop_download_item`, which can
 * only run during the server's own install/reinstall. Adding/removing a mod
 * here just edits the WORKSHOP_MOD_IDS variable; "Apply Changes" triggers
 * the actual reinstall that downloads it.
 *
 * Registers the "Workshop" server tab (only for eggs with the
 * `steam_workshop` feature flag) and the admin settings page for the Steam
 * Web API key used to search the Workshop.
 */
class SteamWorkshopExtension extends Extension {
  public packageName = 'dev.kkblparker.steamworkshop';
  public cardConfigurationPage = SteamWorkshopConfigPage;

  public initialize(ctx: ExtensionContext): void {
    ctx.extensionRegistry.routes.addServerRoute({
      name: () => 'Workshop',
      icon: faSteam,
      path: '/workshop',
      element: ServerWorkshop,
      permission: 'steam_workshop.read',
      requiresEggFeature: 'steam_workshop',
    });
  }
}

export default new SteamWorkshopExtension();
