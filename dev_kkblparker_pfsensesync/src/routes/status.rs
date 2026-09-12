use shared::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use axum::extract::Query;
    use serde::{Deserialize, Serialize};
    use shared::{
        GetState,
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    use crate::{config::Config, sync::ManagedAllocationStatus};

    #[derive(ToSchema, Deserialize)]
    pub struct Params {
        /// Show the real error on failure, even if not saved yet.
        #[serde(default)]
        debug: bool,
    }

    #[derive(ToSchema, Serialize)]
    pub struct Response {
        allocations: Vec<ManagedAllocationStatus>,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
    ), params(
        (
            "debug" = Option<bool>, Query,
            description = "Show the real error on failure, even if not saved yet",
        ),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        Query(params): Query<Params>,
    ) -> ApiResponseResult {
        permissions.has_admin_permission("extensions.manage")?;

        let mut config = Config::load(&state).await?;
        if params.debug {
            config.debug = true;
        }

        let allocations = match crate::sync::status(&state, &config).await {
            Ok(allocations) => allocations,
            Err(err) => {
                return crate::routes::pfsense_call_error(
                    &config,
                    "failed to fetch status from pfSense",
                    err,
                )
                .ok();
            }
        };

        ApiResponse::new_serialized(Response { allocations }).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .with_state(state.clone())
}
