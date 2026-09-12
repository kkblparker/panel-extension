//! Minecraft Workshop — a Calagopus backend extension.
//!
//! Adds `/api/client/servers/{server}/workshop/*` routes that let a server
//! built from an egg with the `minecraft_workshop` feature flag browse
//! Modrinth (covers both plugins and mods under one API) and install/
//! uninstall content by pulling the file directly onto the node via Wings'
//! remote file-pull, and tracking what's installed in a small manifest file
//! in the plugins/mods directory (jar files don't self-report their
//! Modrinth project id the way Reforger's config.json mods array did).
//!
//! Uses a DISTINCT permission group (`minecraft_workshop`) from the Arma
//! Reforger Workshop extension's `workshop` group - reusing the same group
//! name across extensions would silently clobber one extension's
//! permissions with the other's in the global permission registry.

mod config;
mod modrinth;
mod routes;
mod variables;

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
            "minecraft_workshop",
            PermissionGroup {
                description: "Permissions that control the ability to browse and manage this server's Minecraft plugins/mods.",
                permissions: IndexMap::from([
                    (
                        "read",
                        "Allows browsing the workshop and viewing installed plugins/mods.",
                    ),
                    (
                        "manage",
                        "Allows installing and removing workshop plugins/mods.",
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
