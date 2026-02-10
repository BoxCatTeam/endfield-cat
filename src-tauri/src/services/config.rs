use std::fs;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoragePaths {
    pub config: String,
    pub data_dir: String,
    pub database: String,
    pub metadata: String,
}

pub struct ResolvedPaths {
    pub config_path: PathBuf,
    pub data_dir: PathBuf,
    pub database_path: PathBuf,
    pub metadata_dir: PathBuf,
}

fn current_exe_dir() -> Result<PathBuf, String> {
    let mut exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    exe_path.pop();
    Ok(exe_path)
}

fn app_config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_config_dir(app)?.join("config.json"))
}

fn legacy_config_path(exe_dir: &Path) -> PathBuf {
    exe_dir.join("data").join("config").join("config.json")
}

fn legacy_db_paths(app: &AppHandle, exe_dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Prefer the unconfigured default (portable `exe_dir/data` first, then `Documents/endcat`).
    if let Ok(default_dir) = default_data_dir(app) {
        paths.push(default_dir.join("database").join("endcat.db"));
    }

    // Legacy portable paths (old versions).
    paths.push(exe_dir.join("data").join("database").join("endcat.db"));
    paths.push(exe_dir.join("userData").join("endcat.db"));

    if let Ok(dir) = app_config_dir(app) {
        paths.push(dir.join("endcat.db"));
    }

    paths
}

fn default_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    // Portable mode: if a `data/` directory exists next to the executable,
    // prefer it as the data root.
    if let Ok(exe_dir) = current_exe_dir() {
        let portable = exe_dir.join("data");
        if portable.is_dir() {
            return Ok(portable);
        }
    }

    let docs = app.path().document_dir().or_else(|_| app.path().app_data_dir());
    let base = docs.map_err(|e| e.to_string())?;
    Ok(base.join("endcat"))
}

pub fn resolve_data_dir(app: &AppHandle, config: &serde_json::Value) -> Result<PathBuf, String> {
    let configured = config
        .get("dataDir")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    let default_dir = default_data_dir(app)?;
    if configured.is_empty() {
        return Ok(default_dir);
    }

    let configured_path = PathBuf::from(configured);
    if configured_path.is_absolute() {
        return Ok(configured_path);
    }

    let parent = default_dir.parent().unwrap_or(&default_dir);
    Ok(parent.join(configured_path))
}

fn migrate_legacy_config_if_needed(_app: &AppHandle, new_config_path: &Path) -> Result<(), String> {
    if new_config_path.exists() {
        return Ok(());
    }

    let exe_dir = current_exe_dir()?;
    let legacy = legacy_config_path(&exe_dir);
    if !legacy.exists() {
        return Ok(());
    }

    if let Some(parent) = new_config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let content = fs::read_to_string(&legacy).map_err(|e| e.to_string())?;
    // Only migrate if legacy JSON parses successfully.
    let _: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    fs::write(new_config_path, content).map_err(|e| e.to_string())?;

    // Keep legacy file as backup; do not delete.
    Ok(())
}

fn read_json_or_empty(path: &Path) -> Result<serde_json::Value, String> {
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(value)
}

pub fn resolve_paths(app: &AppHandle) -> Result<ResolvedPaths, String> {
    let cfg_path = config_path(app)?;
    migrate_legacy_config_if_needed(app, &cfg_path)?;
    let cfg = read_json_or_empty(&cfg_path)?;

    let data_dir = resolve_data_dir(app, &cfg)?;
    let database_path = data_dir.join("database").join("endcat.db");
    let metadata_dir = data_dir.join("metadata");

    Ok(ResolvedPaths {
        config_path: cfg_path,
        data_dir,
        database_path,
        metadata_dir,
    })
}

fn copy_sqlite_sidecar(src_db: &Path, dst_db: &Path, suffix: &str) -> Result<(), String> {
    let src = PathBuf::from(format!("{}{}", src_db.to_string_lossy(), suffix));
    if !src.exists() {
        return Ok(());
    }
    let dst = PathBuf::from(format!("{}{}", dst_db.to_string_lossy(), suffix));
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let _ = fs::copy(&src, &dst).map_err(|e| e.to_string())?;
    Ok(())
}

fn copy_sqlite_files(src_db: &Path, dst_db: &Path) -> Result<(), String> {
    if let Some(parent) = dst_db.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let _ = fs::copy(src_db, dst_db).map_err(|e| e.to_string())?;
    // WAL mode sidecars (best-effort)
    let _ = copy_sqlite_sidecar(src_db, dst_db, "-wal");
    let _ = copy_sqlite_sidecar(src_db, dst_db, "-shm");
    Ok(())
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn migrate_legacy_db_if_needed(app: &AppHandle, dst_db_path: &Path) -> Result<(), String> {
    if dst_db_path.exists() {
        return Ok(());
    }

    let exe_dir = current_exe_dir()?;
    for src in legacy_db_paths(app, &exe_dir) {
        if src.exists() {
            // Prefer copying (rename may fail跨盘，且更安全)
            if copy_sqlite_files(&src, dst_db_path).is_ok() {
                return Ok(());
            }
        }
    }

    Ok(())
}

pub fn ensure_resolved_paths(app: &AppHandle) -> Result<ResolvedPaths, String> {
    let resolved = resolve_paths(app)?;

    ensure_dir(&resolved.data_dir)?;
    let db_parent = resolved
        .database_path
        .parent()
        .ok_or("Invalid database path")?;
    ensure_dir(db_parent)?;
    ensure_dir(&resolved.metadata_dir)?;

    // Legacy DB migration (best-effort)
    migrate_legacy_db_if_needed(app, &resolved.database_path)?;

    Ok(resolved)
}

pub fn ensure_paths(app: &AppHandle) -> Result<StoragePaths, String> {
    let resolved = ensure_resolved_paths(app)?;

    Ok(StoragePaths {
        config: resolved.config_path.to_string_lossy().to_string(),
        data_dir: resolved.data_dir.to_string_lossy().to_string(),
        database: resolved.database_path.to_string_lossy().to_string(),
        metadata: resolved.metadata_dir.to_string_lossy().to_string(),
    })
}

pub fn read_config(app: &AppHandle) -> Result<serde_json::Value, String> {
    let cfg_path = config_path(app)?;
    migrate_legacy_config_if_needed(app, &cfg_path)?;
    read_json_or_empty(&cfg_path)
}

pub fn save_config(app: &AppHandle, config: serde_json::Value) -> Result<(), String> {
    let cfg_path = config_path(app)?;
    if let Some(parent) = cfg_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&cfg_path, content).map_err(|e| e.to_string())?;
    Ok(())
}
