use serde::Serialize;
use serde_json::json;
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use super::utils::{json_i64, json_str};

macro_rules! log_dev {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            println!($($arg)*);
        }
    };
}

const SYSTEM_UID_AUTO: &str = "system";
const SYSTEM_UID_OFFICIAL: &str = "system_official";
const SYSTEM_UID_BILIBILI: &str = "system_bilibili";

fn infer_system_uid(channel: Option<&str>, sub_channel: Option<&str>) -> String {
    let channel = channel.unwrap_or("");
    let sub_channel = sub_channel.unwrap_or("");

    if channel == "1" && sub_channel == "1" {
        return SYSTEM_UID_OFFICIAL.to_owned();
    }
    if channel == "2" && sub_channel == "2" {
        return SYSTEM_UID_BILIBILI.to_owned();
    }
    SYSTEM_UID_AUTO.to_owned()
}

fn default_windows_log_path() -> Result<PathBuf, String> {
    if !cfg!(target_os = "windows") {
        return Err("日志解析仅支持 Windows".to_owned());
    }
    let home = std::env::var("USERPROFILE").map_err(|_| "无法获取 USERPROFILE 环境变量".to_owned())?;
    Ok(PathBuf::from(home)
        .join("AppData")
        .join("LocalLow")
        .join("Hypergryph")
        .join("Endfield")
        .join("sdklogs")
        .join("HGWebview.log"))
}

fn read_tail_text(path: &Path, max_bytes: u64) -> Result<String, String> {
    let mut f = File::open(path).map_err(|e| format!("无法打开日志文件：{} ({})", path.display(), e))?;
    let len = f.metadata().map_err(|e| e.to_string())?.len();
    let start = len.saturating_sub(max_bytes);
    f.seek(SeekFrom::Start(start)).map_err(|e| e.to_string())?;

    let mut buf = Vec::new();
    f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

fn extract_url_from_line(line: &str) -> Option<String> {
    let start = line.find("https://ef-webview.")?;
    let mut end = line.len();
    for (i, ch) in line[start..].char_indices() {
        if ch.is_whitespace() {
            end = start + i;
            break;
        }
    }
    let raw = line[start..end].trim();
    let cleaned = raw.trim_end_matches(|c: char| matches!(c, '"' | '\'' | ')' | ']' | '}' | ',' | ';'));
    Some(cleaned.to_string())
}

fn extract_latest_gacha_url(log_text: &str) -> Option<String> {
    // Prefer gacha_char, fallback to any /page/gacha_ URL.
    for line in log_text.lines().rev() {
        if line.contains("/page/gacha_char") && line.contains("https://ef-webview.") {
            if let Some(url) = extract_url_from_line(line) {
                return Some(url);
            }
        }
    }
    for line in log_text.lines().rev() {
        if line.contains("/page/gacha_") && line.contains("https://ef-webview.") {
            if let Some(url) = extract_url_from_line(line) {
                return Some(url);
            }
        }
    }
    None
}

fn query_map(url: &tauri::Url) -> HashMap<String, String> {
    url.query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogGachaAuth {
    pub u8_token: String,
    pub server_id: String,
    pub provider: String,
    pub inferred_uid: String,
    pub channel: Option<String>,
    pub sub_channel: Option<String>,
    pub source_path: String,
    pub source_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleIdentity {
    pub role_id: String,
    pub nick_name: String,
    pub server_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleListResult {
    pub uid: String,
    pub roles: Vec<RoleIdentity>,
    pub channel_id: Option<i64>,
}

#[tauri::command]
pub async fn hg_query_role_list(
    client: tauri::State<'_, reqwest::Client>,
    token: String,
    server_id: String,
) -> Result<RoleListResult, String> {
    let parse_code = |v: &serde_json::Value| -> Option<i64> {
        v.get("code")
            .and_then(|c| c.as_i64().or_else(|| c.as_str().and_then(|s| s.parse::<i64>().ok())))
            .or_else(|| {
                v.get("status")
                    .and_then(|c| c.as_i64().or_else(|| c.as_str().and_then(|s| s.parse::<i64>().ok())))
            })
    };

    let url = "https://u8.hypergryph.com/game/role/v1/query_role_list";
    let req_body = json!({
        "token": token,
        "serverId": server_id,
    });

    let json = client
        .post(url)
        .json(&req_body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?;

    let code = parse_code(&json).unwrap_or_else(|| json_i64(&json, "code").unwrap_or(-1));
    if code != 0 {
        let msg = json.get("msg").and_then(|v| v.as_str()).unwrap_or("query_role_list 失败");
        return Err(msg.to_owned());
    }

    let Some(uid) = json_str(&json, "/data/uid") else {
        return Err("query_role_list 响应缺少 data.uid".to_owned());
    };

    let channel_id = json
        .pointer("/data/channelId")
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())));
    let roles = json.pointer("/data/roles").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut out_roles = Vec::new();
    for r in roles {
        if let Some(role_id) = r.get("roleId").and_then(|v| v.as_str()) {
            if !role_id.is_empty() {
                let nick_name = r
                    .get("nickName")
                    .or_else(|| r.get("nick_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                let server_name = r
                    .get("serverName")
                    .or_else(|| r.get("server_name"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_owned());
                out_roles.push(RoleIdentity {
                    role_id: role_id.to_owned(),
                    nick_name,
                    server_name,
                });
            }
        }
    }

    Ok(RoleListResult {
        uid,
        roles: out_roles,
        channel_id,
    })
}

#[tauri::command]
pub async fn hg_gacha_auth_from_log(log_path: Option<String>) -> Result<LogGachaAuth, String> {
    let path = match log_path {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => default_windows_log_path()?,
    };

    // Read only tail to avoid loading huge logs.
    let text = read_tail_text(&path, 2 * 1024 * 1024)?;
    let Some(url_str) = extract_latest_gacha_url(&text) else {
        return Err("未在日志中找到抽卡链接：请先在游戏内打开一次抽卡记录页面（角色池即可）再同步".to_owned());
    };

    let parsed = tauri::Url::parse(&url_str).map_err(|e| format!("抽卡链接解析失败：{} ({})", url_str, e))?;
    let q = query_map(&parsed);

    let Some(u8_token) = q.get("u8_token").cloned() else {
        return Err("抽卡链接参数解析失败：未找到 u8_token".to_owned());
    };

    let server_id = q.get("server_id").cloned().unwrap_or_else(|| "1".to_owned());
    let channel = q.get("channel").cloned();
    let sub_channel = q.get("subChannel").cloned().or_else(|| q.get("sub_channel").cloned());
    let inferred_uid = infer_system_uid(channel.as_deref(), sub_channel.as_deref());

    let provider = parsed
        .host_str()
        .and_then(|host| host.strip_prefix("ef-webview."))
        .and_then(|rest| rest.strip_suffix(".com"))
        .unwrap_or("hypergryph")
        .to_owned();

    // 日志解析暂时仅支持国服（hypergryph）。国际服请走手动添加账号流程。
    if provider != "hypergryph" {
        return Err(format!("日志暂时只支持国服（hypergryph），检测到 provider={provider}"));
    }

    log_dev!(
        "[hg-log] path={}, provider={}, inferred_uid={}, token_len={}",
        path.display(),
        provider,
        inferred_uid,
        u8_token.len()
    );

    Ok(LogGachaAuth {
        u8_token,
        server_id,
        provider,
        inferred_uid,
        channel,
        sub_channel,
        source_path: path.to_string_lossy().to_string(),
        source_url: url_str,
    })
}
