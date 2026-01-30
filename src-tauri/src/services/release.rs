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

pub async fn fetch_latest_release(client: &reqwest::Client) -> Result<LatestRelease, String> {
    async fn fetch(client: &reqwest::Client, url: &str) -> Result<LatestRelease, FetchReleaseError> {
        let resp = client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| FetchReleaseError { message: e.to_string(), status: None })?;

        let status = resp.status();
        if !status.is_success() {
            return Err(FetchReleaseError { message: format!("GitHub API status {}", status), status: Some(status) });
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| FetchReleaseError { message: e.to_string(), status: None })?;
        let tag_name = json
            .get("tag_name")
            .or_else(|| json.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if tag_name.is_empty() {
            return Err(FetchReleaseError { message: "Missing tag_name in GitHub response".to_string(), status: None });
        }

        let name = json.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
        let html_url = json.get("html_url").and_then(|v| v.as_str()).map(|s| s.to_string());
        let body = json.get("body").and_then(|v| v.as_str()).map(|s| s.to_string());
        
        let download_url = json.get("assets")
            .and_then(|v| v.as_array())
            .and_then(|assets| {
                assets.iter().find_map(|asset| {
                    let name = asset.get("name").and_then(|v| v.as_str())?;
                    if name.ends_with(".exe") {
                        asset.get("browser_download_url").and_then(|v| v.as_str()).map(|s| s.to_string())
                    } else {
                        None
                    }
                })
            });

        Ok(LatestRelease { tag_name, name, html_url, download_url, body })
    }

    let primary = "https://api.github.com/repos/BoxCatTeam/endfield-cat/releases/latest";
    match fetch(client, primary).await {
        Ok(res) => Ok(res),
        Err(err) if err.status == Some(StatusCode::NOT_FOUND) => Err(err.message),
        Err(err) => Err(err.message),
    }
}
