import { lazy } from 'react';
import { Extension, ExtensionContext } from 'shared';

const PlayerCountStatCard = lazy(() => import('./PlayerCountStatCard.tsx'));
const DashboardPlayerCounts = lazy(() => import('./DashboardPlayerCounts.tsx'));
const PlayerListBlock = lazy(() => import('./PlayerListBlock.tsx'));

/**
 * Player Count
 *
 * The heavy lifting (Minecraft Server List Ping / Source A2S queries, on a
 * background poll) lives in the backend crate. This file just wires the
 * result into two existing, stock-compatible extension slots:
 * - A new stat tile on the server's own Overview page
 *   (`pages.server.console.addStatCard`), shown between CPU Load and Memory
 *   Load, for servers whose egg declares `SERVER_QUERY_PROTOCOL`.
 * - A bigger block alongside the CPU/Memory/Network charts
 *   (`pages.server.console.addStatBlock`) listing currently connected
 *   player names, when the query protocol provides them (Source's separate
 *   A2S_PLAYER query, or Minecraft's SLP `sample` list).
 * - A summary panel above the dashboard's server list
 *   (`pages.dashboard.home.container{All,Grouped}.prependContentComponent`).
 *   There's no per-card injection point in core for a badge on each server
 *   card, so this is a single panel listing every server's live count
 *   instead - see the extension's design notes for why.
 */
class PlayerCountExtension extends Extension {
  public packageName = 'dev.kkblparker.playercount';

  public initialize(ctx: ExtensionContext): void {
    ctx.extensionRegistry.pages.server.console.addStatCard(PlayerCountStatCard);
    ctx.extensionRegistry.pages.server.console.addStatBlock(PlayerListBlock);

    ctx.extensionRegistry.pages.dashboard.home.containerAll.prependContentComponent(DashboardPlayerCounts);
    ctx.extensionRegistry.pages.dashboard.home.containerGrouped.prependContentComponent(DashboardPlayerCounts);
  }
}

export default new PlayerCountExtension();
