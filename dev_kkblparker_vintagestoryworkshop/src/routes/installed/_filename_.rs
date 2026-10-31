use utoipa_axum::{router::OpenApiRouter, routes};

mod delete {
    use axum::extract::Path;
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
            "filename" = String,
            description = "The installed mod's file name",
            example = "thestonecoreadmincontrol_1.0.1.zip",
        ),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        mut server: GetServer,
        activity_logger: GetServerActivityLogger,
        Path((_server, filename)): Path<(String, String)>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("workshop.manage")?;
        crate::routes::ensure_vintagestory_workshop(&server)?;

        crate::files::delete_mod_file(&state, &mut server, &filename).await?;

        activity_logger
            .log(
                "server:workshop.uninstall",
                serde_json::json!({ "file_name": filename }),
            )
            .await;

        ApiResponse::new_serialized(Response {}).ok()
    }
}

pub fn router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .routes(routes!(delete::route))
        .with_state(state.clone())
}
