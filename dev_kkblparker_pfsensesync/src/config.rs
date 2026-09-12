//! Configuration for the pfSense firewall sync extension.
//!
//! Every field can be set two ways: a `PFSENSE_*` environment variable (see
//! [`Config::from_env`]), or the extension's admin settings page, persisted as
//! [`crate::settings::PfSenseSettings`] in the database. [`Config::resolve`]
//! merges the two — a field left unset on the admin page falls back to its
//! environment variable (or built-in default) — so an env-var-only deployment
//! keeps working untouched until someone actually saves the admin page.
//! [`Config::load`] does this fresh from the database on every call, so
//! changes made through the admin page take effect on the very next reconcile
//! pass with no restart needed.

use std::time::Duration;

/// The marker prefix written into the `descr` field of every port-forward this
/// extension manages. The reconcile loop only ever touches rules whose
/// description starts with this marker, so it can never disturb port-forwards
/// created by hand in the pfSense UI.
pub const MANAGED_MARKER: &str = "[calagopus-pf-sync]";

/// Minimum interval we allow, to avoid hammering the firewall API.
const MIN_INTERVAL_SECS: u64 = 10;
/// Default reconcile interval when `PFSENSE_SYNC_INTERVAL` is unset. Kept
/// short because this is also the fallback path for anything the
/// `InstallCompleted` event hook doesn't cover (extra allocations added to an
/// already-running server, power-state changes, manual pfSense edits).
const DEFAULT_INTERVAL_SECS: u64 = 15;

#[derive(Clone, Debug)]
pub struct Config {
    /// Whether the sync loop should run at all.
    pub enabled: bool,
    /// Base URL of the pfSense box, e.g. `https://192.168.1.1`. No trailing slash.
    pub base_url: String,
    /// pfSense REST API key (sent via the `X-API-Key` header).
    pub api_key: String,
    /// Accept self-signed / invalid TLS certificates (common on a LAN pfSense box).
    pub insecure_tls: bool,
    /// The pfSense interface the port-forwards are attached to (e.g. `wan`).
    pub interface: String,
    /// Transport protocol for the forwards. Defaults to `tcp/udp` which suits
    /// most game servers.
    pub protocol: String,
    /// Source address filter. `any` by default.
    pub source: String,
    /// Destination address for the forward. Defaults to `"{interface}:ip"`
    /// (e.g. `wan:ip`) — pfSense-pkg-RESTAPI's `FilterAddressField` resolves
    /// this to that interface's *current* IP address, dynamically, on every
    /// apply, so it survives a DHCP WAN IP change. The bare interface name
    /// alone (no `:ip`) means something different and much broader — "this
    /// interface's entire attached subnet" — and the legacy `wanip` keyword
    /// some other pfSense API tooling uses is rejected outright (confirmed
    /// against live 400 responses for both). Set this to whatever else your
    /// pfSense box accepts: an IP, a subnet CIDR, an alias, `any`, `(self)`,
    /// `l2tp`, `pppoe`.
    pub destination: String,
    /// The `associated_rule_id` mode. `pass` (default) makes the forward pass
    /// traffic automatically without a separate firewall rule. Use `new` to
    /// have pfSense create a linked pass rule, or an empty string to require
    /// you to add the firewall rule yourself.
    pub associated_rule: String,
    /// How often to reconcile.
    pub interval: Duration,
    /// When enabled, admin API routes return the real pfSense/reqwest error
    /// text on failure instead of a generic message. Scoped to this
    /// extension so you don't need the panel-wide `APP_DEBUG` flag (and a
    /// full panel restart) just to see why a pfSense call is failing.
    pub debug: bool,
}

impl Config {
    /// Build a [`Config`] from the environment. Returns `None` only if a hard
    /// parse error happens; missing optional values fall back to defaults.
    ///
    /// Note this never *fails* on a disabled/misconfigured extension — the
    /// `enabled` flag and a later validation step decide whether we actually
    /// try to talk to pfSense. This keeps the panel booting even if the
    /// extension is half-configured.
    pub fn from_env() -> Self {
        let enabled = env_bool("PFSENSE_SYNC_ENABLED", false);
        let base_url = env_string("PFSENSE_API_URL", "")
            .trim_end_matches('/')
            .to_string();
        let api_key = env_string("PFSENSE_API_KEY", "");
        let insecure_tls = env_bool("PFSENSE_INSECURE_TLS", false);
        let interface = env_string("PFSENSE_WAN_INTERFACE", "wan");
        let protocol = env_string("PFSENSE_PROTOCOL", "tcp/udp");
        let source = env_string("PFSENSE_SOURCE", "any");
        // NOT the bare interface name: pfSense resolves that to "<interface>
        // subnets" (the whole attached network), which is too broad for a NAT
        // destination and actively wrong for a multi-WAN-IP setup. The `:ip`
        // suffix (confirmed against the pfSense-pkg-RESTAPI `FilterAddressField`
        // source) tells pfSense to resolve this interface's current IP address
        // instead of its subnet — dynamically, on every apply, so it keeps
        // working through a DHCP WAN IP change without us ever fetching or
        // caching the address ourselves.
        let destination = env_string("PFSENSE_DESTINATION", &format!("{interface}:ip"));
        let associated_rule = env_string("PFSENSE_ASSOCIATED_RULE", "pass");
        let debug = env_bool("PFSENSE_DEBUG", false);

        let interval_secs = env_string("PFSENSE_SYNC_INTERVAL", "")
            .parse::<u64>()
            .unwrap_or(DEFAULT_INTERVAL_SECS)
            .max(MIN_INTERVAL_SECS);

        Self {
            enabled,
            base_url,
            api_key,
            insecure_tls,
            interface,
            protocol,
            source,
            destination,
            associated_rule,
            interval: Duration::from_secs(interval_secs),
            debug,
        }
    }

    /// Returns `true` if the config has the minimum required values to actually
    /// talk to pfSense. Used to short-circuit (and warn) rather than spamming
    /// connection errors on every reconcile tick.
    pub fn is_operational(&self) -> bool {
        self.enabled && !self.base_url.is_empty() && !self.api_key.is_empty()
    }

    /// A human-readable reason the config is not operational, for logging.
    pub fn inoperational_reason(&self) -> Option<&'static str> {
        if !self.enabled {
            Some("PFSENSE_SYNC_ENABLED is not set to a truthy value")
        } else if self.base_url.is_empty() {
            Some("PFSENSE_API_URL is empty")
        } else if self.api_key.is_empty() {
            Some("PFSENSE_API_KEY is empty")
        } else {
            None
        }
    }

    /// Merge environment-variable defaults with any overrides saved via the
    /// admin settings page. A `None` field in `settings` falls back to the
    /// matching environment variable (or its built-in default).
    pub fn resolve(settings: &crate::settings::PfSenseSettings) -> Self {
        let env = Self::from_env();

        fn non_empty(v: &Option<String>) -> Option<&str> {
            v.as_deref().map(str::trim).filter(|v| !v.is_empty())
        }

        let interval_secs = settings
            .interval_secs
            .unwrap_or_else(|| env.interval.as_secs())
            .max(MIN_INTERVAL_SECS);

        let interface = non_empty(&settings.interface)
            .map(str::to_string)
            .unwrap_or(env.interface);
        // Falls back to *this* resolved interface's `:ip` (see `from_env`'s
        // comment) — not `env.destination` — so changing the interface on
        // the admin page without touching destination keeps them in sync,
        // rather than pinning destination to whatever the env-var default was.
        let destination = non_empty(&settings.destination)
            .map(str::to_string)
            .unwrap_or_else(|| format!("{interface}:ip"));

        Self {
            enabled: settings.enabled.unwrap_or(env.enabled),
            base_url: non_empty(&settings.base_url)
                .map(|v| v.trim_end_matches('/').to_string())
                .unwrap_or(env.base_url),
            api_key: non_empty(&settings.api_key)
                .map(str::to_string)
                .unwrap_or(env.api_key),
            insecure_tls: settings.insecure_tls.unwrap_or(env.insecure_tls),
            interface,
            protocol: non_empty(&settings.protocol)
                .map(str::to_string)
                .unwrap_or(env.protocol),
            source: non_empty(&settings.source)
                .map(str::to_string)
                .unwrap_or(env.source),
            destination,
            associated_rule: non_empty(&settings.associated_rule)
                .map(str::to_string)
                .unwrap_or(env.associated_rule),
            interval: Duration::from_secs(interval_secs),
            debug: settings.debug.unwrap_or(env.debug),
        }
    }

    /// Load the effective config fresh from the database, merging admin
    /// settings over environment defaults (see [`Self::resolve`]).
    pub async fn load(state: &shared::State) -> anyhow::Result<Self> {
        let app_settings = state.settings.get().await?;
        let settings =
            app_settings.get_extension_settings::<crate::settings::PfSenseSettings>(
                crate::settings::PACKAGE_NAME,
            )?;

        Ok(Self::resolve(settings))
    }
}

fn env_string(key: &str, default: &str) -> String {
    match std::env::var(key) {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => default.to_string(),
    }
}

fn env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => default,
    }
}
