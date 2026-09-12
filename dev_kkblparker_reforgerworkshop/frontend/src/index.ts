import { faPuzzlePiece } from '@fortawesome/free-solid-svg-icons';
import { lazy } from 'react';
import { Extension, ExtensionContext } from 'shared';

const ServerWorkshop = lazy(() => import('./ServerWorkshop.tsx'));

/**
 * Arma Reforger Workshop
 *
 * The heavy lifting lives in the backend crate (proxying reforgermods.net and
 * patching a server's config.json mods array). This file just registers the
 * "Workshop" server tab, shown only for servers built from an egg with the
 * `reforger_workshop` feature flag.
 */
class ReforgerWorkshopExtension extends Extension {
  public packageName = 'dev.kkblparker.reforgerworkshop';

  public initialize(ctx: ExtensionContext): void {
    ctx.extensionRegistry.routes.addServerRoute({
      name: () => 'Workshop',
      icon: faPuzzlePiece,
      path: '/workshop',
      element: ServerWorkshop,
      permission: 'workshop.read',
      requiresEggFeature: 'reforger_workshop',
    });
  }
}

export default new ReforgerWorkshopExtension();
