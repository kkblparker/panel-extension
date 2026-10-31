//! Vintage Story Workshop — a Calagopus backend extension.
//!
//! Adds `/api/client/servers/{server}/vintagestory-workshop/*` routes that let a server built
//! from an egg with the `vintagestory_workshop` feature flag browse mods.vintagestory.at (the
//! official Vintage Story Mod DB, which has a real public JSON API - no scraping needed, unlike
//! the Arma Reforger Workshop extension) and install/uninstall mods by writing/deleting their
//! `.zip` release file directly in the server's `data/Mods` directory over the Wings files API -
//! no config patching needed, Vintage Story's own mod loader reads `modinfo.json` straight out of
//! each zip sitting in that folder.

mod files;
mod routes;
mod vsmoddb;

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
            "workshop",
            PermissionGroup {
                description: "Permissions that control the ability to browse and manage this server's workshop mods.",
                permissions: IndexMap::from([
                    (
                        "read",
                        "Allows browsing the workshop and viewing installed mods.",
                    ),
                    ("manage", "Allows installing and removing workshop mods."),
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
