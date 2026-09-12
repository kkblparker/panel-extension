//! Thin client for the pfSense REST API package (pfSense-pkg-RESTAPI, v2).
//!
//! Only the endpoints needed for NAT port-forward reconciliation (and the
//! admin settings page's interface/destination pickers) are wrapped:
//!   * `GET    /api/v2/firewall/nat/port_forwards` — list all port forwards
//!   * `POST   /api/v2/firewall/nat/port_forward`  — create one
//!   * `PATCH  /api/v2/firewall/nat/port_forward`  — update one (we only ever flip `disabled`)
//!   * `DELETE /api/v2/firewall/nat/port_forward`  — delete one (by `id`)
//!   * `POST   /api/v2/firewall/apply`             — apply pending firewall changes
//!   * `GET    /api/v2/interfaces`                 — list interfaces, for the settings page dropdown
//!   * `GET    /api/v2/firewall/virtual_ips`       — list additional WAN IPs (CARP/IP-alias/proxy-ARP),
//!     for setups with more than one public IP
//!
//! Authentication uses the API key scheme (`X-API-Key` header).
//!
//! [`Self::list_interfaces`] and [`Self::list_virtual_ips`] field names are
//! cross-checked against the `NetworkInterface` and `VirtualIP` model
//! definitions in the pfrest/pfSense-pkg-RESTAPI source (the `id`/`descr` and
//! `subnet`/`interface`/`descr`/`mode` fields respectively), not against a
//! live pfSense box — a version skew is still possible, but this isn't a
//! blind guess.

use anyhow::Context;
use std::time::Duration;

use crate::config::Config;

/// A port-forward as returned by the pfSense API, reduced to the fields we care
/// about for reconciliation.
#[derive(Debug, Clone)]
pub struct PortForwardEntry {
    /// The pfSense object id (array index). Used for DELETE/PATCH. Stored as a
    /// string because that is how it is passed back as a query parameter.
    pub id: String,
    /// The `descr` field — we use its marker prefix to detect rules we own.
    pub descr: String,
    /// The interface this forward is attached to.
    pub interface: String,
    /// The external port traffic arrives on. Combined with `interface`, this
    /// is what actually determines whether two rules conflict on pfSense —
    /// two rules can share a `local_port`/target without issue, but not the
    /// same `interface` + `destination_port`.
    pub destination_port: String,
    /// The destination address this forward matches against (e.g. `wan:ip`,
    /// `any`, a literal IP). Compared against the current config's
    /// `destination` so a global setting change (e.g. fixing a bad default)
    /// actually gets applied to already-existing rules on the next
    /// reconcile pass, instead of silently only affecting new ones.
    pub destination: String,
    /// The forward target (internal host IP).
    pub target: String,
    /// The internal `local-port`.
    pub local_port: String,
    /// Whether the rule is currently disabled on the firewall. We toggle this
    /// (rather than deleting/recreating) when a server's power state changes,
    /// so a stopped server's port is closed without losing the rule.
    pub disabled: bool,
}

/// An available pfSense interface, for the admin settings page's "WAN
/// interface" dropdown.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct InterfaceEntry {
    /// The short identifier pfSense uses internally (`wan`, `opt1`, ...) —
    /// this is the value that belongs in the `interface` setting.
    pub id: String,
    /// A human-friendly label for the dropdown, if pfSense reports one.
    pub description: String,
    /// This interface's current IPv4 address, if it has a static/DHCP one
    /// assigned. Use *this* (not `id`) as a `destination` value when you
    /// want to match only this interface's own address — passing the bare
    /// interface name to `destination` resolves to "<interface> subnets"
    /// (its whole attached network), which is too broad for a NAT
    /// destination. `None` for interfaces without an assigned address (e.g.
    /// still negotiating DHCP) — the address can also change on a dynamic
    /// WAN, so treat this as a snapshot, not a stable identifier.
    pub address: Option<String>,
}

/// A configured virtual IP (an additional public IP beyond an interface's
/// primary address — CARP, IP-alias, or proxy-ARP), for setups with more
/// than one WAN IP. For the admin settings page's "destination" dropdown.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct VirtualIpEntry {
    /// The actual address — this is the value that belongs in the
    /// `destination` setting.
    pub address: String,
    /// The interface this virtual IP is bound to (`wan`, `opt1`, ...).
    pub interface: String,
    /// A human-friendly label for the dropdown, if pfSense reports one.
    pub description: String,
    /// `ipalias`, `proxyarp`, `carp`, or `other`.
    pub mode: String,
}

pub struct PfSenseClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl PfSenseClient {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(config.insecure_tls)
            .timeout(Duration::from_secs(20))
            .build()
            .context("failed to build reqwest client for pfSense")?;

        Ok(Self {
            http,
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Fetch every port-forward rule on the firewall.
    pub async fn list_port_forwards(&self) -> anyhow::Result<Vec<PortForwardEntry>> {
        // `limit=0` tells the API to return all objects rather than a page.
        // Note the *plural* endpoint returns the full collection.
        let resp = self
            .http
            .get(self.url("/api/v2/firewall/nat/port_forwards"))
            .header("X-API-Key", &self.api_key)
            .query(&[("limit", "0")])
            .send()
            .await
            .context("request to list pfSense port forwards failed")?;

        let body = read_json(resp, "list port forwards").await?;

        let data = body
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();

        let mut entries = Vec::with_capacity(data.len());
        for item in data {
            // `id` may come back as a number or a string depending on version.
            let id = match item.get("id") {
                Some(serde_json::Value::Number(n)) => n.to_string(),
                Some(serde_json::Value::String(s)) => s.clone(),
                _ => continue, // an entry we cannot address is not useful to us
            };

            entries.push(PortForwardEntry {
                id,
                descr: item
                    .get("descr")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                interface: item
                    .get("interface")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                destination_port: json_to_port_string(item.get("destination_port")),
                destination: item
                    .get("destination")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                target: item
                    .get("target")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                local_port: json_to_port_string(item.get("local_port")),
                disabled: item.get("disabled").is_some_and(json_to_bool),
            });
        }

        Ok(entries)
    }

    /// Create a port-forward. `apply` is deferred — call [`Self::apply`] once
    /// after a batch of changes so the ruleset only reloads a single time.
    /// `disabled` should mirror whether the target server is currently
    /// running, so a forward for a stopped server is created closed.
    pub async fn create_port_forward(
        &self,
        config: &Config,
        descr: &str,
        target_ip: &str,
        port: i32,
        disabled: bool,
    ) -> anyhow::Result<()> {
        let port = port.to_string();
        let body = serde_json::json!({
            "interface": config.interface,
            "ipprotocol": "inet",
            "protocol": config.protocol,
            "source": config.source,
            "destination": config.destination,
            "destination_port": port,
            "target": target_ip,
            "local_port": port,
            "descr": descr,
            "associated_rule_id": config.associated_rule,
            "disabled": disabled,
            "apply": false
        });

        let resp = self
            .http
            .post(self.url("/api/v2/firewall/nat/port_forward"))
            .header("X-API-Key", &self.api_key)
            .json(&body)
            .send()
            .await
            .context("request to create pfSense port forward failed")?;

        read_json(resp, "create port forward").await?;
        Ok(())
    }

    /// Enable or disable an existing port-forward in place, without touching
    /// any other field. This is how the reconcile loop takes a managed
    /// forward down/up as its server stops/starts — a PATCH avoids the
    /// delete+recreate churn (and the id-shifting hazard described on
    /// [`Self::delete_port_forward`]) that a full replace would need.
    pub async fn set_port_forward_disabled(&self, id: &str, disabled: bool) -> anyhow::Result<()> {
        let resp = self
            .http
            .patch(self.url("/api/v2/firewall/nat/port_forward"))
            .header("X-API-Key", &self.api_key)
            .json(&serde_json::json!({
                "id": id,
                "disabled": disabled,
                "apply": false
            }))
            .send()
            .await
            .context("request to update pfSense port forward failed")?;

        read_json(resp, "update port forward disabled state").await?;
        Ok(())
    }

    /// Delete a port-forward by its pfSense object id.
    pub async fn delete_port_forward(&self, id: &str) -> anyhow::Result<()> {
        let resp = self
            .http
            .delete(self.url("/api/v2/firewall/nat/port_forward"))
            .header("X-API-Key", &self.api_key)
            .query(&[("id", id), ("apply", "false")])
            .send()
            .await
            .context("request to delete pfSense port forward failed")?;

        read_json(resp, "delete port forward").await?;
        Ok(())
    }

    /// List every interface pfSense knows about, for the admin settings
    /// page's interface picker. See the module-level note about this
    /// endpoint's schema being unverified against a live pfSense box.
    pub async fn list_interfaces(&self) -> anyhow::Result<Vec<InterfaceEntry>> {
        let resp = self
            .http
            .get(self.url("/api/v2/interfaces"))
            .header("X-API-Key", &self.api_key)
            .query(&[("limit", "0")])
            .send()
            .await
            .context("request to list pfSense interfaces failed")?;

        let body = read_json(resp, "list interfaces").await?;

        let data = body
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();

        let mut entries = Vec::with_capacity(data.len());
        for item in data {
            let id = match item.get("id").and_then(|v| v.as_str()) {
                Some(id) if !id.is_empty() => id.to_string(),
                _ => continue, // an interface we cannot address by id is not useful to us
            };

            let description = item
                .get("descr")
                .and_then(|v| v.as_str())
                .filter(|d| !d.is_empty())
                .unwrap_or(&id)
                .to_string();

            let address = item
                .get("ipaddr")
                .and_then(|v| v.as_str())
                .filter(|v| !v.is_empty())
                .map(str::to_string);

            entries.push(InterfaceEntry {
                id,
                description,
                address,
            });
        }

        Ok(entries)
    }

    /// List every configured virtual IP, for the admin settings page's
    /// destination picker (setups with more than one WAN IP).
    pub async fn list_virtual_ips(&self) -> anyhow::Result<Vec<VirtualIpEntry>> {
        let resp = self
            .http
            .get(self.url("/api/v2/firewall/virtual_ips"))
            .header("X-API-Key", &self.api_key)
            .query(&[("limit", "0")])
            .send()
            .await
            .context("request to list pfSense virtual IPs failed")?;

        let body = read_json(resp, "list virtual IPs").await?;

        let data = body
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();

        let mut entries = Vec::with_capacity(data.len());
        for item in data {
            let address = match item.get("subnet").and_then(|v| v.as_str()) {
                Some(addr) if !addr.is_empty() => addr.to_string(),
                _ => continue, // a virtual IP we cannot address is not useful to us
            };

            let interface = item
                .get("interface")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let description = item
                .get("descr")
                .and_then(|v| v.as_str())
                .filter(|d| !d.is_empty())
                .unwrap_or(&address)
                .to_string();

            let mode = item
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            entries.push(VirtualIpEntry {
                address,
                interface,
                description,
                mode,
            });
        }

        Ok(entries)
    }

    /// Apply any pending firewall changes (reloads the ruleset).
    pub async fn apply(&self) -> anyhow::Result<()> {
        let resp = self
            .http
            .post(self.url("/api/v2/firewall/apply"))
            .header("X-API-Key", &self.api_key)
            .json(&serde_json::json!({}))
            .send()
            .await
            .context("request to apply pfSense firewall changes failed")?;

        read_json(resp, "apply firewall changes").await?;
        Ok(())
    }
}

/// Read a response body as JSON, turning a non-2xx status into a descriptive
/// error that includes the pfSense response body (which carries a useful
/// `message` / `response_id`).
async fn read_json(
    resp: reqwest::Response,
    action: &str,
) -> anyhow::Result<serde_json::Value> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .with_context(|| format!("failed to read pfSense response body while trying to {action}"))?;

    if !status.is_success() {
        anyhow::bail!("pfSense returned {status} while trying to {action}: {text}");
    }

    if text.trim().is_empty() {
        return Ok(serde_json::Value::Null);
    }

    serde_json::from_str(&text)
        .with_context(|| format!("failed to parse pfSense JSON response while trying to {action}"))
}

/// Normalise a port value that may be a JSON number or string into a string.
fn json_to_port_string(value: Option<&serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

/// Normalise a "boolish" value (pfSense has returned these as JSON booleans,
/// `"1"`/`""` strings, and numbers across versions) into a bool.
fn json_to_bool(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::String(s) => matches!(s.as_str(), "1" | "true" | "yes"),
        serde_json::Value::Number(n) => n.as_i64().is_some_and(|v| v != 0),
        _ => false,
    }
}
