use utoipa_axum::router::OpenApiRouter;

mod all;
mod status;

/// Mounted under `/api/client/servers/{server}`.
pub fn server_router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .nest("/player-count", status::router(state))
        .with_state(state.clone())
}

/// Mounted under `/api/client` (not tied to a single server).
pub fn client_router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .nest("/player-counts", all::router(state))
        .with_state(state.clone())
}
