use std::fs;
use std::path::Path;

#[derive(serde::Serialize)]
pub struct StoragePaths {
    pub config: String,
    pub database: String,
}

pub fn ensure_paths(exe_dir: &Path) -> Result<StoragePaths, String> {
    let config_dir = exe_dir.join("data").join("config");
    let db_dir = exe_dir.join("data").join("database");

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    }
    if !db_dir.exists() {
        fs::create_dir_all(&db_dir).map_err(|e| e.to_string())?;
    }

    Ok(StoragePaths {
        config: config_dir.join("config.json").to_string_lossy().to_string(),
        database: db_dir.join("endcat.db").to_string_lossy().to_string(),
    })
}

pub fn read_config(exe_dir: &Path) -> Result<serde_json::Value, String> {
    let config_path = exe_dir.join("data").join("config").join("config.json");

    if !config_path.exists() {
        return Ok(serde_json::json!({}));
    }

    let content = fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    let config: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(config)
}

pub fn save_config(exe_dir: &Path, config: serde_json::Value) -> Result<(), String> {
    let config_dir = exe_dir.join("data").join("config");
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    }

    let config_path = config_dir.join("config.json");
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;

    fs::write(&config_path, content).map_err(|e| e.to_string())?;
    Ok(())
}
