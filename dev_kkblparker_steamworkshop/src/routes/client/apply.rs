use shared::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod post {
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::GetPermissionManager,
        },
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {}

    /// Restarts the server so its entrypoint script re-scans its Workshop mod list variable and
    /// downloads/updates anything new via steamcmd. Verified live against the actual
    /// `ghcr.io/parkervcp/games:arma3` image: its `entrypoint.sh` checks every `@<id>` token in
    /// the mods variable on EVERY startup (not just install) when the egg's "Automatic Updates"
    /// variable is enabled - so, unlike a from-scratch steamcmd game, applying a Workshop change
    /// here only needs a normal restart, not a destructive reinstall.
    ///
    /// Requires `control.restart` in addition to `steam_workshop.manage` - this issues the same
    /// power action the server's own Restart button does.
    #[utoipa::path(post, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = EXPECTATION_FAILED, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        mut server: GetServer,
        activity_logger: GetServerActivityLogger,
    ) -> ApiResponseResult {
        permissions.has_server_permission("steam_workshop.manage")?;
        permissions.has_server_permission("control.restart")?;
        let config = crate::variables::resolve(&state, &mut server).await?;

        tokio::spawn(async move {
            server
                .node
                .fetch_cached(&state.database)
                .await?
                .api_client(&state.database)
                .await?
                .post_servers_server_power(
                    server.uuid,
                    &wings_api::servers_server_power::post::RequestBody {
                        action: wings_api::ServerPowerAction::Restart,
                        wait_seconds: None,
                    },
                )
                .await?;

            activity_logger
                .log(
                    "server:steam-workshop.apply",
                    serde_json::json!({ "mod_ids": config.mod_ids() }),
                )
                .await;

            ApiResponse::new_serialized(Response {}).ok()
        })
        .await?
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(post::route))
        .with_state(state.clone())
}
