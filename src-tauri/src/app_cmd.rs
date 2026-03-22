use crate::services::{config, metadata, mirror, release, update};
use tauri::{AppHandle, Emitter, State};

#[cfg(target_os = "windows")]
fn open_dir_in_os(path: &std::path::Path) -> Result<(), String> {
    std::process::Command::new("explorer")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
fn open_dir_in_os(path: &std::path::Path) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn open_dir_in_os(path: &std::path::Path) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

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

#[tauri::command]
pub fn get_storage_paths(app: AppHandle) -> Result<config::StoragePaths, String> {
    config::ensure_paths(&app)
}

#[tauri::command]
pub fn open_data_dir(app: AppHandle) -> Result<(), String> {
    let resolved = config::ensure_resolved_paths(&app)?;
    open_dir_in_os(&resolved.data_dir)
}

#[tauri::command]
pub fn read_config(app: AppHandle) -> Result<serde_json::Value, String> {
    config::read_config(&app)
}

#[tauri::command]
pub fn save_config(app: AppHandle, config: serde_json::Value) -> Result<(), String> {
    config::save_config(&app, config)
}

#[tauri::command]
pub fn check_metadata(app: AppHandle) -> Result<metadata::MetadataStatus, String> {
    let resolved = config::ensure_resolved_paths(&app)?;
    metadata::check_metadata_status(&resolved.data_dir)
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
    app: AppHandle,
    client: State<'_, reqwest::Client>,
    base_url: Option<String>,
    version: Option<String>,
) -> Result<metadata::MetadataStatus, String> {
    let resolved = config::ensure_resolved_paths(&app)?;

    metadata::reset_metadata(
        &resolved.data_dir,
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
    let resolved = config::ensure_resolved_paths(&app)?;

    metadata::update_metadata(
        &resolved.data_dir,
        &client,
        base_url,
        None,
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
    let mirror_config = mirror::read_mirror_config(&app);
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
