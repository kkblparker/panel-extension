use utoipa_axum::{router::OpenApiRouter, routes};

mod post {
    use axum::extract::Path;
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::{GetPermissionManager, GetUser},
        },
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        /// The exact file name of the release to install, e.g. "thestonecoreadmincontrol_1.0.1.zip"
        /// - taken straight from the mod detail response's `releases[].fileName`.
        #[garde(length(min = 1, max = 255))]
        #[schema(min_length = 1, max_length = 255)]
        file_name: compact_str::CompactString,
        /// The release's direct download URL, from `releases[].downloadUrl`.
        #[garde(length(min = 1, max = 2000))]
        #[schema(min_length = 1, max_length = 2000)]
        download_url: compact_str::CompactString,
    }

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
            description = "The Vintage Story mod ID",
            example = "12643",
        ),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        user: GetUser,
        mut server: GetServer,
        activity_logger: GetServerActivityLogger,
        Path((_server, id)): Path<(String, String)>,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        if let Err(errors) = shared::utils::validate_data(&data) {
            return ApiResponse::new_serialized(ApiError::new_strings_value(errors))
                .with_status(axum::http::StatusCode::BAD_REQUEST)
                .ok();
        }

        permissions.has_server_permission("workshop.manage")?;
        crate::routes::ensure_vintagestory_workshop(&server)?;

        if !data.file_name.ends_with(".zip") && !data.file_name.ends_with(".cs") {
            return ApiResponse::error("only .zip and .cs mod files are supported")
                .with_status(axum::http::StatusCode::BAD_REQUEST)
                .ok();
        }

        let bytes = reqwest::get(data.download_url.as_str())
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|err| anyhow::anyhow!("failed to download {}: {err}", data.file_name))?
            .bytes()
            .await
            .map_err(|err| anyhow::anyhow!("failed to download {}: {err}", data.file_name))?;

        crate::files::write_mod_file(
            &state,
            &mut server,
            user.uuid,
            &data.file_name,
            bytes.to_vec(),
        )
        .await?;

        activity_logger
            .log(
                "server:workshop.install",
                serde_json::json!({
                    "mod_id": id,
                    "file_name": data.file_name,
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
