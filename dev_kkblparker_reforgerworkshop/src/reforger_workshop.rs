use std::sync::LazyLock;

const BASE_URL: &str = "https://api.reforgermods.net/v2";

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(format!(
            "github.com/calagopus/panel dev.kkblparker.reforgerworkshop {}",
            shared::VERSION
        ))
        .build()
        .expect("Failed to create HTTP client")
});

pub struct SearchMods<'a> {
    pub search: Option<&'a str>,
    pub tags: &'a [String],
    pub sort: Option<&'a str>,
    pub page: u32,
    pub per_page: Option<u32>,
}

pub async fn search_mods(params: SearchMods<'_>) -> Result<serde_json::Value, anyhow::Error> {
    let mut query: Vec<(&str, String)> = vec![("page", params.page.to_string())];

    if let Some(search) = params.search {
        query.push(("search", search.to_string()));
    }
    if let Some(sort) = params.sort {
        query.push(("sort", sort.to_string()));
    }
    if let Some(per_page) = params.per_page {
        query.push(("perPage", per_page.to_string()));
    }
    for tag in params.tags {
        query.push(("tags", tag.clone()));
    }

    let response = CLIENT
        .get(format!("{BASE_URL}/mods"))
        .query(&query)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "reforgermods.net responded with status {}",
            response.status()
        ));
    }

    Ok(response.json().await?)
}

pub async fn get_mod(id: &str) -> Result<serde_json::Value, anyhow::Error> {
    let response = CLIENT.get(format!("{BASE_URL}/mods/{id}")).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "reforgermods.net responded with status {}",
            response.status()
        ));
    }

    Ok(response.json().await?)
}
