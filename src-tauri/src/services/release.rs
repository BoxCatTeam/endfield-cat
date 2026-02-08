use reqwest::StatusCode;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct LatestRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub html_url: Option<String>,
    pub download_url: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug)]
struct FetchReleaseError {
    message: String,
    status: Option<StatusCode>,
}

fn latest_release_from_json(json: &serde_json::Value) -> Result<LatestRelease, String> {
    let tag_name = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            json.get("name")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
        })
        .ok_or("Missing tag_name in GitHub response")?
        .to_string();

    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let html_url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let body = json
        .get("body")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let download_url = if cfg!(target_os = "windows") {
        json.get("assets")
            .and_then(|v| v.as_array())
            .and_then(|assets| {
                assets.iter().find_map(|asset| {
                    let name = asset.get("name").and_then(|v| v.as_str())?;
                    if name.ends_with(".exe") {
                        asset
                            .get("browser_download_url")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    } else {
                        None
                    }
                })
            })
    } else {
        None
    };

    Ok(LatestRelease {
        tag_name,
        name,
        html_url,
        download_url,
        body,
    })
}

pub async fn fetch_latest_release(client: &reqwest::Client) -> Result<LatestRelease, String> {
    async fn fetch(
        client: &reqwest::Client,
        url: &str,
    ) -> Result<LatestRelease, FetchReleaseError> {
        let resp = client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "endfield-cat/tauri")
            .send()
            .await
            .map_err(|e| FetchReleaseError {
                message: e.to_string(),
                status: None,
            })?;

        let status = resp.status();
        if !status.is_success() {
            return Err(FetchReleaseError {
                message: format!("GitHub API status {}", status),
                status: Some(status),
            });
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| FetchReleaseError {
            message: e.to_string(),
            status: None,
        })?;

        latest_release_from_json(&json).map_err(|message| FetchReleaseError {
            message,
            status: None,
        })
    }

    let primary = "https://api.github.com/repos/BoxCatTeam/endfield-cat/releases/latest";
    match fetch(client, primary).await {
        Ok(res) => Ok(res),
        Err(err)
            if matches!(
                err.status,
                Some(StatusCode::FORBIDDEN) | Some(StatusCode::TOO_MANY_REQUESTS)
            ) =>
        {
            // Fallback: use jsDelivr to read package.json for version to avoid GitHub API limits
            let fallback_url =
                "https://cdn.jsdelivr.net/gh/BoxCatTeam/endfield-cat@master/package.json";
            let resp = client
                .get(fallback_url)
                .header("User-Agent", "endfield-cat/tauri")
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !resp.status().is_success() {
                return Err(err.message);
            }

            let pkg: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            if let Some(ver) = pkg.get("version").and_then(|v| v.as_str()) {
                let tag_name = format!("v{}", ver);
                return Ok(LatestRelease {
                    tag_name,
                    name: None,
                    html_url: Some(
                        "https://github.com/BoxCatTeam/endfield-cat/releases".to_string(),
                    ),
                    download_url: None,
                    body: None,
                });
            }

            Err(err.message)
        }
        Err(err) if err.status == Some(StatusCode::NOT_FOUND) => Err(err.message),
        Err(err) => Err(err.message),
    }
}

pub async fn fetch_latest_prerelease(client: &reqwest::Client) -> Result<LatestRelease, String> {
    let url = "https://api.github.com/repos/BoxCatTeam/endfield-cat/releases?per_page=20";
    let resp = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "endfield-cat/tauri")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("GitHub API status {}", status));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let releases = json
        .as_array()
        .ok_or("Invalid GitHub response: expected array")?;

    let target = releases.iter().find(|r| {
        r.get("draft").and_then(|v| v.as_bool()) == Some(false)
            && r.get("prerelease").and_then(|v| v.as_bool()) == Some(true)
    });

    let Some(target) = target else {
        return Err("No prerelease found".to_string());
    };

    latest_release_from_json(target)
}
