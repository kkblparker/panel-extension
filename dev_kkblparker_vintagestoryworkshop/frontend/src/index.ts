import { faCubes } from '@fortawesome/free-solid-svg-icons';
import { lazy } from 'react';
import { Extension, ExtensionContext } from 'shared';

const ServerWorkshop = lazy(() => import('./ServerWorkshop.tsx'));

/**
 * Vintage Story Workshop
 *
 * The heavy lifting lives in the backend crate (proxying mods.vintagestory.at and writing mod
 * zips directly into the server's data/Mods directory). This file just registers the "Vintage
 * Story Workshop" server tab.
 */
class VintageStoryWorkshopExtension extends Extension {
  public packageName = 'dev.kkblparker.vintagestoryworkshop';

  public initialize(ctx: ExtensionContext): void {
    ctx.extensionRegistry.routes.addServerRoute({
      name: () => 'Vintage Story Workshop',
      icon: faCubes,
      path: '/vintagestory-workshop',
      element: ServerWorkshop,
      permission: 'workshop.read',
      requiresEggFeature: 'vintagestory_workshop',
    });
  }
}

export default new VintageStoryWorkshopExtension();
