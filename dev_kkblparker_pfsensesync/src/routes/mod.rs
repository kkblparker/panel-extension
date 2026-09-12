//! Admin API routes for the pfSense Firewall Sync extension's settings page,
//! mounted at `/api/admin/pfsensesync/*` (see [`crate::Extension::initialize_router`]).
//!
//! All routes here gate on the existing `extensions.manage` admin permission
//! rather than registering a dedicated permission group — this extension's
//! settings are just another flavor of "manage an extension".

mod interfaces;
mod settings;
mod status;

use axum::http::StatusCode;
use shared::{State, response::ApiResponse};

use crate::config::Config;

use utoipa_axum::router::OpenApiRouter;

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .nest("/pfsensesync/settings", settings::router(state))
        .nest("/pfsensesync/interfaces", interfaces::router(state))
        .nest("/pfsensesync/status", status::router(state))
        .with_state(state.clone())
}

/// Build an admin-facing error response for a failed pfSense call. Shows the
/// real error text when the extension's own `debug` setting is on, a generic
/// message otherwise — this is scoped to this extension precisely so seeing
/// what's actually wrong (a bad URL, an auth failure, a TLS error, ...)
/// doesn't require flipping the panel-wide `APP_DEBUG` flag and restarting
/// the whole panel.
pub(crate) fn pfsense_call_error(config: &Config, context: &str, err: anyhow::Error) -> ApiResponse {
    let message = if config.debug {
        format!("{context}: {err:?}")
    } else {
        format!(
            "{context} (enable \"debug\" in this extension's settings to see the detailed error)"
        )
    };

    ApiResponse::error(message).with_status(StatusCode::BAD_GATEWAY)
}
