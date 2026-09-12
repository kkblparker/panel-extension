import { faPuzzlePiece } from '@fortawesome/free-solid-svg-icons';
import { lazy } from 'react';
import { Extension, ExtensionContext } from 'shared';

const ServerWorkshop = lazy(() => import('./ServerWorkshop.tsx'));

/**
 * Minecraft Workshop
 *
 * The heavy lifting lives in the backend crate (proxying Modrinth and
 * pulling/deleting plugin or mod jars directly on the node via Wings). This
 * file just registers the "Workshop" server tab, shown only for servers
 * built from an egg with the `minecraft_workshop` feature flag.
 */
class MinecraftWorkshopExtension extends Extension {
  public packageName = 'dev.kkblparker.minecraftworkshop';

  public initialize(ctx: ExtensionContext): void {
    ctx.extensionRegistry.routes.addServerRoute({
      name: () => 'Workshop',
      icon: faPuzzlePiece,
      path: '/workshop',
      element: ServerWorkshop,
      permission: 'minecraft_workshop.read',
      requiresEggFeature: 'minecraft_workshop',
    });
  }
}

export default new MinecraftWorkshopExtension();
