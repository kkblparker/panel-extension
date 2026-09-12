use axum::http::StatusCode;
use shared::response::ApiResponse;
use utoipa_axum::router::OpenApiRouter;

mod installed;
mod mods;

const FEATURE: &str = "reforger_workshop";

pub(crate) fn ensure_reforger_workshop(
    server: &shared::models::server::Server,
) -> Result<(), ApiResponse> {
    if server.egg.features.iter().any(|feature| feature == FEATURE) {
        Ok(())
    } else {
        Err(
            ApiResponse::error("workshop mods are not supported for this server's egg")
                .with_status(StatusCode::BAD_REQUEST),
        )
    }
}

pub fn router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .nest("/workshop/mods", mods::router(state))
        .nest("/workshop/installed", installed::router(state))
        .with_state(state.clone())
}
