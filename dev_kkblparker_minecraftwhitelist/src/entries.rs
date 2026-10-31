use serde::Serialize;
use utoipa::ToSchema;

#[derive(ToSchema, Serialize, Clone)]
pub(crate) struct WhitelistEntry {
    pub uuid: String,
    pub name: String,
}

pub(crate) fn parse_whitelist(raw: Vec<serde_json::Value>) -> Vec<WhitelistEntry> {
    raw.into_iter()
        .filter_map(|v| {
            Some(WhitelistEntry {
                uuid: v.get("uuid")?.as_str()?.to_string(),
                name: v
                    .get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
            })
        })
        .collect()
}

#[derive(ToSchema, Serialize, Clone)]
pub(crate) struct BanEntry {
    pub uuid: String,
    pub name: String,
    pub created: Option<String>,
    pub source: Option<String>,
    pub expires: Option<String>,
    pub reason: Option<String>,
}

pub(crate) fn parse_bans(raw: Vec<serde_json::Value>) -> Vec<BanEntry> {
    raw.into_iter()
        .filter_map(|v| {
            Some(BanEntry {
                uuid: v.get("uuid")?.as_str()?.to_string(),
                name: v
                    .get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                created: v.get("created").and_then(|x| x.as_str()).map(str::to_string),
                source: v.get("source").and_then(|x| x.as_str()).map(str::to_string),
                expires: v.get("expires").and_then(|x| x.as_str()).map(str::to_string),
                reason: v.get("reason").and_then(|x| x.as_str()).map(str::to_string),
            })
        })
        .collect()
}
