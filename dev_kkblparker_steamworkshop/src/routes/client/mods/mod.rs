use shared::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod _id_;

mod get {
    use axum::extract::Query;
    use serde::{Deserialize, Serialize};
    use shared::{
        ApiError, GetState,
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
        sort: Option<String>,
        #[serde(default = "default_page")]
        page: u32,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        #[schema(value_type = Object)]
        result: serde_json::Value,
        /// Which `ModListKind`s this egg supports - a single entry means the frontend should just
        /// show a plain "Add" button, more than one means it should offer a picker.
        available_kinds: Vec<crate::variables::ModListKind>,
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
            description = "Search text to filter Workshop results by",
        ),
        (
            "sort" = Option<String>, Query,
            description = "Sort order for the results when not searching by text (\"newest\", \"recently-updated\", \"trending\", or omitted for most popular)",
        ),
        (
            "page" = u32, Query,
            description = "The page number",
            example = "1",
        ),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        mut server: GetServer,
        Query(params): Query<Params>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("steam_workshop.read")?;
        let workshop_config = crate::variables::resolve(&state, &mut server).await?;
        let config = crate::config::Config::load(&state).await?;

        let result = crate::steam::search_workshop(crate::steam::SearchWorkshop {
            api_key: &config.api_key,
            appid: &workshop_config.appid,
            query: params.search.as_deref(),
            sort: params.sort.as_deref(),
            page: params.page,
        })
        .await?;

        ApiResponse::new_serialized(Response {
            result,
            available_kinds: workshop_config.available_kinds(),
        })
        .ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .nest("/{id}", _id_::router(state))
        .with_state(state.clone())
}
