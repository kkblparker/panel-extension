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
        #[garde(length(max = 255))]
        #[schema(max_length = 255)]
        name: Option<compact_str::CompactString>,
        #[garde(length(max = 64))]
        #[schema(max_length = 64)]
        version: Option<compact_str::CompactString>,
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
            description = "The workshop mod ID",
            example = "5965550F24A0C152",
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
        crate::routes::ensure_reforger_workshop(&server)?;

        let mut config = crate::config::read_config(&state, &mut server).await?;
        let mut mods = crate::config::mods_array(&config);

        mods.retain(|entry| entry.get("modId").and_then(|v| v.as_str()) != Some(id.as_str()));
        mods.push(serde_json::json!({
            "modId": id,
            "name": data.name,
            "version": data.version,
        }));

        config["mods"] = serde_json::Value::Array(mods);
        crate::config::write_config(&state, &mut server, user.uuid, &config).await?;

        activity_logger
            .log(
                "server:workshop.install",
                serde_json::json!({
                    "mod_id": id,
                    "name": data.name,
                    "version": data.version,
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
