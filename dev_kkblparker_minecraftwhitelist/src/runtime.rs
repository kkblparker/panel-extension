/// Best-effort: any failure to reach Wings/the node is treated as "not running" so callers fall
/// back to editing the file directly, which is the only thing that can work anyway if the node
/// itself can't be reached.
pub(crate) async fn is_running(
    state: &shared::State,
    server: &mut shared::models::server::Server,
) -> bool {
    let Ok(node) = server.node.fetch_cached(&state.database).await else {
        return false;
    };
    let Ok(api_client) = node.api_client(&state.database).await else {
        return false;
    };
    let Ok(response) = api_client.get_servers_server_utilization(server.uuid).await else {
        return false;
    };

    matches!(response.utilization.state, wings_api::ServerState::Running)
}

pub(crate) async fn send_command(
    state: &shared::State,
    server: &mut shared::models::server::Server,
    command: impl Into<compact_str::CompactString>,
) -> Result<(), anyhow::Error> {
    server
        .node
        .fetch_cached(&state.database)
        .await?
        .api_client(&state.database)
        .await?
        .post_servers_server_commands(
            server.uuid,
            &wings_api::servers_server_commands::post::RequestBody {
                commands: vec![command.into()],
            },
        )
        .await?;

    // Give the server a moment to process the command and flush the resulting file write before
    // the caller re-reads it for the response.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    Ok(())
}
