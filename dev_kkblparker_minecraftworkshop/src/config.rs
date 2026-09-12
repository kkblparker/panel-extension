use tokio::io::AsyncReadExt;

const MANIFEST_FILE_NAME: &str = ".calagopus-workshop.json";

fn manifest_path(install_dir: &str) -> String {
    format!("{install_dir}/{MANIFEST_FILE_NAME}")
}

pub(crate) async fn read_manifest(
    state: &shared::State,
    server: &mut shared::models::server::Server,
    install_dir: &str,
) -> Result<serde_json::Value, anyhow::Error> {
    let max_size = state
        .settings
        .get_as(|s| s.server.max_file_manager_view_size)
        .await?;

    let result = server
        .node
        .fetch_cached(&state.database)
        .await?
        .api_client(&state.database)
        .await?
        .get_servers_server_files_contents(
            server.uuid,
            &wings_api::servers_server_files_contents::get::Query {
                file: Some(manifest_path(install_dir).into()),
                max_size: Some(max_size),
                ignored: server.subuser_ignored_files.clone(),
                ..Default::default()
            },
        )
        .await;

    let mut reader = match result {
        Ok(reader) => reader,
        // No manifest yet (nothing installed through the Workshop tab so far) - not an error.
        Err(wings_api::client::ApiHttpError::Http(axum::http::StatusCode::NOT_FOUND, _)) => {
            return Ok(serde_json::json!({ "installed": [] }));
        }
        Err(err) => return Err(err.into()),
    };

    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await?;

    if buf.trim_ascii().is_empty() {
        return Ok(serde_json::json!({ "installed": [] }));
    }

    Ok(serde_json::from_slice(&buf)?)
}

pub(crate) fn installed_array(manifest: &serde_json::Value) -> Vec<serde_json::Value> {
    manifest
        .get("installed")
        .and_then(|installed| installed.as_array())
        .cloned()
        .unwrap_or_default()
}

pub(crate) async fn write_manifest(
    state: &shared::State,
    server: &mut shared::models::server::Server,
    install_dir: &str,
    user_uuid: uuid::Uuid,
    manifest: &serde_json::Value,
) -> Result<(), anyhow::Error> {
    let body = serde_json::to_vec_pretty(manifest)?;

    server
        .node
        .fetch_cached(&state.database)
        .await?
        .api_client(&state.database)
        .await?
        .post_servers_server_files_write(
            server.uuid,
            wings_api::client::AsyncRequestReader::new(std::io::Cursor::new(body)),
            &wings_api::servers_server_files_write::post::Query {
                file: Some(manifest_path(install_dir).into()),
                user: Some(user_uuid),
                ignored: server.subuser_ignored_files.clone(),
                ..Default::default()
            },
        )
        .await?;

    Ok(())
}
