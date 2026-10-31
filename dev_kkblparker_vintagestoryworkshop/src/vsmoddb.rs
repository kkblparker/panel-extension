//! Client for the official Vintage Story Mod DB (<https://mods.vintagestory.at>), which has a
//! real, documented, unauthenticated JSON API - confirmed live, no HTML-scraping needed (unlike
//! the Arma Reforger Workshop extension's site).
//!
//! Confirmed live: `/api/mods` has no server-side pagination at all - an unfiltered call returns
//! the entire catalog (~9k mods) in one response, and the documented query params don't include a
//! page/limit. `search_mods` therefore fetches the full (filtered) result set and slices it to
//! the requested page itself.
//!
//! No dependency information is exposed anywhere in this API - a mod's actual dependencies live
//! inside its own `modinfo.json`, bundled inside the downloaded zip, not discoverable without
//! downloading and inspecting every candidate zip first. Out of scope here.

use std::sync::LazyLock;

const BASE_URL: &str = "https://mods.vintagestory.at/api";
const RESULTS_PER_PAGE: usize = 16;

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(format!(
            "github.com/calagopus/panel dev.kkblparker.vintagestoryworkshop {}",
            shared::VERSION
        ))
        .build()
        .expect("Failed to create HTTP client")
});

fn normalize_summary(mod_json: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "id": mod_json.get("modid"),
        "name": mod_json.get("name"),
        "summary": mod_json.get("summary"),
        "author": mod_json.get("author"),
        "imageUrl": mod_json.get("logo"),
        "tags": mod_json.get("tags"),
        "downloads": mod_json.get("downloads"),
        "side": mod_json.get("side"),
        "lastReleased": mod_json.get("lastreleased"),
        "workshopUrl": mod_json
            .get("urlalias")
            .and_then(|v| v.as_str())
            .map(|alias| format!("https://mods.vintagestory.at/show/mod/{alias}")),
    })
}

fn normalize_release(release: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "releaseId": release.get("releaseid"),
        "version": release.get("modversion"),
        "fileName": release.get("filename"),
        "downloadUrl": release.get("mainfile"),
        "gameVersions": release.get("tags"),
        "created": release.get("created"),
        "changelog": release.get("changelog"),
    })
}

fn normalize_detail(mod_json: &serde_json::Value) -> serde_json::Value {
    let mut summary = normalize_summary(mod_json);
    let releases: Vec<serde_json::Value> = mod_json
        .get("releases")
        .and_then(|r| r.as_array())
        .into_iter()
        .flatten()
        .map(normalize_release)
        .collect();

    if let Some(obj) = summary.as_object_mut() {
        obj.insert(
            "description".to_string(),
            mod_json.get("text").cloned().unwrap_or_default(),
        );
        obj.insert("releases".to_string(), serde_json::Value::Array(releases));
    }

    summary
}

pub struct SearchMods<'a> {
    pub search: Option<&'a str>,
    pub page: u32,
}

pub struct SearchResult {
    pub mods: Vec<serde_json::Value>,
    pub total: u64,
    pub per_page: u32,
}

pub async fn search_mods(params: SearchMods<'_>) -> Result<SearchResult, anyhow::Error> {
    let mut query: Vec<(&str, &str)> = Vec::new();
    if let Some(search) = params.search.filter(|s| !s.is_empty()) {
        query.push(("text", search));
    }

    let response = CLIENT.get(format!("{BASE_URL}/mods")).query(&query).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "the Vintage Story Mod DB responded with status {}",
            response.status()
        ));
    }

    let data: serde_json::Value = response.json().await?;
    let all_mods: Vec<&serde_json::Value> = data
        .get("mods")
        .and_then(|m| m.as_array())
        .into_iter()
        .flatten()
        .collect();

    let total = all_mods.len() as u64;
    let start = (params.page.saturating_sub(1) as usize) * RESULTS_PER_PAGE;
    let mods = all_mods
        .into_iter()
        .skip(start)
        .take(RESULTS_PER_PAGE)
        .map(normalize_summary)
        .collect();

    Ok(SearchResult {
        mods,
        total,
        per_page: RESULTS_PER_PAGE as u32,
    })
}

pub async fn get_mod(id: &str) -> Result<serde_json::Value, anyhow::Error> {
    let response = CLIENT.get(format!("{BASE_URL}/mod/{id}")).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "the Vintage Story Mod DB responded with status {}",
            response.status()
        ));
    }

    let data: serde_json::Value = response.json().await?;
    let mod_json = data
        .get("mod")
        .ok_or_else(|| anyhow::anyhow!("mod not found"))?;

    Ok(normalize_detail(mod_json))
}
