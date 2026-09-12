//! Reconciliation between Calagopus server allocations and pfSense NAT
//! port-forward rules.
//!
//! The panel is the source of truth. Every in-use server allocation (an
//! `IP:port` assigned to a server) should have exactly one corresponding
//! port-forward on the firewall. This module computes the difference between
//! the desired set (from the database) and the actual set of *managed*
//! port-forwards on pfSense, then creates/deletes/toggles to converge.
//!
//! We only ever touch port-forwards whose `descr` starts with
//! [`crate::config::MANAGED_MARKER`], and we embed the owning server/allocation
//! UUIDs in that description so a rule can be matched back to its allocation
//! across reconciles. Rules created by hand in the pfSense UI are never
//! touched.
//!
//! A rule exists for as long as its allocation exists (deleted the moment the
//! server or allocation is deleted — cascades make it vanish from the desired
//! set on the very next pass), but it is only ever *enabled* while the server
//! is actually running. A suspended server, or one that is stopped/starting/
//! stopping/unreachable, gets its forward disabled rather than deleted — so
//! toggling power state is a single PATCH instead of a delete+recreate. If we
//! cannot determine a server's power state at all (its node is unreachable),
//! we fail closed and treat it as not running: better a false "offline" than
//! a port left open on a server we can no longer see.
//!
//! pfSense has no concept of "reserved but not yet forwarded" ports, so
//! nothing stops an admin from manually creating a NAT rule (for something
//! entirely unrelated to Calagopus) on the same interface + external port an
//! allocation already uses. We can't prevent that allocation from being
//! created in the first place — that's the core panel's own allocation pool,
//! outside this extension's reach — but we do refuse to create or replace a
//! forward that would collide with such a rule, rather than silently
//! producing two rules fighting over the same port. See
//! [`find_conflicting_unmanaged`].

use std::collections::HashMap;

use shared::State;
use shared::models::node::Node;
use shared::prelude::ByUuid;
use sqlx::Row;
use uuid::Uuid;

use crate::config::{Config, MANAGED_MARKER};
use crate::pfsense::PfSenseClient;

/// A port-forward we want to exist, derived from a server allocation.
#[derive(Debug, Clone)]
struct Desired {
    server_uuid: Uuid,
    /// Display-only — carried through for [`status`], not used by the
    /// reconcile diff itself.
    server_name: String,
    allocation_uuid: Uuid,
    /// Internal game-server IP (the forward target).
    ip: String,
    /// Port (both the external destination port and the internal local port).
    port: i32,
    /// Whether the forward should be disabled (server suspended, not
    /// running, or its power state could not be determined).
    disabled: bool,
}

/// A `server_allocations` row joined with the bits needed to look up its
/// server's live power state, before that lookup has happened.
struct RawAllocation {
    server_uuid: Uuid,
    server_name: String,
    allocation_uuid: Uuid,
    node_uuid: Uuid,
    ip: String,
    port: i32,
    suspended: bool,
}

impl Desired {
    /// The `descr` this allocation's port-forward should carry.
    fn descr(&self) -> String {
        format!(
            "{MANAGED_MARKER} server={} alloc={}",
            self.server_uuid, self.allocation_uuid
        )
    }
}

/// Run one reconciliation pass.
pub async fn reconcile(state: &State, config: &Config) -> anyhow::Result<()> {
    let client = PfSenseClient::new(config)?;

    let desired = fetch_desired(state).await?;
    let desired_by_alloc: HashMap<Uuid, Desired> = desired
        .iter()
        .map(|d| (d.allocation_uuid, d.clone()))
        .collect();

    let current = client.list_port_forwards().await?;

    // Only consider port-forwards this extension manages (by marker).
    let managed: Vec<&crate::pfsense::PortForwardEntry> = current
        .iter()
        .filter(|e| e.descr.starts_with(MANAGED_MARKER))
        .collect();

    // Map each managed rule back to the allocation it claims to serve.
    let mut current_by_alloc: HashMap<Uuid, &crate::pfsense::PortForwardEntry> = HashMap::new();
    let mut delete_ids: Vec<String> = Vec::new();

    for &entry in &managed {
        match parse_alloc_uuid(&entry.descr) {
            // Duplicate rule for the same allocation — keep one, drop the rest.
            Some(alloc) if current_by_alloc.contains_key(&alloc) => {
                delete_ids.push(entry.id.clone());
            }
            Some(alloc) => {
                current_by_alloc.insert(alloc, entry);
            }
            // Marked as ours but unparseable — an orphan, remove it.
            None => delete_ids.push(entry.id.clone()),
        }
    }

    let mut to_create: Vec<&Desired> = Vec::new();
    let mut to_toggle: Vec<(String, bool)> = Vec::new();
    let mut conflicts_skipped = 0usize;

    // Decide creates, mismatched updates (update == delete + recreate), and
    // plain disabled-flag toggles (server started/stopped, nothing else changed).
    for d in &desired {
        // Checked whenever we're about to create or replace a rule: is this
        // port already spoken for, either by a rule we don't own or by
        // another desired allocation (possibly on a completely different
        // node) racing for the same external port? Never checked against a
        // rule that already matches — we only refuse *new* collisions, never
        // evict something already working because a conflict showed up
        // elsewhere.
        let conflict_reason = find_conflicting_unmanaged(&current, &config.interface, d.port)
            .map(|entry| format!("an existing unmanaged rule ({})", entry.descr))
            .or_else(|| {
                find_conflicting_sibling(d, &desired, &current_by_alloc)
                    .map(|sibling| format!("server \"{}\" on a different node/allocation", sibling.server_name))
            });

        match current_by_alloc.get(&d.allocation_uuid) {
            Some(entry) => {
                // Also checked against the *current* config's interface/
                // destination, not just this allocation's own ip/port — so
                // changing either setting (e.g. fixing a bad destination
                // default) actually gets applied to already-existing rules
                // on the next pass, instead of only ever affecting new ones.
                let matches = entry.target == d.ip
                    && entry.local_port == d.port.to_string()
                    && entry.interface == config.interface
                    && entry.destination == config.destination;
                if !matches {
                    match conflict_reason {
                        Some(reason) => {
                            conflicts_skipped += 1;
                            tracing::warn!(
                                server = %d.server_uuid,
                                allocation = %d.allocation_uuid,
                                port = d.port,
                                "pfSense sync: desired port collides with {reason}; leaving this allocation's forward as-is"
                            );
                        }
                        None => {
                            delete_ids.push(entry.id.clone());
                            to_create.push(d);
                        }
                    }
                } else if entry.disabled != d.disabled {
                    to_toggle.push((entry.id.clone(), d.disabled));
                }
            }
            None => match conflict_reason {
                Some(reason) => {
                    conflicts_skipped += 1;
                    tracing::warn!(
                        server = %d.server_uuid,
                        allocation = %d.allocation_uuid,
                        port = d.port,
                        "pfSense sync: desired port collides with {reason}; not creating a forward for this allocation"
                    );
                }
                None => to_create.push(d),
            },
        }
    }

    // Managed rules whose allocation no longer exists → delete.
    for (alloc, entry) in &current_by_alloc {
        if !desired_by_alloc.contains_key(alloc) {
            delete_ids.push(entry.id.clone());
        }
    }

    if delete_ids.is_empty() && to_create.is_empty() && to_toggle.is_empty() {
        tracing::debug!(
            managed = managed.len(),
            desired = desired.len(),
            conflicts_skipped,
            "pfSense sync: already in sync"
        );
        return Ok(());
    }

    // Toggle disabled-state in place first. A PATCH mutates a rule's fields
    // without touching the array, so it can't invalidate the ids the delete
    // pass below is about to use.
    let mut toggled = 0usize;
    for (id, disabled) in &to_toggle {
        if let Err(err) = client.set_port_forward_disabled(id, *disabled).await {
            tracing::error!(
                id,
                disabled,
                "pfSense sync: failed to toggle port forward disabled state: {err:?}"
            );
        } else {
            toggled += 1;
        }
    }

    // Delete highest ids first. pfSense addresses `many` objects by array
    // index, so removing a lower index would shift the ids of everything after
    // it; descending order keeps the remaining ids we hold valid.
    delete_ids.sort_by(|a, b| numeric_id(b).cmp(&numeric_id(a)));
    delete_ids.dedup();

    let mut deleted = 0usize;
    for id in &delete_ids {
        if let Err(err) = client.delete_port_forward(id).await {
            tracing::error!(id, "pfSense sync: failed to delete port forward: {err:?}");
        } else {
            deleted += 1;
        }
    }

    let mut created = 0usize;
    for desired in &to_create {
        if let Err(err) = client
            .create_port_forward(
                config,
                &desired.descr(),
                &desired.ip,
                desired.port,
                desired.disabled,
            )
            .await
        {
            tracing::error!(
                server = %desired.server_uuid,
                allocation = %desired.allocation_uuid,
                target = %desired.ip,
                port = desired.port,
                "pfSense sync: failed to create port forward: {err:?}"
            );
        } else {
            created += 1;
        }
    }

    if deleted > 0 || created > 0 || toggled > 0 {
        client.apply().await?;
        tracing::info!(
            created,
            deleted,
            toggled,
            conflicts_skipped,
            desired = desired.len(),
            "pfSense sync: applied port-forward changes"
        );
    } else if conflicts_skipped > 0 {
        tracing::debug!(conflicts_skipped, "pfSense sync: nothing applied this pass (only conflicts pending)");
    }

    Ok(())
}

/// Enumerate every in-use allocation across the whole panel, together with
/// whether the forward for it should currently be enabled. There is no
/// built-in panel-wide finder for this, so we query directly. A
/// `server_allocations` row existing means the allocation is assigned/in-use;
/// the joined `node_allocations` row carries the internal IP and port, and
/// `servers.suspended` lets us fail a suspended server's forward closed even
/// if its process hasn't actually stopped yet.
async fn fetch_desired(state: &State) -> anyhow::Result<Vec<Desired>> {
    // Static SQL, no user input interpolated. `host()` yields the bare IP
    // address text (no CIDR suffix) from the inet/cidr column.
    let sql = r#"
        SELECT
            server_allocations.server_uuid AS server_uuid,
            servers.name                    AS server_name,
            node_allocations.uuid          AS allocation_uuid,
            node_allocations.node_uuid     AS node_uuid,
            host(node_allocations.ip)      AS ip,
            node_allocations.port          AS port,
            servers.suspended              AS suspended
        FROM server_allocations
        JOIN node_allocations
            ON server_allocations.allocation_uuid = node_allocations.uuid
        JOIN servers
            ON servers.uuid = server_allocations.server_uuid
    "#
    .to_string();

    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
        .fetch_all(state.database.read())
        .await?;

    let mut raw = Vec::with_capacity(rows.len());
    for row in &rows {
        let ip: String = row.try_get("ip")?;
        if ip.is_empty() {
            continue;
        }

        raw.push(RawAllocation {
            server_uuid: row.try_get("server_uuid")?,
            server_name: row.try_get("server_name")?,
            allocation_uuid: row.try_get("allocation_uuid")?,
            node_uuid: row.try_get("node_uuid")?,
            ip,
            port: row.try_get("port")?,
            suspended: row.try_get("suspended")?,
        });
    }

    // Group by node so each node's (cached, 15s TTL) resource-usage endpoint
    // is hit at most once per reconcile pass, no matter how many allocations
    // it hosts.
    let mut by_node: HashMap<Uuid, Vec<&RawAllocation>> = HashMap::new();
    for alloc in &raw {
        by_node.entry(alloc.node_uuid).or_default().push(alloc);
    }

    let mut running_by_server: HashMap<Uuid, bool> = HashMap::new();
    for (node_uuid, allocs) in &by_node {
        let resources: anyhow::Result<_> = async {
            let node = Node::by_uuid_cached(&state.database, *node_uuid).await?;
            node.fetch_server_resources(&state.database).await
        }
        .await;

        match resources {
            Ok(resources) => {
                for alloc in allocs {
                    let running = matches!(
                        resources.get(&alloc.server_uuid).map(|r| r.state),
                        Some(wings_api::ServerState::Running)
                    );
                    running_by_server.insert(alloc.server_uuid, running);
                }
            }
            Err(err) => {
                // Fail closed: leave these servers out of `running_by_server`
                // entirely, so the lookup below defaults them to "not
                // running" rather than risking a port left open because we
                // could not reach its node.
                tracing::warn!(
                    node = %node_uuid,
                    "pfSense sync: failed to fetch power state for node's servers, treating them as offline this pass: {err:?}"
                );
            }
        }
    }

    let mut desired = Vec::with_capacity(raw.len());
    for alloc in raw {
        let running = !alloc.suspended
            && running_by_server
                .get(&alloc.server_uuid)
                .copied()
                .unwrap_or(false);

        desired.push(Desired {
            server_uuid: alloc.server_uuid,
            server_name: alloc.server_name,
            allocation_uuid: alloc.allocation_uuid,
            ip: alloc.ip,
            port: alloc.port,
            disabled: !running,
        });
    }

    Ok(desired)
}

/// A read-only snapshot combining what the extension wants (from the
/// database) with what currently exists on pfSense, for the admin status
/// page. Never mutates anything — pure reporting.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ManagedAllocationStatus {
    #[schema(value_type = String)]
    pub server_uuid: Uuid,
    pub server_name: String,
    #[schema(value_type = String)]
    pub allocation_uuid: Uuid,
    pub ip: String,
    pub port: i32,
    /// `true` once the extension is confident the server is running (and not
    /// suspended) — i.e. the forward *should* be enabled.
    pub server_running: bool,
    /// Whether a matching pfSense rule currently exists at all.
    pub rule_exists: bool,
    /// Whether the existing pfSense rule is currently disabled.
    pub rule_disabled: Option<bool>,
    /// Whether the existing rule's target/port/disabled-state all match what
    /// reconcile currently wants. `false` just means the next reconcile pass
    /// (or the `InstallCompleted` hook) hasn't caught up yet — not an error.
    pub in_sync: bool,
    /// Set when a rule *not* managed by this extension already claims the
    /// same interface + external port this allocation wants — e.g. a
    /// hand-made forward for something unrelated to Calagopus. Reconcile
    /// refuses to create/replace this allocation's forward while this is
    /// set, rather than risk breaking whatever that other rule is for. The
    /// value is that rule's description, for tracking it down in pfSense.
    pub conflicting_rule: Option<String>,
}

/// Build a status snapshot for every currently in-use allocation. Does not
/// modify pfSense or the database.
pub async fn status(state: &State, config: &Config) -> anyhow::Result<Vec<ManagedAllocationStatus>> {
    let client = PfSenseClient::new(config)?;

    let desired = fetch_desired(state).await?;

    let current = client.list_port_forwards().await?;
    let mut managed_by_alloc: HashMap<Uuid, &crate::pfsense::PortForwardEntry> = HashMap::new();
    for entry in &current {
        if !entry.descr.starts_with(MANAGED_MARKER) {
            continue;
        }

        if let Some(alloc) = parse_alloc_uuid(&entry.descr) {
            // A duplicate here is a transient, already-handled reconcile
            // artifact (see `reconcile`'s dedup pass) — which one we show is
            // immaterial for a read-only status view.
            managed_by_alloc.insert(alloc, entry);
        }
    }

    // Kept whole (not consumed by the `.map()` below) so each entry can also
    // be checked against every other desired allocation for a cross-node
    // port collision — see `find_conflicting_sibling`.
    let all_desired = desired.clone();

    Ok(desired
        .into_iter()
        .map(|desired| {
            let entry = managed_by_alloc.get(&desired.allocation_uuid);
            let in_sync = entry.is_some_and(|entry| {
                entry.target == desired.ip
                    && entry.local_port == desired.port.to_string()
                    && entry.disabled == desired.disabled
                    && entry.interface == config.interface
                    && entry.destination == config.destination
            });
            let conflicting_rule =
                find_conflicting_unmanaged(&current, &config.interface, desired.port)
                    .map(|entry| entry.descr.clone())
                    .or_else(|| {
                        find_conflicting_sibling(&desired, &all_desired, &managed_by_alloc)
                            .map(|sibling| {
                                format!(
                                    "server \"{}\" on a different node/allocation wants the same port",
                                    sibling.server_name
                                )
                            })
                    });

            ManagedAllocationStatus {
                server_uuid: desired.server_uuid,
                server_name: desired.server_name,
                allocation_uuid: desired.allocation_uuid,
                ip: desired.ip,
                port: desired.port,
                server_running: !desired.disabled,
                rule_exists: entry.is_some(),
                rule_disabled: entry.map(|e| e.disabled),
                in_sync,
                conflicting_rule,
            }
        })
        .collect())
}

/// Find an existing rule — one this extension does *not* manage — that
/// already claims the same interface + external port a desired allocation
/// needs. Two pfSense NAT rules can't both meaningfully listen on the same
/// interface + destination port (only one ever actually takes effect), so
/// this is a hard conflict regardless of what the other rule is for. We
/// never touch or replace a rule we don't own, so the only thing we can do
/// here is refuse to create ours into the same slot.
fn find_conflicting_unmanaged<'a>(
    current: &'a [crate::pfsense::PortForwardEntry],
    interface: &str,
    port: i32,
) -> Option<&'a crate::pfsense::PortForwardEntry> {
    let port = port.to_string();
    current.iter().find(|entry| {
        !entry.descr.starts_with(MANAGED_MARKER)
            && entry.interface == interface
            && entry.destination_port == port
    })
}

/// Find another *desired* allocation — possibly on a completely different
/// node — that wants the same external port as `d`. `interface` and
/// `destination` are global settings on this extension, not per-node, so two
/// allocations sharing a port number collide at the NAT layer the moment
/// both need a forward, even though Calagopus itself never objects (its own
/// `node_allocations` uniqueness is scoped per node, so it has no reason to
/// stop you from doing this — see the discussion this was built for).
///
/// Returns `None` if nobody else wants this port, or if `d` is the one that
/// should get the rule: either it already holds a matching one, or (if
/// nobody involved has a rule yet) it wins a deterministic tie-break so we
/// don't flap between candidates on every pass. Otherwise returns the
/// sibling `d` has to defer to.
fn find_conflicting_sibling<'a>(
    d: &Desired,
    all_desired: &'a [Desired],
    current_by_alloc: &HashMap<Uuid, &crate::pfsense::PortForwardEntry>,
) -> Option<&'a Desired> {
    let has_matching_rule = |candidate: &Desired| {
        current_by_alloc.get(&candidate.allocation_uuid).is_some_and(|entry| {
            entry.target == candidate.ip && entry.local_port == candidate.port.to_string()
        })
    };

    let mut contenders: Vec<&Desired> = all_desired.iter().filter(|c| c.port == d.port).collect();
    if contenders.len() <= 1 {
        return None;
    }

    let winner = match contenders.iter().find(|c| has_matching_rule(c)) {
        Some(existing_holder) => *existing_holder,
        None => {
            contenders.sort_by_key(|c| c.allocation_uuid);
            contenders[0]
        }
    };

    if winner.allocation_uuid == d.allocation_uuid {
        None
    } else {
        Some(winner)
    }
}

/// Extract the `alloc=<uuid>` value from a managed rule's description.
fn parse_alloc_uuid(descr: &str) -> Option<Uuid> {
    let token = descr.split_whitespace().find_map(|t| t.strip_prefix("alloc="))?;
    Uuid::parse_str(token).ok()
}

/// Parse a pfSense object id string into a number for ordering (defaults to 0).
fn numeric_id(id: &str) -> i64 {
    id.parse::<i64>().unwrap_or(0)
}
