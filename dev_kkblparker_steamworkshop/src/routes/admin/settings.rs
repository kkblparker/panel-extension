use shared::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use serde::Serialize;
    use shared::{
        GetState,
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    use crate::config::Config;

    #[derive(ToSchema, Serialize)]
    pub struct Response {
        /// Never the key itself - just whether one is configured.
        pub api_key_set: bool,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
    ))]
    pub async fn route(state: GetState, permissions: GetPermissionManager) -> ApiResponseResult {
        permissions.has_admin_permission("extensions.manage")?;

        let config = Config::load(&state).await?;

        ApiResponse::new_serialized(Response {
            api_key_set: !config.api_key.is_empty(),
        })
        .ok()
    }
}

mod put {
    use serde::{Deserialize, Serialize};
    use shared::{
        GetState,
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    use crate::settings::{PACKAGE_NAME, SteamWorkshopSettings};

    #[derive(ToSchema, Serialize)]
    struct Response {}

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        /// Omit (or send `null`) to keep the currently stored key. Send an
        /// empty string to clear it and fall back to `STEAM_API_KEY`.
        pub api_key: Option<String>,
    }

    #[utoipa::path(put, path = "/", responses(
        (status = OK, body = inline(Response)),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        permissions.has_admin_permission("extensions.manage")?;

        let mut app_settings = state.settings.get_mut().await?;
        let settings =
            app_settings.get_mut_extension_settings::<SteamWorkshopSettings>(PACKAGE_NAME)?;

        if let Some(api_key) = data.api_key {
            settings.api_key = if api_key.is_empty() {
                None
            } else {
                Some(api_key)
            };
        }

        app_settings.save().await?;

        activity_logger
            .log("steamworkshop:settings-update", serde_json::json!({}))
            .await;

        ApiResponse::new_serialized(Response {}).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .routes(routes!(put::route))
        .with_state(state.clone())
}
