//! Minimal `key=value` INI line editor, plus the Wings file read/write plumbing to use it on a
//! server's config file (e.g. Project Zomboid's `Zomboid/Server/<name>.ini`, which - unlike Arma
//! 3's `MODIFICATIONS` env variable - is a plain file the game server itself reads on boot, not
//! something exposed as an egg variable).
//!
//! Deliberately dumb: it only ever touches the one line matching `key=`, leaving every other
//! line, comment, and ordering exactly as it was. It does not understand INI sections - none of
//! the games this extension targets so far use them for their mod-list keys.

use tokio::io::AsyncReadExt;

/// Returns the trimmed value of the first `key=...` line, if present.
pub(crate) fn get_key(content: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");

    for line in content.lines() {
        if let Some(value) = line.trim_start().strip_prefix(&prefix) {
            return Some(value.trim().to_string());
        }
    }

    None
}

/// Replaces the first `key=...` line's value, or appends a new `key=value` line if none exists.
/// Preserves every other line untouched, and matches the file's existing line ending style.
pub(crate) fn set_key(content: &str, key: &str, value: &str) -> String {
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let prefix = format!("{key}=");
    let mut found = false;

    let mut lines: Vec<String> = content
        .lines()
        .map(|line| {
            if !found && line.trim_start().starts_with(&prefix) {
                found = true;
                format!("{key}={value}")
            } else {
                line.to_string()
            }
        })
        .collect();

    if !found {
        lines.push(format!("{key}={value}"));
    }

    lines.join(newline)
}

/// Reads a text file from the server via Wings. A missing file (server never started, or the
/// game hasn't generated its config yet) is treated as an empty string, not an error.
pub(crate) async fn read(
    state: &shared::State,
    server: &mut shared::models::server::Server,
    path: &str,
) -> Result<String, anyhow::Error> {
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
                file: Some(path.into()),
                max_size: Some(max_size),
                ignored: server.subuser_ignored_files.clone(),
                ..Default::default()
            },
        )
        .await;

    let mut reader = match result {
        Ok(reader) => reader,
        Err(wings_api::client::ApiHttpError::Http(axum::http::StatusCode::NOT_FOUND, _)) => {
            return Ok(String::new());
        }
        Err(err) => return Err(err.into()),
    };

    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await?;

    Ok(String::from_utf8_lossy(&buf).into_owned())
}

pub(crate) async fn write(
    state: &shared::State,
    server: &mut shared::models::server::Server,
    user_uuid: uuid::Uuid,
    path: &str,
    content: &str,
) -> Result<(), anyhow::Error> {
    server
        .node
        .fetch_cached(&state.database)
        .await?
        .api_client(&state.database)
        .await?
        .post_servers_server_files_write(
            server.uuid,
            wings_api::client::AsyncRequestReader::new(std::io::Cursor::new(
                content.as_bytes().to_vec(),
            )),
            &wings_api::servers_server_files_write::post::Query {
                file: Some(path.into()),
                user: Some(user_uuid),
                ignored: server.subuser_ignored_files.clone(),
                ..Default::default()
            },
        )
        .await?;

    Ok(())
}
