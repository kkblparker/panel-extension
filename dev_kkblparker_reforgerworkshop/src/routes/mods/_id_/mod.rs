use utoipa_axum::{router::OpenApiRouter, routes};

mod install;

mod get {
    use axum::extract::Path;
    use serde::Serialize;
    use shared::{
        ApiError,
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        #[schema(value_type = Object)]
        result: serde_json::Value,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
        (
            "id" = String,
            description = "The workshop mod ID",
            example = "5965550F24A0C152",
        ),
    ))]
    pub async fn route(
        permissions: GetPermissionManager,
        server: GetServer,
        Path((_server, id)): Path<(String, String)>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("workshop.read")?;
        crate::routes::ensure_reforger_workshop(&server)?;

        let result = crate::reforger_workshop::get_mod(&id).await?;

        ApiResponse::new_serialized(Response { result }).ok()
    }
}

mod delete {
    use axum::extract::Path;
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::{GetPermissionManager, GetUser},
        },
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {}

    #[utoipa::path(delete, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
    ), params(
        (
            "server" = uuid::Uuid,
            description = "The server ID",
            example = "123e4567-e89b-12d3-a456-426614174000",
        ),
        (
            "id" = String,
            description = "The workshop mod ID",
            example = "5965550F24A0C152",
        ),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        user: GetUser,
        mut server: GetServer,
        activity_logger: GetServerActivityLogger,
        Path((_server, id)): Path<(String, String)>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("workshop.manage")?;
        crate::routes::ensure_reforger_workshop(&server)?;

        let mut config = crate::config::read_config(&state, &mut server).await?;
        let mut mods = crate::config::mods_array(&config);
        mods.retain(|entry| entry.get("modId").and_then(|v| v.as_str()) != Some(id.as_str()));

        config["mods"] = serde_json::Value::Array(mods);
        crate::config::write_config(&state, &mut server, user.uuid, &config).await?;

        activity_logger
            .log(
                "server:workshop.uninstall",
                serde_json::json!({ "mod_id": id }),
            )
            .await;

        ApiResponse::new_serialized(Response {}).ok()
    }
}

pub fn router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .routes(routes!(delete::route))
        .nest("/install", install::router(state))
        .with_state(state.clone())
}
