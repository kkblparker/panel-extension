use utoipa_axum::{router::OpenApiRouter, routes};

mod _filename_;

mod get {
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct InstalledMod {
        #[serde(rename = "fileName")]
        file_name: String,
        size: u64,
        modified: chrono::DateTime<chrono::Local>,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        mods: Vec<InstalledMod>,
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
        permissions.has_server_permission("workshop.read")?;
        crate::routes::ensure_vintagestory_workshop(&server)?;

        let entries = crate::files::list_installed(&state, &mut server).await?;
        let mods = entries
            .into_iter()
            .filter(|entry| entry.name.ends_with(".zip") || entry.name.ends_with(".cs"))
            .map(|entry| InstalledMod {
                file_name: entry.name.to_string(),
                size: entry.size,
                modified: entry.modified,
            })
            .collect();

        ApiResponse::new_serialized(Response { mods }).ok()
    }
}

pub fn router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .nest("/{filename}", _filename_::router(state))
        .with_state(state.clone())
}
