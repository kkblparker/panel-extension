use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use serde::Serialize;
    use shared::{
        ApiError,
        models::{server::Server, user::GetUser},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    /// Effectively "all of them" - this panel is not expected to host tens of
    /// thousands of servers per user, and this route is polled on a plain
    /// interval by the dashboard summary panel, not paginated by the caller.
    const MAX_SERVERS: i64 = 10_000;

    #[derive(ToSchema, Serialize)]
    struct ServerPlayerCount {
        #[schema(value_type = String)]
        server_uuid: uuid::Uuid,
        server_name: String,
        online: bool,
        players: Option<u32>,
        max_players: Option<u32>,
        map: Option<String>,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        servers: Vec<ServerPlayerCount>,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
    ))]
    pub async fn route(state: shared::GetState, user: GetUser) -> ApiResponseResult {
        let servers = Server::by_user_uuid_with_pagination(&state.database, user.uuid, 1, MAX_SERVERS, None)
            .await
            .map_err(ApiResponse::from)?;

        let mut out = Vec::new();
        for server in servers.data {
            let Some(status) = crate::poll::get_cached(&state, server.uuid).await else {
                // Never queried - either the egg has no SERVER_QUERY_PROTOCOL
                // variable, or the first poll tick hasn't happened yet.
                // Either way, nothing useful to show for this server.
                continue;
            };

            out.push(ServerPlayerCount {
                server_uuid: server.uuid,
                server_name: server.name.to_string(),
                online: status.online,
                players: status.players,
                max_players: status.max_players,
                map: status.map,
            });
        }

        ApiResponse::new_serialized(Response { servers: out }).ok()
    }
}

pub fn router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .with_state(state.clone())
}
