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
        /// The effective settings currently in use — from the admin settings
        /// page where saved, otherwise from `PFSENSE_*` environment variables.
        pub enabled: bool,
        pub base_url: String,
        /// Never the key itself — just whether one is configured.
        pub api_key_set: bool,
        pub insecure_tls: bool,
        pub interface: String,
        pub protocol: String,
        pub source: String,
        pub destination: String,
        pub associated_rule: String,
        pub interval_secs: u64,
        /// When true, admin API errors from pfSense calls include the real
        /// error text instead of a generic message.
        pub debug: bool,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
    ))]
    pub async fn route(state: GetState, permissions: GetPermissionManager) -> ApiResponseResult {
        permissions.has_admin_permission("extensions.manage")?;

        let config = Config::load(&state).await?;

        ApiResponse::new_serialized(Response {
            enabled: config.enabled,
            base_url: config.base_url,
            api_key_set: !config.api_key.is_empty(),
            insecure_tls: config.insecure_tls,
            interface: config.interface,
            protocol: config.protocol,
            source: config.source,
            destination: config.destination,
            associated_rule: config.associated_rule,
            interval_secs: config.interval.as_secs(),
            debug: config.debug,
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

    use crate::settings::{PACKAGE_NAME, PfSenseSettings};

    #[derive(ToSchema, Serialize)]
    struct Response {}

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        pub enabled: bool,
        pub base_url: String,
        /// Omit (or send `null`) to keep the currently stored key. Send an
        /// empty string to clear it and fall back to `PFSENSE_API_KEY`.
        pub api_key: Option<String>,
        pub insecure_tls: bool,
        pub interface: String,
        pub protocol: String,
        pub source: String,
        pub destination: String,
        pub associated_rule: String,
        pub interval_secs: u64,
        pub debug: bool,
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
        let settings = app_settings.get_mut_extension_settings::<PfSenseSettings>(PACKAGE_NAME)?;

        settings.enabled = Some(data.enabled);
        settings.base_url = Some(data.base_url);
        if let Some(api_key) = data.api_key {
            settings.api_key = if api_key.is_empty() {
                None
            } else {
                Some(api_key)
            };
        }
        settings.insecure_tls = Some(data.insecure_tls);
        settings.interface = Some(data.interface);
        settings.protocol = Some(data.protocol);
        settings.source = Some(data.source);
        settings.destination = Some(data.destination);
        settings.associated_rule = Some(data.associated_rule);
        settings.interval_secs = Some(data.interval_secs);
        settings.debug = Some(data.debug);

        app_settings.save().await?;

        activity_logger
            .log("pfsensesync:settings-update", serde_json::json!({}))
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
