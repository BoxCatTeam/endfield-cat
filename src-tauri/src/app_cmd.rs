use tauri::{AppHandle, State};
use reqwest::StatusCode;
use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};

// GitHub release 信息载体，仅挑选前端需要展示的字段
#[derive(Serialize)]
pub struct LatestRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub html_url: Option<String>,
}

#[tauri::command]
pub fn get_app_version(app: AppHandle) -> Result<String, String> {
    // 从 Tauri 配置读取应用版本，缺失时回退为 0.0.0
    let version = app
        .config()
        .version
        .clone()
        .unwrap_or_else(|| "0.0.0".to_string());
    Ok(version)
}

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
pub fn quit(app_handle: tauri::AppHandle) {
    // 主动退出应用（退出码 0）
    app_handle.exit(0);
}

#[derive(serde::Serialize)]
pub struct StoragePaths {
    pub config: String,
    pub database: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataStatus {
    pub path: String,
    pub is_empty: bool,
    pub file_count: usize,
    pub has_manifest: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteManifest {
    pub package_version: Option<String>,
    pub metadata_checksum: Option<String>,
    pub item_count: Option<usize>,
}

#[tauri::command]
pub fn get_storage_paths() -> Result<StoragePaths, String> {
    // 可执行文件同级的 data 目录下隔离配置与数据库路径
    let mut exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    exe_path.pop();
    
    let config_dir = exe_path.join("data").join("config");
    let db_dir = exe_path.join("data").join("database");
    
    // 目录不存在时提前创建，避免后续读写失败
    if !config_dir.exists() { let _ = std::fs::create_dir_all(&config_dir); }
    if !db_dir.exists() { let _ = std::fs::create_dir_all(&db_dir); }
    
    Ok(StoragePaths {
        config: config_dir.join("config.json").to_string_lossy().to_string(),
        database: db_dir.join("endcat.db").to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn read_config() -> Result<serde_json::Value, String> {
    let mut exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    exe_path.pop();
    
    let config_path = exe_path.join("data").join("config").join("config.json");
    
    if !config_path.exists() {
        // 配置文件不存在时返回空对象，保持前端调用幂等
        return Ok(serde_json::json!({}));
    }
    
    let content = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    let config: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(config)
}

#[tauri::command]
pub fn save_config(config: serde_json::Value) -> Result<(), String> {
    let mut exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    exe_path.pop();
    
    let config_dir = exe_path.join("data").join("config");
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    }
    
    let config_path = config_dir.join("config.json");
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    
    std::fs::write(&config_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

fn build_manifest_url(base_url: &str, version: &str) -> Result<String, String> {
    // 根据基础地址与版本号拼出 manifest.json 的完整地址
    let mut url = base_url.trim().to_string();
    if url.is_empty() {
        return Err("base_url is empty".to_string());
    }

    // 若传入已经带 manifest.json 的地址，先剔除文件名
    if url.ends_with("manifest.json") {
        if let Some(idx) = url.rfind('/') {
            url.truncate(idx + 1);
        }
    }

    let ver = {
        let v = version.trim();
        if v.is_empty() { "latest" } else { v }
    };

    // 如果存在占位符 {version} 则直接替换
    if url.contains("{version}") {
        url = url.replace("{version}", ver);
    } else {
        // 官方仓库未显式带 @v 时自动插入版本段
        const REPO: &str = "endfield-cat-metadata";
        if let Some(pos) = url.find(REPO) {
            let start = pos + REPO.len();
            let after = url[start..].chars().next();
            match after {
                Some('@') => {
                    let rest = &url[start + 1..];
                    if let Some(slash_offset) = rest.find('/') {
                        let abs = start + 1 + slash_offset;
                        url = format!("{}@v{}{}", &url[..start], ver, &url[abs..]);
                    } else {
                        url = format!("{}@v{}", &url[..start], ver);
                    }
                }
                _ => {
                    if let Some(slash_offset) = url[start..].find('/') {
                        let abs = start + slash_offset;
                        url = format!("{}@v{}{}", &url[..start], ver, &url[abs..]);
                    } else {
                        url = format!("{}@v{}", url, ver);
                    }
                }
            }
        }
    }

    if !url.ends_with('/') {
        url.push('/');
    }
    url.push_str("manifest.json");
    Ok(url)
}

fn count_files(dir: &Path) -> Result<usize, String> {
    // 递归统计目录下的文件数量（含子目录）
    let mut count = 0usize;
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        if ty.is_file() {
            count += 1;
        } else if ty.is_dir() {
            count += count_files(&path)?;
        }
    }
    Ok(count)
}

#[tauri::command]
pub fn check_metadata() -> Result<MetadataStatus, String> {
    let mut exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    exe_path.pop();

    let metadata_dir = exe_path.join("data").join("metadata");

    // 若不存在则创建，以便后续读写
    if !metadata_dir.exists() {
        fs::create_dir_all(&metadata_dir).map_err(|e| e.to_string())?;
    }

    let file_count = count_files(&metadata_dir)?;
    let has_manifest = metadata_dir.join("manifest.json").exists();

    Ok(MetadataStatus {
        path: metadata_dir.to_string_lossy().to_string(),
        is_empty: file_count == 0,
        file_count,
        has_manifest,
    })
}

#[tauri::command]
pub async fn fetch_metadata_manifest(
    client: State<'_, reqwest::Client>,
    base_url: String,
    version: Option<String>,
) -> Result<RemoteManifest, String> {
    // 拉取远端 manifest.json 并抽取关键字段
    let version = version.unwrap_or_else(|| "latest".to_string());
    let url = build_manifest_url(&base_url, &version)?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let package_version = json.get("package_version").and_then(|v| v.as_str()).map(|s| s.to_string());
    let metadata_checksum = json.get("metadata_checksum").and_then(|v| v.as_str()).map(|s| s.to_string());
    let item_count = json.get("item_count").and_then(|v| v.as_u64()).map(|v| v as usize);

    Ok(RemoteManifest { package_version, metadata_checksum, item_count })
}

use tauri::Emitter;

#[derive(Clone, Serialize)]
pub struct DownloadProgress {
    pub current: usize,
    pub total: usize,
    pub filename: String,
}

#[tauri::command]
pub async fn reset_metadata(
    window: tauri::Window,
    client: State<'_, reqwest::Client>,
    base_url: Option<String>,
    version: Option<String>,
) -> Result<MetadataStatus, String> {
    // 清空本地 metadata 并按 manifest 重新下载，同时向前端推送进度事件
    let mut exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    exe_path.pop();

    let metadata_dir = exe_path.join("data").join("metadata");

    if metadata_dir.exists() {
        fs::remove_dir_all(&metadata_dir).map_err(|e| e.to_string())?;
    }

    fs::create_dir_all(&metadata_dir).map_err(|e| e.to_string())?;

    let mut status = MetadataStatus {
        path: metadata_dir.to_string_lossy().to_string(),
        is_empty: true,
        file_count: 0,
        has_manifest: false,
    };

    let Some(base) = base_url.and_then(|s| {
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() { None } else { Some(trimmed) }
    }) else {
        return Ok(status);
    };

    let ver = version.unwrap_or_else(|| "latest".to_string());
    let manifest_url = build_manifest_url(&base, &ver)?;
    let manifest_base = manifest_url
        .rsplit_once('/')
        .map(|(head, _)| {
            let mut h = head.to_string();
            if !h.ends_with('/') {
                h.push('/');
            }
            h
        })
        .ok_or_else(|| "Invalid manifest url".to_string())?;

    let resp = client
        .get(&manifest_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {} when fetching manifest", resp.status()));
    }

    let manifest_bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    let manifest_path = metadata_dir.join("manifest.json");
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&manifest_path, &manifest_bytes).map_err(|e| e.to_string())?;

    let manifest_json: serde_json::Value = serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
    if let Some(entries) = manifest_json.get("entries").and_then(|v| v.as_array()) {
        let total = entries.len();
        for (i, entry) in entries.iter().enumerate() {
            let Some(path) = entry.get("path").and_then(|v| v.as_str()) else {
                continue;
            };

            let _ = window.emit("metadata-progress", DownloadProgress {
                current: i + 1,
                total,
                filename: path.to_string(),
            });

            let file_url = format!("{}{}", manifest_base, path);
            let dest_path = metadata_dir.join(path);
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }

            let file_resp = client
                .get(&file_url)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !file_resp.status().is_success() {
                return Err(format!("HTTP {} when fetching {}", file_resp.status(), path));
            }

            let bytes = file_resp.bytes().await.map_err(|e| e.to_string())?;
            fs::write(&dest_path, &bytes).map_err(|e| e.to_string())?;
        }
    }

    let file_count = count_files(&metadata_dir)?;
    let has_manifest = metadata_dir.join("manifest.json").exists();

    status = MetadataStatus {
        path: metadata_dir.to_string_lossy().to_string(),
        is_empty: file_count == 0,
        file_count,
        has_manifest,
    };

    Ok(status)
}

#[derive(Debug)]
struct FetchReleaseError {
    message: String,
    status: Option<StatusCode>,
}

#[tauri::command]
pub async fn fetch_latest_release(client: State<'_, reqwest::Client>) -> Result<LatestRelease, String> {
    // 访问 GitHub 最新 release，针对 404 单独返回信息，其余错误透传
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

        Ok(LatestRelease { tag_name, name, html_url })
    }

    let primary = "https://api.github.com/repos/BoxCatTeam/endfield-cat/releases/latest";
    match fetch(&client, primary).await {
        Ok(res) => Ok(res),
        Err(err) if err.status == Some(StatusCode::NOT_FOUND) => Err(err.message),
        Err(err) => Err(err.message),
    }
}
