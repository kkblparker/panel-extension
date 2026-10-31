use tokio::io::AsyncReadExt;

pub(crate) const WHITELIST_FILE: &str = "/whitelist.json";
pub(crate) const BANNED_PLAYERS_FILE: &str = "/banned-players.json";

pub(crate) async fn read_json_array(
    state: &shared::State,
    server: &mut shared::models::server::Server,
    file: &str,
) -> Result<Vec<serde_json::Value>, anyhow::Error> {
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
                file: Some(file.into()),
                max_size: Some(max_size),
                ignored: server.subuser_ignored_files.clone(),
                ..Default::default()
            },
        )
        .await;

    let mut reader = match result {
        Ok(reader) => reader,
        // No file yet (fresh server, nothing whitelisted/banned so far) - not an error.
        Err(wings_api::client::ApiHttpError::Http(axum::http::StatusCode::NOT_FOUND, _)) => {
            return Ok(Vec::new());
        }
        Err(err) => return Err(err.into()),
    };

    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await?;

    if buf.trim_ascii().is_empty() {
        return Ok(Vec::new());
    }

    Ok(serde_json::from_slice(&buf)?)
}

pub(crate) async fn write_json_array(
    state: &shared::State,
    server: &mut shared::models::server::Server,
    file: &str,
    user_uuid: uuid::Uuid,
    entries: &[serde_json::Value],
) -> Result<(), anyhow::Error> {
    let body = serde_json::to_vec_pretty(entries)?;

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
                file: Some(file.into()),
                user: Some(user_uuid),
                ignored: server.subuser_ignored_files.clone(),
                ..Default::default()
            },
        )
        .await?;

    Ok(())
}
