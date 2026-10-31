use utoipa_axum::{router::OpenApiRouter, routes};

mod _id_;

mod get {
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    use crate::entries::{BanEntry, parse_bans};

    #[derive(ToSchema, Serialize)]
    struct Response {
        entries: Vec<BanEntry>,
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
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        mut server: GetServer,
    ) -> ApiResponseResult {
        permissions.has_server_permission("minecraft_whitelist.read")?;
        crate::gate::require_feature(&server)?;

        let raw =
            crate::files::read_json_array(&state, &mut server, crate::files::BANNED_PLAYERS_FILE)
                .await?;

        ApiResponse::new_serialized(Response {
            entries: parse_bans(raw),
        })
        .ok()
    }
}

mod post {
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

    use crate::entries::{BanEntry, parse_bans};

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        #[garde(length(chars, min = 1, max = 32))]
        #[schema(min_length = 1, max_length = 32)]
        username: compact_str::CompactString,
        #[garde(skip)]
        #[schema(max_length = 512)]
        reason: Option<compact_str::CompactString>,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        entries: Vec<BanEntry>,
    }

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
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        user: GetUser,
        mut server: GetServer,
        activity_logger: GetServerActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        if let Err(errors) = shared::utils::validate_data(&data) {
            return ApiResponse::new_serialized(ApiError::new_strings_value(errors))
                .with_status(axum::http::StatusCode::BAD_REQUEST)
                .ok();
        }

        permissions.has_server_permission("minecraft_whitelist.manage")?;
        crate::gate::require_feature(&server)?;

        let profile = crate::mojang::resolve_username(&data.username).await?;
        let reason = data
            .reason
            .as_deref()
            .filter(|r| !r.trim().is_empty())
            .unwrap_or("Banned by an operator.")
            .to_string();

        if crate::runtime::is_running(&state, &mut server).await {
            crate::runtime::send_command(
                &state,
                &mut server,
                format!("ban {} {}", profile.name, reason),
            )
            .await?;
        } else {
            let mut raw = crate::files::read_json_array(
                &state,
                &mut server,
                crate::files::BANNED_PLAYERS_FILE,
            )
            .await?;
            raw.retain(|v| v.get("uuid").and_then(|x| x.as_str()) != Some(profile.uuid.as_str()));
            raw.push(serde_json::json!({
                "uuid": profile.uuid,
                "name": profile.name,
                "created": chrono::Local::now().to_rfc3339(),
                "source": "Panel",
                "expires": "forever",
                "reason": reason,
            }));
            crate::files::write_json_array(
                &state,
                &mut server,
                crate::files::BANNED_PLAYERS_FILE,
                user.uuid,
                &raw,
            )
            .await?;
        }

        let raw =
            crate::files::read_json_array(&state, &mut server, crate::files::BANNED_PLAYERS_FILE)
                .await?;

        activity_logger
            .log(
                "server:minecraft-whitelist.ban",
                serde_json::json!({ "name": profile.name, "uuid": profile.uuid, "reason": reason }),
            )
            .await;

        ApiResponse::new_serialized(Response {
            entries: parse_bans(raw),
        })
        .ok()
    }
}

pub fn router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .routes(routes!(post::route))
        .nest("/{uuid}", _id_::router(state))
        .with_state(state.clone())
}
