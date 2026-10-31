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
            description = "The Vintage Story mod ID",
            example = "12643",
        ),
    ))]
    pub async fn route(
        permissions: GetPermissionManager,
        server: GetServer,
        Path((_server, id)): Path<(String, String)>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("workshop.read")?;
        crate::routes::ensure_vintagestory_workshop(&server)?;

        let result = crate::vsmoddb::get_mod(&id).await?;

        ApiResponse::new_serialized(Response { result }).ok()
    }
}

pub fn router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .nest("/install", install::router(state))
        .with_state(state.clone())
}
