use axum::http::StatusCode;
use shared::{models::server_variable::ServerVariable, response::ApiResponse};

pub struct WorkshopConfig {
    pub loader: String,
    pub project_type: String,
    pub install_dir: &'static str,
    pub mc_version: String,
}

fn missing() -> ApiResponse {
    ApiResponse::error("workshop mods are not supported for this server's egg")
        .with_status(StatusCode::BAD_REQUEST)
}

/// Resolves the WORKSHOP_LOADER / WORKSHOP_TYPE / WORKSHOP_MC_VERSION_VARIABLE hidden variables
/// an egg must declare to support this extension, plus the actual current Minecraft version
/// through the indirection the last one points at (different eggs name their MC-version
/// variable differently, e.g. MINECRAFT_VERSION vs MC_VERSION).
pub(crate) async fn resolve(
    state: &shared::State,
    server: &shared::models::server::Server,
) -> Result<WorkshopConfig, ApiResponse> {
    if !server
        .egg
        .features
        .iter()
        .any(|feature| feature == "minecraft_workshop")
    {
        return Err(missing());
    }

    let variables =
        ServerVariable::all_by_server_uuid_egg_uuid(&state.database, server.uuid, server.egg.uuid)
            .await
            .map_err(ApiResponse::from)?;

    let find = |name: &str| {
        variables
            .iter()
            .find(|v| v.variable.env_variable == name)
            .map(|v| v.value.clone())
    };

    let loader = find("WORKSHOP_LOADER").ok_or_else(missing)?;
    let project_type = find("WORKSHOP_TYPE").ok_or_else(missing)?;
    let mc_version_variable = find("WORKSHOP_MC_VERSION_VARIABLE").ok_or_else(missing)?;

    let install_dir = match project_type.as_str() {
        "plugin" => "/plugins",
        "mod" => "/mods",
        _ => return Err(missing()),
    };

    let mc_version = find(&mc_version_variable).ok_or_else(missing)?;

    Ok(WorkshopConfig {
        loader,
        project_type,
        install_dir,
        mc_version,
    })
}
