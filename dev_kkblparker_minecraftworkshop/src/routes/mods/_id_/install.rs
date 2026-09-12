use utoipa_axum::{router::OpenApiRouter, routes};

mod post {
    use axum::extract::Path;
    use serde::Serialize;
    use shared::{
        ApiError,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::{GetPermissionManager, GetUser},
        },
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {}

    #[utoipa::path(post, path = "/", responses(
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
        user: GetUser,
        mut server: GetServer,
        activity_logger: GetServerActivityLogger,
        Path((_server, id)): Path<(String, String)>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("minecraft_workshop.manage")?;
        let config = crate::variables::resolve(&state, &server).await?;

        let Some(file) =
            crate::modrinth::get_best_version_file(&id, &config.loader, &config.mc_version).await?
        else {
            return ApiResponse::error(
                "no compatible version found for this server's Minecraft version/loader",
            )
            .with_status(axum::http::StatusCode::EXPECTATION_FAILED)
            .ok();
        };

        // Look up the project's display name/icon for the manifest/installed list, best-effort -
        // if this fails we still install, just with a less friendly display name.
        let project = crate::modrinth::get_project(&id).await.ok();
        let name = project
            .as_ref()
            .and_then(|p| p.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();
        let icon_url = project
            .as_ref()
            .and_then(|p| p.get("icon_url"))
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());

        let pull_response = server
            .node
            .fetch_cached(&state.database)
            .await?
            .api_client(&state.database)
            .await?
            .post_servers_server_files_pull(
                server.uuid,
                &wings_api::servers_server_files_pull::post::RequestBody {
                    root: config.install_dir.into(),
                    url: file.url.clone().into(),
                    file_name: Some(file.filename.clone().into()),
                    use_header: false,
                    foreground: true,
                },
            )
            .await?;

        // `foreground: true` asks Wings to download inline rather than queue a background job, so
        // in practice this should always be `Ok` - but handle `Accepted` (queued) too rather than
        // assume, and don't record the manifest entry until the file is actually confirmed written.
        if matches!(
            pull_response,
            wings_api::servers_server_files_pull::post::Response::Accepted(_)
        ) {
            return ApiResponse::error("download queued, please try again shortly")
                .with_status(axum::http::StatusCode::ACCEPTED)
                .ok();
        }

        let mut manifest =
            crate::config::read_manifest(&state, &mut server, config.install_dir).await?;
        let mut installed = crate::config::installed_array(&manifest);
        installed
            .retain(|entry| entry.get("project_id").and_then(|v| v.as_str()) != Some(id.as_str()));
        installed.push(serde_json::json!({
            "project_id": id,
            "version_id": file.version_id,
            "file_name": file.filename,
            "name": name,
            "icon_url": icon_url,
        }));
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
                "server:minecraft-workshop.install",
                serde_json::json!({
                    "project_id": id,
                    "version_id": file.version_id,
                    "file_name": file.filename,
                }),
            )
            .await;

        ApiResponse::new_serialized(Response {}).ok()
    }
}

pub fn router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .routes(routes!(post::route))
        .with_state(state.clone())
}
