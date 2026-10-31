//! Player Count — a Calagopus backend extension.
//!
//! Shows live online/max player counts for game servers, on both the
//! individual server's Overview page and a dashboard summary panel. Two
//! query protocols, picked per-egg:
//! - Minecraft's own Server List Ping protocol (`craftping`).
//! - The Source-engine A2S query protocol (`a2s`), for Arma Reforger,
//!   Arma 3, and Project Zomboid.
//!
//! An egg opts in by declaring a hidden (`user_viewable: false`) variable,
//! `SERVER_QUERY_PROTOCOL`, set to `"minecraft"` or `"source"` - the same
//! egg-author-set-hidden-variable trick the Minecraft/Steam Workshop
//! extensions already use for their own per-egg configuration. A dedicated
//! variable (rather than piggybacking on an existing Workshop feature flag)
//! keeps this orthogonal: an admin can disable Workshop without losing
//! player counts, or vice versa.
//!
//! A background task polls every server that declares the variable on a
//! fixed interval and caches the result (`shared::cache::Cache`, TTL-based -
//! a live player count has no reason to be persisted once stale). See
//! [`poll`] for the enumeration query and [`query`] for the protocol
//! implementations. `routes::status` (per-server) and `routes::all` (dashboard
//! summary) just read whatever's currently cached.

mod poll;
mod query;
mod routes;

use shared::{
    State,
    extensions::{Extension, ExtensionRouteBuilder, background_tasks::BackgroundTaskBuilder},
};

#[derive(Default)]
pub struct ExtensionStruct;

#[async_trait::async_trait]
impl Extension for ExtensionStruct {
    async fn initialize_router(
        &mut self,
        state: State,
        builder: ExtensionRouteBuilder,
    ) -> ExtensionRouteBuilder {
        builder
            .add_client_server_api_router(|router| router.merge(routes::server_router(&state)))
            .add_client_api_router(|router| router.merge(routes::client_router(&state)))
    }

    async fn initialize_background_tasks(
        &mut self,
        _state: State,
        builder: BackgroundTaskBuilder,
    ) -> BackgroundTaskBuilder {
        builder
            .add_task(poll::POLL_TASK, move |state| async move {
                if let Err(err) = poll::poll_once(&state).await {
                    tracing::error!("player count: poll pass failed: {err:?}");
                }

                tokio::time::sleep(poll::interval()).await;

                Ok(())
            })
            .await;

        builder
    }
}
