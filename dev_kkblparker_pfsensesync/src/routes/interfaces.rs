use shared::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod post {
    use axum::http::StatusCode;
    use serde::{Deserialize, Serialize};
    use shared::{
        GetState,
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    use crate::{
        config::Config,
        pfsense::{InterfaceEntry, PfSenseClient, VirtualIpEntry},
    };

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        /// Connection details to test/fetch with, straight from the (possibly
        /// unsaved) settings form. Any field left blank falls back to the
        /// currently saved/env value, so testing a new URL doesn't require
        /// retyping an already-saved API key.
        pub base_url: Option<String>,
        pub api_key: Option<String>,
        pub insecure_tls: Option<bool>,
        /// Show the real error on failure, even if not saved yet.
        pub debug: Option<bool>,
    }

    #[derive(ToSchema, Serialize)]
    pub struct Response {
        interfaces: Vec<InterfaceEntry>,
        /// Additional WAN IPs (CARP/IP-alias/proxy-ARP) — populated only for
        /// setups with more than one public IP; empty otherwise.
        virtual_ips: Vec<VirtualIpEntry>,
    }

    #[utoipa::path(post, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, description = "pfSense URL/API key are not configured yet"),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        permissions.has_admin_permission("extensions.manage")?;

        let mut config = Config::load(&state).await?;

        if let Some(base_url) = non_empty(data.base_url.as_deref()) {
            config.base_url = base_url.trim_end_matches('/').to_string();
        }
        if let Some(api_key) = non_empty(data.api_key.as_deref()) {
            config.api_key = api_key.to_string();
        }
        if let Some(insecure_tls) = data.insecure_tls {
            config.insecure_tls = insecure_tls;
        }
        if let Some(debug) = data.debug {
            config.debug = debug;
        }

        if config.base_url.is_empty() || config.api_key.is_empty() {
            return ApiResponse::error("set the pfSense URL and API key first")
                .with_status(StatusCode::BAD_REQUEST)
                .ok();
        }

        let client = PfSenseClient::new(&config)?;
        let (interfaces, virtual_ips) =
            match tokio::try_join!(client.list_interfaces(), client.list_virtual_ips()) {
                Ok(result) => result,
                Err(err) => {
                    return crate::routes::pfsense_call_error(
                        &config,
                        "failed to fetch interfaces/virtual IPs from pfSense",
                        err,
                    )
                    .ok();
                }
            };

        ApiResponse::new_serialized(Response {
            interfaces,
            virtual_ips,
        })
        .ok()
    }

    fn non_empty(v: Option<&str>) -> Option<&str> {
        v.map(str::trim).filter(|v| !v.is_empty())
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(post::route))
        .with_state(state.clone())
}
