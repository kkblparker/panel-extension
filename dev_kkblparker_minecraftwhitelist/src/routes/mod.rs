use utoipa_axum::router::OpenApiRouter;

mod bans;
mod whitelist;

pub fn router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .nest("/minecraft-whitelist/whitelist", whitelist::router(state))
        .nest("/minecraft-whitelist/bans", bans::router(state))
        .with_state(state.clone())
}
