use std::sync::LazyLock;

const BASE_URL: &str = "https://api.steampowered.com";

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(format!(
            "github.com/calagopus/panel dev.kkblparker.steamworkshop {}",
            shared::VERSION
        ))
        .build()
        .expect("Failed to create HTTP client")
});

pub struct SearchWorkshop<'a> {
    pub api_key: &'a str,
    pub appid: &'a str,
    pub query: Option<&'a str>,
    pub sort: Option<&'a str>,
    pub page: u32,
}

/// Search the Workshop for `appid` - requires a Steam Web API key
/// (`IPublishedFileService/QueryFiles`). Unlike `get_item_details`, this
/// endpoint is NOT keyless.
pub async fn search_workshop(
    params: SearchWorkshop<'_>,
) -> Result<serde_json::Value, anyhow::Error> {
    if params.api_key.is_empty() {
        return Err(anyhow::anyhow!(
            "no Steam Web API key configured - set one on this extension's admin settings page or the STEAM_API_KEY environment variable"
        ));
    }

    let has_search_text = params.query.is_some_and(|search| !search.is_empty());

    // k_PublishedFileQueryType_RankedByTextSearch (9) is what actually ranks by relevance to
    // `search_text` - Steam ignores any other query_type's ordering once search text is present,
    // so the sort dropdown only applies to plain browsing (no search text typed).
    let query_type = if has_search_text {
        "9"
    } else {
        match params.sort {
            Some("newest") => "1", // k_PublishedFileQueryType_RankedByPublicationDate
            Some("recently-updated") => "20", // k_PublishedFileQueryType_RankedByLastUpdatedDate
            Some("trending") => "3", // k_PublishedFileQueryType_RankedByTrend
            _ => "10", // k_PublishedFileQueryType_RankedByTotalUniqueSubscriptions ("Most Popular")
        }
    };

    let mut query: Vec<(&str, String)> = vec![
        ("key", params.api_key.to_string()),
        ("query_type", query_type.to_string()),
        ("appid", params.appid.to_string()),
        ("page", params.page.max(1).to_string()),
        ("numperpage", "20".to_string()),
        ("return_short_description", "true".to_string()),
        ("return_previews", "true".to_string()),
        ("return_vote_data", "true".to_string()),
        ("return_tags", "true".to_string()),
    ];

    if has_search_text {
        query.push(("search_text", params.query.unwrap().to_string()));
    }

    let response = CLIENT
        .get(format!("{BASE_URL}/IPublishedFileService/QueryFiles/v1/"))
        .query(&query)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "steam web api responded with status {}",
            response.status()
        ));
    }

    Ok(response.json().await?)
}

/// Fetch details for one or more published file IDs. Keyless - works without
/// any Steam Web API key.
pub async fn get_item_details(ids: &[String]) -> Result<serde_json::Value, anyhow::Error> {
    if ids.is_empty() {
        return Ok(serde_json::json!({ "response": { "publishedfiledetails": [] } }));
    }

    let mut form: Vec<(String, String)> = vec![("itemcount".to_string(), ids.len().to_string())];
    for (i, id) in ids.iter().enumerate() {
        form.push((format!("publishedfileids[{i}]"), id.clone()));
    }

    let response = CLIENT
        .post(format!(
            "{BASE_URL}/ISteamRemoteStorage/GetPublishedFileDetails/v1/"
        ))
        .form(&form)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "steam web api responded with status {}",
            response.status()
        ));
    }

    Ok(response.json().await?)
}
