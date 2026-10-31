//! Thin wrappers around the Wings files API for the server's `data/Mods` directory - this is the
//! only place installed mods live for Vintage Story (flat `.zip` files, no config to patch; the
//! game's own mod loader reads `modinfo.json` straight out of each zip at boot).

const MODS_DIR: &str = "data/Mods";

pub(crate) async fn list_installed(
    state: &shared::State,
    server: &mut shared::models::server::Server,
) -> Result<Vec<wings_api::DirectoryEntry>, anyhow::Error> {
    let result = server
        .node
        .fetch_cached(&state.database)
        .await?
        .api_client(&state.database)
        .await?
        .get_servers_server_files_list_directory(
            server.uuid,
            &wings_api::servers_server_files_list_directory::get::Query {
                directory: Some(MODS_DIR.into()),
                ..Default::default()
            },
        )
        .await;

    // The Mods directory doesn't exist until the first mod is ever installed - treat that as an
    // empty list rather than an error.
    match result {
        Ok(entries) => Ok(entries
            .into_iter()
            .filter(|entry| entry.file && !entry.directory)
            .collect()),
        Err(wings_api::client::ApiHttpError::Http(reqwest::StatusCode::NOT_FOUND, _)) => {
            Ok(Vec::new())
        }
        Err(err) => Err(err.into()),
    }
}

pub(crate) async fn write_mod_file(
    state: &shared::State,
    server: &mut shared::models::server::Server,
    user_uuid: uuid::Uuid,
    filename: &str,
    bytes: Vec<u8>,
) -> Result<(), anyhow::Error> {
    server
        .node
        .fetch_cached(&state.database)
        .await?
        .api_client(&state.database)
        .await?
        .post_servers_server_files_write(
            server.uuid,
            wings_api::client::AsyncRequestReader::new(std::io::Cursor::new(bytes)),
            &wings_api::servers_server_files_write::post::Query {
                file: Some(format!("{MODS_DIR}/{filename}").into()),
                user: Some(user_uuid),
                ignored: server.subuser_ignored_files.clone(),
                ..Default::default()
            },
        )
        .await?;

    Ok(())
}

pub(crate) async fn delete_mod_file(
    state: &shared::State,
    server: &mut shared::models::server::Server,
    filename: &str,
) -> Result<(), anyhow::Error> {
    server
        .node
        .fetch_cached(&state.database)
        .await?
        .api_client(&state.database)
        .await?
        .post_servers_server_files_delete(
            server.uuid,
            &wings_api::servers_server_files_delete::post::RequestBody {
                root: MODS_DIR.into(),
                files: vec![filename.into()],
            },
        )
        .await?;

    Ok(())
}
