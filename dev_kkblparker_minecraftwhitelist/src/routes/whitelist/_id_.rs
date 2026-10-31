use utoipa_axum::{router::OpenApiRouter, routes};

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

    use crate::entries::{WhitelistEntry, parse_whitelist};

    #[derive(ToSchema, Serialize)]
    struct Response {
        entries: Vec<WhitelistEntry>,
    }

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
            "uuid" = String,
            description = "The whitelist entry's player UUID",
        ),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        user: GetUser,
        mut server: GetServer,
        activity_logger: GetServerActivityLogger,
        Path((_server, entry_uuid)): Path<(String, String)>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("minecraft_whitelist.manage")?;
        crate::gate::require_feature(&server)?;

        let mut raw =
            crate::files::read_json_array(&state, &mut server, crate::files::WHITELIST_FILE).await?;

        let removed_name = raw
            .iter()
            .find(|v| v.get("uuid").and_then(|x| x.as_str()) == Some(entry_uuid.as_str()))
            .and_then(|v| v.get("name").and_then(|x| x.as_str()).map(str::to_string));

        if crate::runtime::is_running(&state, &mut server).await {
            if let Some(name) = &removed_name {
                crate::runtime::send_command(&state, &mut server, format!("whitelist remove {name}"))
                    .await?;
            }
        } else {
            raw.retain(|v| v.get("uuid").and_then(|x| x.as_str()) != Some(entry_uuid.as_str()));
            crate::files::write_json_array(
                &state,
                &mut server,
                crate::files::WHITELIST_FILE,
                user.uuid,
                &raw,
            )
            .await?;
        }

        let raw =
            crate::files::read_json_array(&state, &mut server, crate::files::WHITELIST_FILE).await?;

        activity_logger
            .log(
                "server:minecraft-whitelist.remove",
                serde_json::json!({ "name": removed_name, "uuid": entry_uuid }),
            )
            .await;

        ApiResponse::new_serialized(Response {
            entries: parse_whitelist(raw),
        })
        .ok()
    }
}

pub fn router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .routes(routes!(delete::route))
        .with_state(state.clone())
}
