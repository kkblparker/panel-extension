use tokio::io::AsyncReadExt;

const CONFIG_FILE: &str = "config.json";

pub(crate) async fn read_config(
    state: &shared::State,
    server: &mut shared::models::server::Server,
) -> Result<serde_json::Value, anyhow::Error> {
    let max_size = state
        .settings
        .get_as(|s| s.server.max_file_manager_view_size)
        .await?;

    let mut reader = server
        .node
        .fetch_cached(&state.database)
        .await?
        .api_client(&state.database)
        .await?
        .get_servers_server_files_contents(
            server.uuid,
            &wings_api::servers_server_files_contents::get::Query {
                file: Some(CONFIG_FILE.into()),
                max_size: Some(max_size),
                ignored: server.subuser_ignored_files.clone(),
                ..Default::default()
            },
        )
        .await?;

    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await?;

    if buf.trim_ascii().is_empty() {
        return Ok(serde_json::json!({ "mods": [] }));
    }

    Ok(serde_json::from_slice(&buf)?)
}

pub(crate) fn mods_array(config: &serde_json::Value) -> Vec<serde_json::Value> {
    config
        .get("mods")
        .and_then(|mods| mods.as_array())
        .cloned()
        .unwrap_or_default()
}

pub(crate) async fn write_config(
    state: &shared::State,
    server: &mut shared::models::server::Server,
    user_uuid: uuid::Uuid,
    config: &serde_json::Value,
) -> Result<(), anyhow::Error> {
    let body = serde_json::to_vec_pretty(config)?;

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
                file: Some(CONFIG_FILE.into()),
                user: Some(user_uuid),
                ignored: server.subuser_ignored_files.clone(),
                ..Default::default()
            },
        )
        .await?;

    Ok(())
}
