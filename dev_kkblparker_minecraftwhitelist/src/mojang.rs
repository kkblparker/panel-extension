use shared::response::DisplayError;
use std::sync::LazyLock;

const BASE_URL: &str = "https://api.mojang.com";

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(format!(
            "github.com/calagopus/panel dev.kkblparker.minecraftwhitelist {}",
            shared::VERSION
        ))
        .build()
        .expect("Failed to create HTTP client")
});

pub struct MojangProfile {
    pub uuid: String,
    pub name: String,
}

/// Mojang returns UUIDs with no dashes; Minecraft's own whitelist.json/banned-players.json (and
/// the `whitelist`/`ban` commands' own output) use the standard dashed 8-4-4-4-12 form.
fn format_uuid(raw: &str) -> String {
    if raw.len() != 32 {
        return raw.to_string();
    }

    format!(
        "{}-{}-{}-{}-{}",
        &raw[0..8],
        &raw[8..12],
        &raw[12..16],
        &raw[16..20],
        &raw[20..32]
    )
}

pub async fn resolve_username(username: &str) -> Result<MojangProfile, anyhow::Error> {
    let response = CLIENT
        .get(format!("{BASE_URL}/users/profiles/minecraft/{username}"))
        .send()
        .await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(DisplayError::new(format!(
            "no Minecraft account named \"{username}\" was found"
        ))
        .into());
    }

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "mojang responded with status {}",
            response.status()
        ));
    }

    let body: serde_json::Value = response.json().await?;

    let raw_uuid = body
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DisplayError::new("mojang's response was missing a player id"))?;
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(username);

    Ok(MojangProfile {
        uuid: format_uuid(raw_uuid),
        name: name.to_string(),
    })
}
