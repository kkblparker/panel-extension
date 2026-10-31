//! Live player-count queries against a server's own game port.
//!
//! Two protocols, picked per-server by the `SERVER_QUERY_PROTOCOL` egg
//! variable ([`crate::poll`]):
//! - `"minecraft"` — Minecraft's own Server List Ping protocol (`craftping`).
//! - `"source"` — the Source-engine A2S query protocol (`a2s`), used by
//!   Arma Reforger, Arma 3, and Project Zomboid.
//!
//! Both are best-effort: a timeout or connection failure just means the
//! server is offline/unreachable right now, not an error worth logging loudly
//! on every poll tick (a stopped server is the common case, not a bug).

use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

/// How long to wait for a single server to respond before giving up on it
/// this tick. Short enough that one unreachable server can't meaningfully
/// delay the whole poll pass (queries run concurrently anyway - see
/// [`crate::poll`] - but a hung TCP connect can otherwise block far longer
/// than a UDP query ever would).
const QUERY_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerCountStatus {
    pub online: bool,
    pub players: Option<u32>,
    pub max_players: Option<u32>,
    /// The currently loaded map/scenario, for games that have one (the
    /// Source-engine A2S response carries this natively). `None` for
    /// Minecraft - it has no round-based map-switching concept, just one
    /// persistent world, so there's nothing meaningful to show here.
    pub map: Option<String>,
    /// Names of currently connected players, best-effort. Source: a
    /// separate A2S_PLAYER query, always complete when it succeeds.
    /// Minecraft: the SLP response's own `sample` list, which server
    /// software caps (vanilla defaults to 12) and some servers configure to
    /// be empty for privacy - `Some(vec![])` there means "hidden/empty
    /// sample", not necessarily "nobody online" (see `players` for the
    /// actual count). `None` only when the list couldn't be fetched at all.
    pub player_names: Option<Vec<String>>,
}

impl PlayerCountStatus {
    fn offline() -> Self {
        Self {
            online: false,
            players: None,
            max_players: None,
            map: None,
            player_names: None,
        }
    }
}

pub async fn query(protocol: &str, ip: &str, port: u16) -> PlayerCountStatus {
    let result = match protocol {
        "minecraft" => query_minecraft(ip, port).await,
        "source" => query_source(ip, port).await,
        other => {
            tracing::warn!(protocol = other, "player count: unknown SERVER_QUERY_PROTOCOL value");
            return PlayerCountStatus::offline();
        }
    };

    result.unwrap_or_else(|err| {
        tracing::debug!(ip, port, protocol, "player count: query failed (treating as offline): {err:?}");
        PlayerCountStatus::offline()
    })
}

async fn query_minecraft(ip: &str, port: u16) -> anyhow::Result<PlayerCountStatus> {
    let mut stream = tokio::time::timeout(
        QUERY_TIMEOUT,
        tokio::net::TcpStream::connect((ip, port)),
    )
    .await??;

    let response = tokio::time::timeout(QUERY_TIMEOUT, craftping::tokio::ping(&mut stream, ip, port)).await??;

    Ok(PlayerCountStatus {
        online: true,
        players: Some(response.online_players as u32),
        max_players: Some(response.max_players as u32),
        map: None,
        player_names: response
            .sample
            .map(|sample| sample.into_iter().map(|player| player.name).collect()),
    })
}

async fn query_source(ip: &str, port: u16) -> anyhow::Result<PlayerCountStatus> {
    let addr: SocketAddr = (ip, port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("could not resolve {ip}:{port}"))?;

    let (info, players) = tokio::task::spawn_blocking(move || {
        let client = a2s::A2SClient::new()?;
        let info = client.info(addr)?;
        // A2S_PLAYER is a separate query from A2S_INFO - some servers (or a
        // stricter firewall/anti-flood rule) allow one but not the other, so
        // a failure here shouldn't take down the whole result, just the
        // name list.
        let players = client.players(addr).ok();
        anyhow::Ok((info, players))
    })
    .await??;

    Ok(PlayerCountStatus {
        online: true,
        players: Some(info.players as u32),
        max_players: Some(info.max_players as u32),
        map: Some(info.map).filter(|m| !m.is_empty()),
        player_names: players.map(|players| players.into_iter().map(|player| player.name).collect()),
    })
}
