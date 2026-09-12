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

    const RESULTS_PER_PAGE: u32 = 20;

    fn default_page() -> u32 {
        1
    }

    #[derive(ToSchema, Deserialize)]
    pub struct Params {
        search: Option<String>,
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
            description = "Search text to filter Modrinth results by",
        ),
        (
            "sort" = Option<String>, Query,
            description = "Sort order for the results (relevance, downloads, follows, newest, updated)",
        ),
        (
            "page" = u32, Query,
            description = "The page number",
            example = "1",
        ),
    ))]
    pub async fn route(
        state: shared::GetState,
        permissions: GetPermissionManager,
        server: GetServer,
        Query(params): Query<Params>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("minecraft_workshop.read")?;
        let config = crate::variables::resolve(&state, &server).await?;

        let result = crate::modrinth::search_projects(crate::modrinth::SearchProjects {
            query: params.search.as_deref(),
            project_type: &config.project_type,
            loader: &config.loader,
            sort: params.sort.as_deref(),
            page: params.page,
            per_page: RESULTS_PER_PAGE,
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
