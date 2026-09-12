//! Arma Reforger Workshop — a Calagopus backend extension.
//!
//! Adds `/api/client/servers/{server}/workshop/*` routes that let a server
//! built from an egg with the `reforger_workshop` feature flag browse
//! reforgermods.net (the only public Workshop API for Arma Reforger — Bohemia
//! doesn't expose one of their own) and install/uninstall mods by patching
//! that server's `config.json` `mods` array over the Wings files API.

mod config;
mod reforger_workshop;
mod routes;

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
