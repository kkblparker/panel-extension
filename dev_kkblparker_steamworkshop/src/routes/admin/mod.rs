//! Admin API routes for the Steam Workshop extension's settings page, mounted
//! at `/api/admin/steamworkshop/*`. Gates on the existing `extensions.manage`
//! admin permission rather than a dedicated permission group - this is just
//! another flavor of "manage an extension", same as pfSense Firewall Sync's
//! settings routes.

mod settings;

use shared::State;
use utoipa_axum::router::OpenApiRouter;

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .nest("/steamworkshop/settings", settings::router(state))
        .with_state(state.clone())
}
