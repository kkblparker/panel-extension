//! Configuration for the Steam Workshop extension. The API key can be set via
//! the `STEAM_API_KEY` environment variable or the admin settings page - the
//! admin page takes precedence when set, mirroring the pattern used by the
//! pfSense Firewall Sync extension's config.

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub api_key: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("STEAM_API_KEY").unwrap_or_default(),
        }
    }

    pub fn resolve(settings: &crate::settings::SteamWorkshopSettings) -> Self {
        let env = Self::from_env();

        Self {
            api_key: settings
                .api_key
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
                .unwrap_or(env.api_key),
        }
    }

    pub async fn load(state: &shared::State) -> anyhow::Result<Self> {
        let app_settings = state.settings.get().await?;
        let settings = app_settings
            .get_extension_settings::<crate::settings::SteamWorkshopSettings>(
                crate::settings::PACKAGE_NAME,
            )?;

        Ok(Self::resolve(settings))
    }
}
