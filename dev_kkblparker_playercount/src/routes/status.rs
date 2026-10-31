use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use serde::Serialize;
    use shared::{
        ApiError,
        models::{server::GetServer, server_variable::ServerVariable},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        /// `false` if this server's egg doesn't declare `SERVER_QUERY_PROTOCOL` -
        /// the frontend stat card renders nothing in that case, rather than a
        /// confusing permanently-offline tile.
        supported: bool,
        online: bool,
        players: Option<u32>,
        max_players: Option<u32>,
        map: Option<String>,
        player_names: Option<Vec<String>>,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
    ))]
    pub async fn route(state: shared::GetState, server: GetServer) -> ApiResponseResult {
        let variables =
            ServerVariable::all_by_server_uuid_egg_uuid(&state.database, server.uuid, server.egg.uuid)
                .await
                .map_err(ApiResponse::from)?;

        let supported = variables.iter().any(|v| v.variable.env_variable == "SERVER_QUERY_PROTOCOL");
        if !supported {
            return ApiResponse::new_serialized(Response {
                supported: false,
                online: false,
                players: None,
                max_players: None,
                map: None,
                player_names: None,
            })
            .ok();
        }

        let status = crate::poll::get_cached(&state, server.uuid).await;

        ApiResponse::new_serialized(Response {
            supported: true,
            online: status.as_ref().is_some_and(|s| s.online),
            players: status.as_ref().and_then(|s| s.players),
            max_players: status.as_ref().and_then(|s| s.max_players),
            map: status.as_ref().and_then(|s| s.map.clone()),
            player_names: status.as_ref().and_then(|s| s.player_names.clone()),
        })
        .ok()
    }
}

pub fn router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .with_state(state.clone())
}
