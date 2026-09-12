//! Database-backed settings for the Steam Workshop extension, editable via
//! its admin page (see [`crate::routes::settings`]).
//!
//! A Steam Web API key is needed for `IPublishedFileService/QueryFiles`
//! (searching the Workshop) - unlike `GetPublishedFileDetails` (fetching a
//! known item's details), which is keyless. Get one free at
//! https://steamcommunity.com/dev/apikey - it's a standard key, not a
//! restricted publisher key.

use shared::extensions::settings::{
    ExtensionSettings, SettingsDeserializeExt, SettingsDeserializer, SettingsSerializeExt,
    SettingsSerializer,
};

pub const PACKAGE_NAME: &str = "dev.kkblparker.steamworkshop";

#[derive(Debug, Clone, Default)]
pub struct SteamWorkshopSettings {
    /// Stored encrypted (see [`SettingsSerializer::write_raw_encrypted_setting`]).
    pub api_key: Option<String>,
}

#[async_trait::async_trait]
impl SettingsSerializeExt for SteamWorkshopSettings {
    async fn serialize(
        &self,
        mut serializer: SettingsSerializer,
    ) -> Result<SettingsSerializer, anyhow::Error> {
        serializer = serializer
            .write_raw_encrypted_setting("api_key", self.api_key.as_deref().unwrap_or(""))
            .await?;

        Ok(serializer)
    }
}

pub struct SteamWorkshopSettingsDeserializer;

#[async_trait::async_trait]
impl SettingsDeserializeExt for SteamWorkshopSettingsDeserializer {
    async fn deserialize_boxed(
        &self,
        deserializer: SettingsDeserializer<'_>,
    ) -> Result<ExtensionSettings, anyhow::Error> {
        let api_key = deserializer
            .read_raw_encrypted_setting("api_key")
            .await?
            .filter(|key| !key.is_empty())
            .map(|key| key.to_string());

        Ok(Box::new(SteamWorkshopSettings { api_key }))
    }
}
