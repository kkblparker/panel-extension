use utoipa_axum::router::OpenApiRouter;

mod installed;
mod mods;

pub fn router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .nest("/workshop/mods", mods::router(state))
        .nest("/workshop/installed", installed::router(state))
        .with_state(state.clone())
}
