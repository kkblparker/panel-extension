//! Database-backed settings for the pfSense Firewall Sync extension, editable
//! via its admin page (see [`crate::routes`]).
//!
//! Every field is optional: `None` means "not set via the admin page", in
//! which case [`crate::config::Config::resolve`] falls back to the matching
//! `PFSENSE_*` environment variable. This means an existing env-var-only
//! deployment keeps working untouched until someone actually saves the admin
//! page — at which point whatever they saved takes over for that field.

use shared::extensions::settings::{
    ExtensionSettings, SettingsDeserializeExt, SettingsDeserializer, SettingsSerializeExt,
    SettingsSerializer,
};

/// This extension's package name, used both as its settings key prefix and
/// to look its settings back up out of [`shared::settings::AppSettings`].
pub const PACKAGE_NAME: &str = "dev.kkblparker.pfsensesync";

#[derive(Debug, Clone, Default)]
pub struct PfSenseSettings {
    pub enabled: Option<bool>,
    pub base_url: Option<String>,
    /// Stored encrypted (see [`SettingsSerializer::write_raw_encrypted_setting`]).
    pub api_key: Option<String>,
    pub insecure_tls: Option<bool>,
    pub interface: Option<String>,
    pub protocol: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub associated_rule: Option<String>,
    pub interval_secs: Option<u64>,
    /// When enabled, admin API routes return the real pfSense/reqwest error
    /// text on failure instead of a generic message — without needing the
    /// panel-wide `APP_DEBUG` flag (and a full restart) just to see why a
    /// pfSense call is failing.
    pub debug: Option<bool>,
}

#[async_trait::async_trait]
impl SettingsSerializeExt for PfSenseSettings {
    async fn serialize(
        &self,
        mut serializer: SettingsSerializer,
    ) -> Result<SettingsSerializer, anyhow::Error> {
        serializer = serializer
            .write_serde_setting("enabled", &self.enabled)?
            .write_serde_setting("base_url", &self.base_url)?
            .write_serde_setting("insecure_tls", &self.insecure_tls)?
            .write_serde_setting("interface", &self.interface)?
            .write_serde_setting("protocol", &self.protocol)?
            .write_serde_setting("source", &self.source)?
            .write_serde_setting("destination", &self.destination)?
            .write_serde_setting("associated_rule", &self.associated_rule)?
            .write_serde_setting("interval_secs", &self.interval_secs)?
            .write_serde_setting("debug", &self.debug)?;

        serializer = serializer
            .write_raw_encrypted_setting("api_key", self.api_key.as_deref().unwrap_or(""))
            .await?;

        Ok(serializer)
    }
}

pub struct PfSenseSettingsDeserializer;

#[async_trait::async_trait]
impl SettingsDeserializeExt for PfSenseSettingsDeserializer {
    async fn deserialize_boxed(
        &self,
        deserializer: SettingsDeserializer<'_>,
    ) -> Result<ExtensionSettings, anyhow::Error> {
        let api_key = deserializer
            .read_raw_encrypted_setting("api_key")
            .await?
            .filter(|key| !key.is_empty())
            .map(|key| key.to_string());

        Ok(Box::new(PfSenseSettings {
            enabled: deserializer.read_serde_setting("enabled")?,
            base_url: deserializer.read_serde_setting("base_url")?,
            api_key,
            insecure_tls: deserializer.read_serde_setting("insecure_tls")?,
            interface: deserializer.read_serde_setting("interface")?,
            protocol: deserializer.read_serde_setting("protocol")?,
            source: deserializer.read_serde_setting("source")?,
            destination: deserializer.read_serde_setting("destination")?,
            associated_rule: deserializer.read_serde_setting("associated_rule")?,
            interval_secs: deserializer.read_serde_setting("interval_secs")?,
            debug: deserializer.read_serde_setting("debug")?,
        }))
    }
}
