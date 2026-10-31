//! Minecraft Whitelist & Bans — a Calagopus backend extension.
//!
//! Adds `/api/client/servers/{server}/minecraft-whitelist/*` routes that let a server built from
//! an egg with the `minecraft_whitelist` feature flag manage its `whitelist.json` and
//! `banned-players.json` by username, resolving names to UUIDs via Mojang's API.
//!
//! Prefers sending the equivalent console command (`whitelist add`/`remove`, `ban`/`pardon`) when
//! the server is running, since that's the server's own code updating both the file and its
//! in-memory cache (the whitelist is cached at startup and only refreshed by commands like
//! `whitelist reload`/`whitelist add` - editing the file directly while running would silently do
//! nothing until a manual reload). Falls back to editing the file directly when the server is
//! offline, since there's nowhere to send a command to.
//!
//! Uses a DISTINCT permission group (`minecraft_whitelist`) from the Minecraft Workshop
//! extension's `minecraft_workshop` group.

mod entries;
mod files;
mod gate;
mod mojang;
mod routes;
mod runtime;

use indexmap::IndexMap;
use shared::{
    State,
    extensions::{Extension, ExtensionPermissionsBuilder, ExtensionRouteBuilder},
    permissions::PermissionGroup,
};

#[derive(Default)]
pub struct ExtensionStruct;

#[async_trait::async_trait]
impl Extension for ExtensionStruct {
    async fn initialize_permissions(
        &mut self,
        _state: State,
        builder: ExtensionPermissionsBuilder,
    ) -> ExtensionPermissionsBuilder {
        builder.add_server_permission_group(
            "minecraft_whitelist",
            PermissionGroup {
                description: "Permissions that control the ability to manage this server's Minecraft whitelist and ban list.",
                permissions: IndexMap::from([
                    (
                        "read",
                        "Allows viewing the whitelist and ban list.",
                    ),
                    (
                        "manage",
                        "Allows adding and removing whitelist and ban entries.",
                    ),
                ]),
            },
        )
    }

    async fn initialize_router(
        &mut self,
        state: State,
        builder: ExtensionRouteBuilder,
    ) -> ExtensionRouteBuilder {
        builder.add_client_server_api_router(|router| router.merge(routes::router(&state)))
    }
}
