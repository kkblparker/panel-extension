use shared::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    use crate::variables::ModListKind;

    #[derive(ToSchema, Serialize)]
    struct ModEntry {
        id: String,
        kinds: Vec<ModListKind>,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        #[schema(value_type = Object)]
        result: serde_json::Value,
        /// Which list(s) each mod ID is currently in - almost always one entry, more than one
        /// just means it was added under multiple kinds (e.g. both regular and server-only).
        mods: Vec<ModEntry>,
        /// Which `ModListKind`s this egg supports - a single entry means the frontend should just
        /// show a plain "Add" button when browsing, more than one means it should offer a picker.
        available_kinds: Vec<ModListKind>,
        /// Whether this egg has a separate "which mods to actually load" list distinct from the
        /// Workshop ID list (e.g. Project Zomboid's `Mods=`). When `false`, `load_order` is
        /// always `null` and the frontend shouldn't offer to edit it.
        load_order_supported: bool,
        load_order: Option<String>,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        mut server: GetServer,
    ) -> ApiResponseResult {
        permissions.has_server_permission("steam_workshop.read")?;
        let config = crate::variables::resolve(&state, &mut server).await?;
        let entries = config.mods();

        // Batch-resolve details for every ID in the list in one call - GetPublishedFileDetails is
        // keyless and accepts multiple IDs at once, so this needs no Steam Web API key even if
        // one isn't configured for searching.
        let result = crate::steam::get_item_details(
            &entries.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
        )
        .await?;

        ApiResponse::new_serialized(Response {
            result,
            mods: entries
                .into_iter()
                .map(|entry| ModEntry {
                    id: entry.id,
                    kinds: entry.kinds,
                })
                .collect(),
            available_kinds: config.available_kinds(),
            load_order_supported: config.load_order_supported(),
            load_order: config.load_order(),
        })
        .ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .with_state(state.clone())
}
