use axum::http::StatusCode;
use shared::response::ApiResponse;

/// Whitelist/ban file format is identical across every Java Edition loader (vanilla, Paper,
/// Fabric, Forge, ...), so unlike the Workshop extension this doesn't need any loader-specific
/// variables - just confirmation the egg opted in, same as Workshop's own feature-flag check.
pub(crate) fn require_feature(server: &shared::models::server::Server) -> Result<(), ApiResponse> {
    if server
        .egg
        .features
        .iter()
        .any(|feature| feature == "minecraft_whitelist")
    {
        Ok(())
    } else {
        Err(
            ApiResponse::error("whitelist/ban management is not supported for this server's egg")
                .with_status(StatusCode::BAD_REQUEST),
        )
    }
}
