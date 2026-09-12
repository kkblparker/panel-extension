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

    // Must match RESULTS_PER_PAGE in the frontend's ServerWorkshop.tsx, which uses a full page
    // of results as its signal that a next page likely exists.
    const RESULTS_PER_PAGE: u32 = 20;

    #[derive(ToSchema, Deserialize)]
    pub struct Params {
        search: Option<String>,
        #[serde(default)]
        tags: Vec<String>,
        sort: Option<String>,
        #[serde(default = "default_page")]
        page: u32,
    }

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
            "search" = Option<String>, Query,
            description = "Search text to filter workshop mods by",
        ),
        (
            "tags" = Vec<String>, Query,
            description = "Workshop tags to filter mods by",
        ),
        (
            "sort" = Option<String>, Query,
            description = "Sort order for the results",
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
        crate::routes::ensure_reforger_workshop(&server)?;

        let result = crate::reforger_workshop::search_mods(crate::reforger_workshop::SearchMods {
            search: params.search.as_deref(),
            tags: &params.tags,
            sort: params.sort.as_deref(),
            page: params.page,
            per_page: Some(RESULTS_PER_PAGE),
        })
        .await?;

        ApiResponse::new_serialized(Response { result }).ok()
    }
}

pub fn router(state: &shared::State) -> OpenApiRouter<shared::State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .nest("/{id}", _id_::router(state))
        .with_state(state.clone())
}
