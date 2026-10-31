//! Periodic panel-wide player-count poll.
//!
//! There is no built-in panel-wide finder for "every server that declares a
//! query protocol, with its primary allocation's ip/port" - the same gap
//! `dev_kkblparker_pfsensesync`'s `fetch_desired` documents and works around
//! with a raw SQL join. We do the same here, joined through
//! `nest_egg_variables`/`server_variables` instead of `server_allocations`
//! alone, to also pick up each server's `SERVER_QUERY_PROTOCOL` value (a
//! hidden, egg-author-set variable - see `crate::lib` docs).
//!
//! Results are cached (not persisted to a table): a live player count is
//! meaningless once stale by more than a poll interval or two, so
//! `shared::cache::Cache`'s TTL is exactly the right tool - no migration,
//! no cleanup job for deleted servers.

use std::time::Duration;

use futures_util::StreamExt;
use shared::State;
use sqlx::Row;
use uuid::Uuid;

use crate::query::{self, PlayerCountStatus};

pub const POLL_TASK: &str = "dev.kkblparker.playercount:poll";
const POLL_INTERVAL: Duration = Duration::from_secs(15);
const CACHE_TTL_SECS: u64 = 30;
const MAX_CONCURRENT_QUERIES: usize = 20;

pub fn cache_key(server_uuid: Uuid) -> String {
    format!("playercount:{server_uuid}")
}

struct QueryTarget {
    server_uuid: Uuid,
    ip: String,
    port: u16,
    protocol: String,
}

/// Resolves an egg's `SERVER_QUERY_PORT_VARIABLE` indirection, if present -
/// the name of another of that egg's variables holding the *actual* live
/// query port. Only needed for eggs where the query protocol doesn't share
/// the main game port (Arma Reforger's A2S responder, which requires its own
/// UDP port entirely separate from `bindPort`/`publicPort` - confirmed
/// against the current server config schema). Absent for eggs where the
/// primary allocation port is already correct (real Source-engine games:
/// Arma 3, Project Zomboid).
async fn resolve_query_port(state: &State, server_uuid: Uuid, variable_name: &str) -> Option<u16> {
    let row = sqlx::query(sqlx::AssertSqlSafe(
        r#"
        SELECT server_variables.value AS value
        FROM server_variables
        JOIN nest_egg_variables ON nest_egg_variables.uuid = server_variables.variable_uuid
        WHERE server_variables.server_uuid = $1 AND nest_egg_variables.env_variable = $2
        "#
        .to_string(),
    ))
    .bind(server_uuid)
    .bind(variable_name)
    .fetch_optional(state.database.read())
    .await
    .ok()??;

    let value: String = row.try_get("value").ok()?;
    value.trim().parse::<u16>().ok()
}

/// One poll pass: find every server that declares `SERVER_QUERY_PROTOCOL`,
/// query each concurrently (bounded), and cache the result.
pub async fn poll_once(state: &State) -> anyhow::Result<()> {
    let targets = fetch_targets(state).await?;

    futures_util::stream::iter(targets)
        .for_each_concurrent(MAX_CONCURRENT_QUERIES, |target| async move {
            let status = query::query(&target.protocol, &target.ip, target.port).await;
            if let Err(err) = state.cache.set(&cache_key(target.server_uuid), CACHE_TTL_SECS, &status).await {
                tracing::error!(
                    server = %target.server_uuid,
                    "player count: failed to write cache entry: {err:?}"
                );
            }
        })
        .await;

    Ok(())
}

pub async fn get_cached(state: &State, server_uuid: Uuid) -> Option<PlayerCountStatus> {
    state.cache.get::<PlayerCountStatus>(&cache_key(server_uuid)).await.ok().flatten()
}

/// Static SQL, no user input interpolated. `host()` yields the bare IP
/// address text (no CIDR suffix) from the inet/cidr column - same helper
/// `dev_kkblparker_pfsensesync` uses. Joins through the server's *primary*
/// allocation (`servers.allocation_uuid`), not every allocation it holds -
/// that's the address players actually connect to.
async fn fetch_targets(state: &State) -> anyhow::Result<Vec<QueryTarget>> {
    let sql = r#"
        SELECT
            servers.uuid AS server_uuid,
            host(node_allocations.ip) AS ip,
            node_allocations.port AS port,
            COALESCE(server_variables.value, nest_egg_variables.default_value) AS protocol,
            COALESCE(port_var_sv.value, port_var_nev.default_value) AS port_variable_name
        FROM servers
        JOIN server_allocations ON server_allocations.uuid = servers.allocation_uuid
        JOIN node_allocations ON node_allocations.uuid = server_allocations.allocation_uuid
        JOIN nest_egg_variables
            ON nest_egg_variables.egg_uuid = servers.egg_uuid
            AND nest_egg_variables.env_variable = 'SERVER_QUERY_PROTOCOL'
        LEFT JOIN server_variables
            ON server_variables.variable_uuid = nest_egg_variables.uuid
            AND server_variables.server_uuid = servers.uuid
        LEFT JOIN nest_egg_variables port_var_nev
            ON port_var_nev.egg_uuid = servers.egg_uuid
            AND port_var_nev.env_variable = 'SERVER_QUERY_PORT_VARIABLE'
        LEFT JOIN server_variables port_var_sv
            ON port_var_sv.variable_uuid = port_var_nev.uuid
            AND port_var_sv.server_uuid = servers.uuid
        WHERE NOT servers.suspended
    "#
    .to_string();

    let rows = sqlx::query(sqlx::AssertSqlSafe(sql)).fetch_all(state.database.read()).await?;

    let mut targets = Vec::with_capacity(rows.len());
    for row in &rows {
        let ip: String = row.try_get("ip")?;
        let primary_port: i32 = row.try_get("port")?;
        let protocol: String = row.try_get("protocol")?;
        if ip.is_empty() || protocol.is_empty() {
            continue;
        }

        let server_uuid: Uuid = row.try_get("server_uuid")?;
        let port_variable_name: Option<String> = row.try_get("port_variable_name")?;

        let port = match port_variable_name {
            Some(name) if !name.is_empty() => resolve_query_port(state, server_uuid, &name)
                .await
                .unwrap_or(primary_port as u16),
            _ => primary_port as u16,
        };

        targets.push(QueryTarget {
            server_uuid,
            ip,
            port,
            protocol,
        });
    }

    Ok(targets)
}

pub fn interval() -> Duration {
    POLL_INTERVAL
}
