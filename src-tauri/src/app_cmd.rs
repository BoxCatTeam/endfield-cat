use crate::services::{config, metadata, mirror, release, update};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub fn get_app_version(app: AppHandle) -> Result<String, String> {
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
pub fn quit(app_handle: AppHandle) {
    app_handle.exit(0);
}

fn exe_dir() -> Result<std::path::PathBuf, String> {
    let mut exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    exe_path.pop();
    Ok(exe_path)
}

#[tauri::command]
pub fn get_storage_paths() -> Result<config::StoragePaths, String> {
    let exe_dir = exe_dir()?;
    config::ensure_paths(&exe_dir)
}

#[tauri::command]
pub fn read_config() -> Result<serde_json::Value, String> {
    let exe_dir = exe_dir()?;
    config::read_config(&exe_dir)
}

#[tauri::command]
pub fn save_config(config: serde_json::Value) -> Result<(), String> {
    let exe_dir = exe_dir()?;
    config::save_config(&exe_dir, config)
}

#[tauri::command]
pub fn check_metadata() -> Result<metadata::MetadataStatus, String> {
    let exe_dir = exe_dir()?;
    metadata::check_metadata_status(&exe_dir)
}

#[tauri::command]
pub async fn fetch_metadata_manifest(
    client: State<'_, reqwest::Client>,
    base_url: String,
    version: Option<String>,
) -> Result<metadata::RemoteManifest, String> {
    let ver = version.unwrap_or_else(|| "latest".to_string());
    metadata::fetch_manifest(&client, &base_url, &ver).await
}

#[tauri::command]
pub async fn reset_metadata(
    window: tauri::Window,
    client: State<'_, reqwest::Client>,
    base_url: Option<String>,
    version: Option<String>,
) -> Result<metadata::MetadataStatus, String> {
    let exe_dir = exe_dir()?;

    metadata::reset_metadata(
        &exe_dir,
        &client,
        base_url,
        version,
        |progress| {
            let _ = window.emit("metadata-progress", progress);
        },
    )
    .await
}

#[tauri::command]
pub async fn update_metadata(
    window: tauri::Window,
    app: AppHandle,
    client: State<'_, reqwest::Client>,
    base_url: Option<String>,
) -> Result<metadata::MetadataStatus, String> {
    let exe_dir = exe_dir()?;
    
    // Use app version for metadata URL
    let app_version = app.package_info().version.to_string();

    metadata::update_metadata(
        &exe_dir,
        &client,
        base_url,
        Some(app_version),
        |progress| {
            let _ = window.emit("metadata-update-progress", progress);
        },
    )
    .await
}

#[tauri::command]
pub async fn fetch_latest_release(client: State<'_, reqwest::Client>) -> Result<release::LatestRelease, String> {
    release::fetch_latest_release(&client).await
}

#[tauri::command]
pub async fn fetch_latest_prerelease(client: State<'_, reqwest::Client>) -> Result<release::LatestRelease, String> {
    release::fetch_latest_prerelease(&client).await
}

#[tauri::command]
pub async fn download_and_apply_update(
    window: tauri::Window,
    app: AppHandle,
    client: State<'_, reqwest::Client>,
    download_url: String,
) -> Result<(), String> {
    let emit_progress = |stage: &str, progress: u32| {
        let _ = window.emit("update-progress", update::UpdateProgress {
            stage: stage.to_string(),
            progress,
        });
    };

    emit_progress("downloading", 0);

    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = current_exe.parent().ok_or("Cannot get exe directory")?.to_path_buf();
    let exe_name = current_exe.file_name().ok_or("Cannot get exe name")?;

    let paths = update::prepare_paths(exe_name)?;

    // 读取镜像配置并转换 URL
    let mirror_config = mirror::read_mirror_config(&exe_dir);
    let actual_download_url = mirror_config.transform_url(&download_url);

    update::download_new_exe(&client, &actual_download_url, &paths.new_exe, |p| {
        emit_progress("downloading", p);
    }).await?;

    emit_progress("preparing", 100);

    let batch_content = update::build_updater_batch(
        &exe_name.to_string_lossy(),
        &paths.new_exe,
        &current_exe,
        &paths.temp_dir,
    );
    std::fs::write(&paths.batch_path, batch_content).map_err(|e| e.to_string())?;

    emit_progress("installing", 100);

    // 启动更新脚本：使用 start /min 创建独立最小化窗口，脚本结束后窗口会自动关闭
    std::process::Command::new("cmd")
        .args([
            "/C",
            &format!("start \"\" /min \"{}\"", paths.batch_path.to_string_lossy()),
        ])
        .current_dir(&exe_dir)
        .spawn()
        .map_err(|e| e.to_string())?;

    app.exit(0);
    Ok(())
}

/// 测试 GitHub 镜像连通性，返回延迟毫秒数
#[tauri::command]
pub async fn test_github_mirror(
    client: State<'_, reqwest::Client>,
    mirror_url_template: String,
) -> Result<u64, String> {
    // 使用一个小的 GitHub 文件测试连通性
    let test_url = "https://raw.githubusercontent.com/BoxCatTeam/endfield-cat/master/package.json";
    let proxied_url = mirror_url_template.replace("{url}", test_url);

    let start = std::time::Instant::now();
    let resp = client
        .head(&proxied_url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    Ok(start.elapsed().as_millis() as u64)
}
