use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataStatus {
    pub path: String,
    pub is_empty: bool,
    pub file_count: usize,
    pub has_manifest: bool,
    pub current_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteManifest {
    pub package_version: Option<String>,
    pub metadata_checksum: Option<String>,
    pub item_count: Option<usize>,
    pub total_size: Option<usize>,
}

#[derive(Clone, Serialize)]
pub struct DownloadProgress {
    pub current: usize,
    pub total: usize,
    pub filename: String,
}

/// Progress information for metadata update with phases
#[derive(Clone, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum UpdateProgress {
    Verifying { current: usize, total: usize, path: String },
    Downloading { current: usize, total: usize, path: String },
    Cleaning { current: usize, total: usize, path: String },
}

/// Compute SHA256 hash of a file, returns uppercase hex string
fn compute_sha256(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    
    loop {
        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    let result = hasher.finalize();
    Ok(format!("{:X}", result))
}

pub fn build_manifest_url(base_url: &str, version: &str) -> Result<String, String> {
    let mut url = base_url.trim().to_string();
    if url.is_empty() {
        return Err("base_url is empty".to_string());
    }

    if url.ends_with("manifest.json") {
        if let Some(idx) = url.rfind('/') {
            url.truncate(idx + 1);
        }
    }

    let ver = {
        let v = version.trim();
        if v.is_empty() { "latest" } else { v }
    };

    if url.contains("{version}") {
        url = url.replace("{version}", ver);
    } else {
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

pub fn check_metadata_status(exe_dir: &Path) -> Result<MetadataStatus, String> {
    let metadata_dir = exe_dir.join("data").join("metadata");

    if !metadata_dir.exists() {
        fs::create_dir_all(&metadata_dir).map_err(|e| e.to_string())?;
    }

    let file_count = count_files(&metadata_dir)?;
    let manifest_path = metadata_dir.join("manifest.json");
    let has_manifest = manifest_path.exists();
    
    let mut current_version = None;
    if has_manifest {
       if let Ok(content) = fs::read(&manifest_path) {
           if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&content) {
               current_version = json.get("package_version")
                   .and_then(|v| v.as_str())
                   .map(|s| s.to_string());
           }
       }
    }

    Ok(MetadataStatus {
        path: metadata_dir.to_string_lossy().to_string(),
        is_empty: file_count == 0,
        file_count,
        has_manifest,
        current_version,
    })
}

pub async fn fetch_manifest(
    client: &reqwest::Client,
    base_url: &str,
    version: &str,
) -> Result<RemoteManifest, String> {
    let url = build_manifest_url(base_url, version)?;

    let resp = client
        .get(&url)
        .header("Cache-Control", "no-cache, no-store, must-revalidate")
        .header("Pragma", "no-cache")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {} when fetching manifest: {}", resp.status(), url));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let package_version = json.get("package_version").and_then(|v| v.as_str()).map(|s| s.to_string());
    let metadata_checksum = json.get("metadata_checksum").and_then(|v| v.as_str()).map(|s| s.to_string());
    let item_count = json.get("item_count").and_then(|v| v.as_u64()).map(|v| v as usize);

    let total_size = json
        .get("entries")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| e.get("size").and_then(|s| s.as_u64()))
                .sum::<u64>() as usize
        });

    Ok(RemoteManifest { package_version, metadata_checksum, item_count, total_size })
}

fn cleanup_extra_files(metadata_dir: &Path, allowed: &HashSet<String>) {
    if !metadata_dir.exists() {
        return;
    }

    let mut to_remove: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(metadata_dir).into_iter().flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        if path.file_name().map(|n| n == "manifest.json").unwrap_or(false) {
            continue;
        }
        if let Some(rel) = path.strip_prefix(metadata_dir).ok() {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if !allowed.contains(&rel_str) {
                to_remove.push(path.to_path_buf());
            }
        }
    }

    for file in to_remove {
        let _ = fs::remove_file(&file);
    }
}

async fn download_metadata<F>(
    exe_dir: &Path,
    client: &reqwest::Client,
    base_url: Option<String>,
    version: Option<String>,
    clean_first: bool,
    mut on_progress: F,
) -> Result<MetadataStatus, String>
where
    F: FnMut(DownloadProgress),
{
    let metadata_dir = exe_dir.join("data").join("metadata");

    if clean_first && metadata_dir.exists() {
        fs::remove_dir_all(&metadata_dir).map_err(|e| e.to_string())?;
    }

    if !metadata_dir.exists() {
        fs::create_dir_all(&metadata_dir).map_err(|e| e.to_string())?;
    }

    let mut status = MetadataStatus {
        path: metadata_dir.to_string_lossy().to_string(),
        is_empty: true,
        file_count: 0,
        has_manifest: false,
        current_version: None,
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
        .header("Cache-Control", "no-cache, no-store, must-revalidate")
        .header("Pragma", "no-cache")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {} when fetching manifest: {}", resp.status(), manifest_url));
    }

    let manifest_bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    let manifest_path = metadata_dir.join("manifest.json");
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&manifest_path, &manifest_bytes).map_err(|e| e.to_string())?;

    let manifest_json: serde_json::Value = serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;

    let mut manifest_paths: Vec<String> = Vec::new();

    if let Some(entries) = manifest_json.get("entries").and_then(|v| v.as_array()) {
        let total = entries.len();
        for (i, entry) in entries.iter().enumerate() {
            let Some(path) = entry.get("path").and_then(|v| v.as_str()) else {
                continue;
            };

            manifest_paths.push(path.to_string());

            on_progress(DownloadProgress {
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

    if !manifest_paths.is_empty() {
        let allowed: HashSet<String> = manifest_paths.into_iter().collect();
        cleanup_extra_files(&metadata_dir, &allowed);
    }

    let file_count = count_files(&metadata_dir)?;
    let has_manifest = metadata_dir.join("manifest.json").exists();

    status = MetadataStatus {
        path: metadata_dir.to_string_lossy().to_string(),
        is_empty: file_count == 0,
        file_count,
        has_manifest,
        current_version: manifest_json.get("package_version").and_then(|v| v.as_str()).map(|s| s.to_string()),
    };

    Ok(status)
}

pub async fn reset_metadata<F>(
    exe_dir: &Path,
    client: &reqwest::Client,
    base_url: Option<String>,
    version: Option<String>,
    on_progress: F,
) -> Result<MetadataStatus, String>
where
    F: FnMut(DownloadProgress),
{
    download_metadata(exe_dir, client, base_url, version, true, on_progress).await
}

pub async fn update_metadata<F>(
    exe_dir: &Path,
    client: &reqwest::Client,
    base_url: Option<String>,
    version: Option<String>,
    mut on_progress: F,
) -> Result<MetadataStatus, String>
where
    F: FnMut(UpdateProgress),
{
    let metadata_dir = exe_dir.join("data").join("metadata");

    if !metadata_dir.exists() {
        fs::create_dir_all(&metadata_dir).map_err(|e| e.to_string())?;
    }

    let mut status = MetadataStatus {
        path: metadata_dir.to_string_lossy().to_string(),
        is_empty: true,
        file_count: 0,
        has_manifest: false,
        current_version: None,
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

    // Emit an early progress event so the UI won't be stuck at "preparing" if the manifest request is slow.
    on_progress(UpdateProgress::Verifying {
        current: 0,
        total: 1,
        path: "manifest.json".to_string(),
    });

    // Fetch remote manifest
    let resp = client
        .get(&manifest_url)
        .header("Cache-Control", "no-cache, no-store, must-revalidate")
        .header("Pragma", "no-cache")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {} when fetching manifest", resp.status()));
    }

    let manifest_bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    let manifest_json: serde_json::Value = serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;

    let entries = manifest_json
        .get("entries")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let total_entries = entries.len();
    let mut manifest_paths: HashSet<String> = HashSet::new();
    let mut to_download: Vec<(String, String)> = Vec::new(); // (path, expected_checksum)

    // Phase 1: Verify existing files
    for (i, entry) in entries.iter().enumerate() {
        let Some(path) = entry.get("path").and_then(|v| v.as_str()) else {
            continue;
        };
        let expected_checksum = entry
            .get("checksum")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_uppercase();

        manifest_paths.insert(path.to_string());

        on_progress(UpdateProgress::Verifying {
            current: i + 1,
            total: total_entries,
            path: path.to_string(),
        });

        let local_path = metadata_dir.join(path);
        
        let needs_download = if local_path.exists() {
            if expected_checksum.is_empty() {
                // No checksum in manifest, skip verification
                false
            } else {
                match compute_sha256(&local_path) {
                    Ok(local_hash) => local_hash.to_uppercase() != expected_checksum,
                    Err(_) => true, // Cannot read file, re-download
                }
            }
        } else {
            true // File doesn't exist
        };

        if needs_download {
            to_download.push((path.to_string(), expected_checksum));
        }
    }

    // Phase 2: Download missing/changed files (only if there are files to download)
    let download_total = to_download.len();
    if download_total > 0 {
        for (i, (path, _expected_checksum)) in to_download.iter().enumerate() {
            on_progress(UpdateProgress::Downloading {
                current: i + 1,
                total: download_total,
                path: path.clone(),
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

    // Phase 3: Clean up extra files
    let mut to_remove: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(&metadata_dir).into_iter().flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        if path.file_name().map(|n| n == "manifest.json").unwrap_or(false) {
            continue;
        }
        if let Some(rel) = path.strip_prefix(&metadata_dir).ok() {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if !manifest_paths.contains(&rel_str) {
                to_remove.push(path.to_path_buf());
            }
        }
    }

    // Only send clean progress if there are files to remove
    let remove_total = to_remove.len();
    if remove_total > 0 {
        for (i, file) in to_remove.iter().enumerate() {
            on_progress(UpdateProgress::Cleaning {
                current: i + 1,
                total: remove_total,
                path: file.to_string_lossy().to_string(),
            });
            let _ = fs::remove_file(file);
        }
    }

    // Save manifest after successful update
    let manifest_path = metadata_dir.join("manifest.json");
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&manifest_path, &manifest_bytes).map_err(|e| e.to_string())?;

    // Build final status
    let file_count = count_files(&metadata_dir)?;
    let has_manifest = manifest_path.exists();

    status = MetadataStatus {
        path: metadata_dir.to_string_lossy().to_string(),
        is_empty: file_count == 0,
        file_count,
        has_manifest,
        current_version: manifest_json.get("package_version").and_then(|v| v.as_str()).map(|s| s.to_string()),
    };

    Ok(status)
}
