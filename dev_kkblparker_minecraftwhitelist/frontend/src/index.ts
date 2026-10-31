import { faUserShield } from '@fortawesome/free-solid-svg-icons';
import { lazy } from 'react';
import { Extension, ExtensionContext } from 'shared';

const ServerWhitelist = lazy(() => import('./ServerWhitelist.tsx'));

/**
 * Minecraft Whitelist & Bans
 *
 * The heavy lifting lives in the backend crate (resolving usernames via Mojang and editing
 * whitelist.json/banned-players.json, or sending the equivalent console command when the server
 * is running). This file just registers the "Whitelist & Bans" server tab.
 *
 * As with the Minecraft Workshop extension, the Egg Configuration route order (Admin > Egg
 * Configurations) is what actually controls which eggs see this tab on stock core today -
 * `requiresEggFeature` is set for when core picks up this fork's gating support.
 */
class MinecraftWhitelistExtension extends Extension {
  public packageName = 'dev.kkblparker.minecraftwhitelist';

  public initialize(ctx: ExtensionContext): void {
    ctx.extensionRegistry.routes.addServerRoute({
      name: () => 'Whitelist & Bans',
      icon: faUserShield,
      path: '/minecraft-whitelist',
      element: ServerWhitelist,
      permission: 'minecraft_whitelist.read',
      requiresEggFeature: 'minecraft_whitelist',
    });
  }
}

export default new MinecraftWhitelistExtension();
