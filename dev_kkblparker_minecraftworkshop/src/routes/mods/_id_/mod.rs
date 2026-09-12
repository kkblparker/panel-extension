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
            description = "The Modrinth project ID or slug",
            example = "AANobbMI",
        ),
    ))]
    pub async fn route(
        state: shared::GetState,
        permissions: GetPermissionManager,
        server: GetServer,
        Path((_server, id)): Path<(String, String)>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("minecraft_workshop.read")?;
        crate::variables::resolve(&state, &server).await?;

        let result = crate::modrinth::get_project(&id).await?;

        ApiResponse::new_serialized(Response { result }).ok()
    }
}

mod delete {
    use axum::extract::Path;
    use serde::Serialize;
    use shared::{
        ApiError,
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
            "id" = String,
            description = "The Modrinth project ID or slug",
            example = "AANobbMI",
        ),
    ))]
    pub async fn route(
        state: shared::GetState,
        permissions: GetPermissionManager,
        user: shared::models::user::GetUser,
        mut server: GetServer,
        activity_logger: GetServerActivityLogger,
        Path((_server, id)): Path<(String, String)>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("minecraft_workshop.manage")?;
        let config = crate::variables::resolve(&state, &server).await?;

        let mut manifest =
            crate::config::read_manifest(&state, &mut server, config.install_dir).await?;
        let mut installed = crate::config::installed_array(&manifest);

        let Some(entry) = installed
            .iter()
            .find(|entry| entry.get("project_id").and_then(|v| v.as_str()) == Some(id.as_str()))
            .cloned()
        else {
            return ApiResponse::error("mod is not installed")
                .with_status(axum::http::StatusCode::NOT_FOUND)
                .ok();
        };

        let file_name = entry
            .get("file_name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        server
            .node
            .fetch_cached(&state.database)
            .await?
            .api_client(&state.database)
            .await?
            .post_servers_server_files_delete(
                server.uuid,
                &wings_api::servers_server_files_delete::post::RequestBody {
                    root: config.install_dir.into(),
                    files: vec![file_name.clone().into()],
                },
            )
            .await?;

        installed
            .retain(|entry| entry.get("project_id").and_then(|v| v.as_str()) != Some(id.as_str()));
        manifest["installed"] = serde_json::Value::Array(installed);
        crate::config::write_manifest(
            &state,
            &mut server,
            config.install_dir,
            user.uuid,
            &manifest,
        )
        .await?;

        activity_logger
            .log(
                "server:minecraft-workshop.uninstall",
                serde_json::json!({ "project_id": id, "file_name": file_name }),
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
