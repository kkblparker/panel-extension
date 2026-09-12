use std::sync::LazyLock;

const BASE_URL: &str = "https://api.modrinth.com/v2";

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(format!(
            "github.com/calagopus/panel dev.kkblparker.minecraftworkshop {}",
            shared::VERSION
        ))
        .build()
        .expect("Failed to create HTTP client")
});

pub struct SearchProjects<'a> {
    pub query: Option<&'a str>,
    pub project_type: &'a str,
    pub loader: &'a str,
    pub sort: Option<&'a str>,
    pub page: u32,
    pub per_page: u32,
}

pub async fn search_projects(
    params: SearchProjects<'_>,
) -> Result<serde_json::Value, anyhow::Error> {
    let facets = serde_json::to_string(&vec![
        vec![format!("project_type:{}", params.project_type)],
        vec![format!("categories:{}", params.loader)],
    ])?;

    let offset = params.page.saturating_sub(1) * params.per_page;

    let mut query: Vec<(&str, String)> = vec![
        ("facets", facets),
        ("offset", offset.to_string()),
        ("limit", params.per_page.to_string()),
    ];

    if let Some(search) = params.query {
        query.push(("query", search.to_string()));
    }
    if let Some(sort) = params.sort {
        query.push(("index", sort.to_string()));
    }

    let response = CLIENT
        .get(format!("{BASE_URL}/search"))
        .query(&query)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "modrinth responded with status {}",
            response.status()
        ));
    }

    Ok(response.json().await?)
}

pub async fn get_project(id: &str) -> Result<serde_json::Value, anyhow::Error> {
    let response = CLIENT
        .get(format!("{BASE_URL}/project/{id}"))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "modrinth responded with status {}",
            response.status()
        ));
    }

    Ok(response.json().await?)
}

pub struct ResolvedFile {
    pub url: String,
    pub filename: String,
    pub version_id: String,
}

/// Finds the newest version of `id` compatible with `loader` + `mc_version`, and returns its
/// primary file (falling back to the first file if none is marked primary).
pub async fn get_best_version_file(
    id: &str,
    loader: &str,
    mc_version: &str,
) -> Result<Option<ResolvedFile>, anyhow::Error> {
    let response = CLIENT
        .get(format!("{BASE_URL}/project/{id}/version"))
        .query(&[
            ("loaders", serde_json::to_string(&[loader])?),
            ("game_versions", serde_json::to_string(&[mc_version])?),
        ])
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "modrinth responded with status {}",
            response.status()
        ));
    }

    let versions: Vec<serde_json::Value> = response.json().await?;
    let Some(version) = versions.first() else {
        return Ok(None);
    };

    let version_id = version
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let files = version.get("files").and_then(|f| f.as_array());
    let Some(files) = files else {
        return Ok(None);
    };

    let file = files
        .iter()
        .find(|f| f.get("primary").and_then(|p| p.as_bool()).unwrap_or(false))
        .or_else(|| files.first());

    Ok(file.and_then(|file| {
        Some(ResolvedFile {
            url: file.get("url")?.as_str()?.to_string(),
            filename: file.get("filename")?.as_str()?.to_string(),
            version_id,
        })
    }))
}
