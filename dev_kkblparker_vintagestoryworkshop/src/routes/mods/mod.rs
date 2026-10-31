use utoipa_axum::{router::OpenApiRouter, routes};

mod _id_;

mod get {
    use axum::extract::Query;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError,
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    fn default_page() -> u32 {
        1
    }

    #[derive(ToSchema, Deserialize)]
    pub struct Params {
        search: Option<String>,
        #[serde(default = "default_page")]
        page: u32,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        #[schema(value_type = Vec<Object>)]
        mods: Vec<serde_json::Value>,
        total: u64,
        per_page: u32,
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
            "search" = Option<String>, Query,
            description = "Search text to filter workshop mods by",
        ),
        (
            "page" = u32, Query,
            description = "The page number",
            example = "1",
        ),
    ))]
    pub async fn route(
        permissions: GetPermissionManager,
        server: GetServer,
        Query(params): Query<Params>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("workshop.read")?;
        crate::routes::ensure_vintagestory_workshop(&server)?;

        let result = crate::vsmoddb::search_mods(crate::vsmoddb::SearchMods {
            search: params.search.as_deref(),
            page: params.page,
        })
        .await?;

        ApiResponse::new_serialized(Response {
            mods: result.mods,
            total: result.total,
            per_page: result.per_page,
        })
        .ok()
    }
}

pub fn router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .nest("/{id}", _id_::router(state))
        .with_state(state.clone())
}
