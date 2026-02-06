//! Sync commands that combine API calls and database operations.
//! These are high-level commands called by the frontend.

use serde::Serialize;
use tauri::State;
use std::collections::HashMap;

use crate::database::{DbPool, ApiGachaRecord};
use crate::hg_api::gacha::GachaRecord;
use crate::hg_api::utils::{json_i64, json_str};

macro_rules! log_dev {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            println!($($arg)*)
        }
    };
}

fn normalize_provider(provider: Option<String>) -> Result<String, String> {
    let raw = provider.unwrap_or_else(|| "hypergryph".to_owned());
    let p = raw.trim().to_lowercase();
    match p.as_str() {
        "hypergryph" | "gryphline" => Ok(p),
        _ => Err(format!("unsupported provider: {raw}")),
    }
}

fn provider_from_channel_id(channel_id: Option<i64>) -> String {
    if channel_id == Some(6) {
        "gryphline".to_owned()
    } else {
        "hypergryph".to_owned()
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Internal API helpers (non-tauri-command versions)
// ───────────────────────────────────────────────────────────────────────────

async fn get_u8_token(
    client: &reqwest::Client,
    uid: &str,
    oauth_token: &str,
    provider: &str,
) -> Result<String, String> {
    let request_body = serde_json::json!({
        "uid": uid,
        "token": oauth_token,
    });

    let u8_json = client
        .post(format!(
            "https://binding-api-account-prod.{provider}.com/account/binding/v1/u8_token_by_uid"
        ))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?;

    let status = json_i64(&u8_json, "status").unwrap_or(-1);
    if status != 0 {
        let msg = u8_json
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("u8_token 获取失败");
        return Err(msg.to_owned());
    }

    json_str(&u8_json, "/data/token").ok_or_else(|| "u8_token 响应缺少 data.token".to_owned())
}

#[derive(Debug)]
struct RoleInfo {
    uid: String,
    role_id: Option<String>,
    nick_name: Option<String>,
    channel_id: Option<i64>,
}

async fn query_role_list(
    client: &reqwest::Client,
    token: &str,
    server_id: &str,
) -> Result<RoleInfo, String> {
    let url = "https://u8.hypergryph.com/game/role/v1/query_role_list";
    let req_body = serde_json::json!({
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

    let code = json_i64(&json, "code")
        .or_else(|| json_i64(&json, "status"))
        .unwrap_or(-1);
    if code != 0 {
        let msg = json
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("query_role_list 失败");
        return Err(msg.to_owned());
    }

    let uid = json_str(&json, "/data/uid").ok_or("query_role_list 响应缺少 data.uid")?;
    let channel_id = json
        .pointer("/data/channelId")
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())));

    let roles = json
        .pointer("/data/roles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let (role_id, nick_name) = if let Some(first_role) = roles.first() {
        let rid = first_role
            .get("roleId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        let nn = first_role
            .get("nickName")
            .or_else(|| first_role.get("nick_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        (rid, nn)
    } else {
        (None, None)
    };

    Ok(RoleInfo {
        uid,
        role_id,
        nick_name,
        channel_id,
    })
}

async fn fetch_char_records_internal(
    client: &reqwest::Client,
    token: &str,
    server_id: &str,
    pool_type: &str,
    last_seq_id_stop: Option<&str>,
    provider: &str,
) -> Result<Vec<GachaRecord>, String> {
    let url = format!("https://ef-webview.{provider}.com/api/record/char");
    let mut all_records = Vec::new();
    let mut next_seq_id: Option<String> = None;

    'outer: loop {
        let mut params = vec![
            ("token", token),
            ("server_id", server_id),
            ("lang", "zh-cn"),
            ("pool_type", pool_type),
        ];
        let seq_holder;
        if let Some(seq) = &next_seq_id {
            seq_holder = seq.clone();
            params.push(("seq_id", &seq_holder));
        }

        let json = client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())?;

        let code = json_i64(&json, "code")
            .or_else(|| json_i64(&json, "status"))
            .unwrap_or(-1);
        if code != 0 {
            let msg = json
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("获取寻访记录失败");
            return Err(msg.to_owned());
        }

        let list = json.pointer("/data/list").and_then(|v| v.as_array());
        let Some(list) = list else { break };
        if list.is_empty() {
            break;
        }

        for item in list {
            let seq_id = item
                .get("seqId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();

            if let Some(stop_id) = last_seq_id_stop {
                if seq_id == stop_id {
                    break 'outer;
                }
            }

            let record = GachaRecord {
                name: item
                    .get("charName")
                    .or(item.get("charId"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned(),
                item_id: item
                    .get("charId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned(),
                rarity: item
                    .get("rarity")
                    .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
                    .unwrap_or(0),
                pool_id: item
                    .get("poolId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned(),
                pool_name: item
                    .get("poolName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned(),
                seq_id,
                pulled_at: item
                    .get("gachaTs")
                    .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
                    .unwrap_or(0),
                pool_type: pool_type.to_owned(),
                is_free: item.get("isFree").and_then(|v| v.as_bool()).unwrap_or(false),
                is_new: item.get("isNew").and_then(|v| v.as_bool()).unwrap_or(false),
            };
            all_records.push(record);
        }

        if let Some(last) = all_records.last() {
            next_seq_id = Some(last.seq_id.clone());
        } else {
            break;
        }

        if all_records.len() > 10000 {
            break;
        }

        if let Some(has_more) = json.pointer("/data/hasMore").and_then(|v| v.as_bool()) {
            if !has_more {
                break;
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    Ok(all_records)
}

async fn fetch_weapon_pools_internal(
    client: &reqwest::Client,
    token: &str,
    server_id: &str,
    provider: &str,
) -> Result<Vec<(String, String)>, String> {
    let url = format!("https://ef-webview.{provider}.com/api/record/weapon/pool");
    let params = [
        ("token", token),
        ("server_id", server_id),
        ("lang", "zh-cn"),
    ];

    let json = client
        .get(&url)
        .query(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?;

    let code = json_i64(&json, "code")
        .or_else(|| json_i64(&json, "status"))
        .unwrap_or(-1);
    if code != 0 {
        let msg = json
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("获取武器池失败");
        return Err(msg.to_owned());
    }

    let data = json
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let pools: Vec<(String, String)> = data
        .iter()
        .map(|item| {
            let pool_id = item
                .get("poolId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            let pool_name = item
                .get("poolName")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            (pool_id, pool_name)
        })
        .collect();

    Ok(pools)
}

async fn fetch_weapon_records_internal(
    client: &reqwest::Client,
    token: &str,
    server_id: &str,
    pool_id: &str,
    last_seq_id_stop: Option<&str>,
    provider: &str,
) -> Result<Vec<GachaRecord>, String> {
    let url = format!("https://ef-webview.{provider}.com/api/record/weapon");
    let mut all_records = Vec::new();
    let mut next_seq_id: Option<String> = None;

    'outer: loop {
        let mut params = vec![
            ("token", token),
            ("server_id", server_id),
            ("pool_id", pool_id),
            ("lang", "zh-cn"),
        ];
        let seq_holder;
        if let Some(seq) = &next_seq_id {
            seq_holder = seq.clone();
            params.push(("seq_id", &seq_holder));
        }

        let json = client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())?;

        let code = json_i64(&json, "code")
            .or_else(|| json_i64(&json, "status"))
            .unwrap_or(-1);
        if code != 0 {
            let msg = json
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("获取武器记录失败");
            return Err(msg.to_owned());
        }

        let list = json.pointer("/data/list").and_then(|v| v.as_array());
        let Some(list) = list else { break };
        if list.is_empty() {
            break;
        }

        for item in list {
            let seq_id = item
                .get("seqId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();

            if let Some(stop_id) = last_seq_id_stop {
                if seq_id == stop_id {
                    break 'outer;
                }
            }

            let record = GachaRecord {
                name: item
                    .get("weaponName")
                    .or(item.get("weaponId"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned(),
                item_id: item
                    .get("weaponId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned(),
                rarity: item
                    .get("rarity")
                    .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
                    .unwrap_or(0),
                pool_id: item
                    .get("poolId")
                    .and_then(|v| v.as_str())
                    .unwrap_or(pool_id)
                    .to_owned(),
                pool_name: item
                    .get("poolName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned(),
                seq_id,
                pulled_at: item
                    .get("gachaTs")
                    .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
                    .unwrap_or(0),
                pool_type: "E_CharacterGachaPoolType_Weapon".to_string(),
                is_free: item.get("isFree").and_then(|v| v.as_bool()).unwrap_or(false),
                is_new: item.get("isNew").and_then(|v| v.as_bool()).unwrap_or(false),
            };
            all_records.push(record);
        }

        if let Some(last) = all_records.last() {
            next_seq_id = Some(last.seq_id.clone());
        } else {
            break;
        }

        if all_records.len() > 10000 {
            break;
        }

        if let Some(has_more) = json.pointer("/data/hasMore").and_then(|v| v.as_bool()) {
            if !has_more {
                break;
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    Ok(all_records)
}

fn gacha_to_api_record(r: GachaRecord) -> ApiGachaRecord {
    ApiGachaRecord {
        name: r.name,
        item_id: Some(r.item_id),
        rarity: r.rarity,
        pool_id: r.pool_id,
        pool_name: r.pool_name,
        seq_id: r.seq_id,
        pulled_at: r.pulled_at,
        pool_type: r.pool_type,
        is_free: r.is_free,
        is_new: r.is_new,
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Public Tauri Commands
// ───────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub count: usize,
    pub account_updated: bool,
}

/// Sync gacha records for an existing account using stored OAuth token.
/// This command:
/// 1. Gets u8_token from stored oauth_token
/// 2. Queries role info and updates account (channel_id, role_id, nick_name)
/// 3. Fetches all gacha records
/// 4. Saves records to database
#[tauri::command]
pub async fn sync_gacha_by_token(
    pool: State<'_, DbPool>,
    client: State<'_, reqwest::Client>,
    uid: String,
    mode: String, // "incremental" or "full"
) -> Result<SyncResult, String> {
    log_dev!("[sync] sync_gacha_by_token uid={}, mode={}", uid, mode);

    // 1. Get account with tokens
    let account = sqlx::query_as::<_, crate::database::AccountWithTokens>(
        "SELECT uid, role_id, nick_name, server_id, channel_id, user_token, oauth_token, u8_token FROM accounts WHERE uid = ? LIMIT 1"
    )
    .bind(&uid)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("账户不存在: {uid}"))?;

    let oauth_token = account.oauth_token.as_ref().filter(|s| !s.is_empty())
        .ok_or("账户缺少 OAuth Token，请重新登录")?;

    let server_id = account.server_id.as_deref().unwrap_or("1");
    let provider = provider_from_channel_id(account.channel_id);

    // 2. Get fresh u8_token
    let u8_token = get_u8_token(&client, &uid, oauth_token, &provider).await?;

    // 3. Query role info and update account
    let role_info = query_role_list(&client, &u8_token, server_id).await.ok();
    let mut account_updated = false;

    if let Some(info) = &role_info {
        sqlx::query(
            "UPDATE accounts SET role_id = COALESCE(?, role_id), nick_name = COALESCE(?, nick_name), channel_id = COALESCE(?, channel_id), updated_at = unixepoch() WHERE uid = ?"
        )
        .bind(&info.role_id)
        .bind(&info.nick_name)
        .bind(info.channel_id)
        .bind(&uid)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
        account_updated = true;
        log_dev!("[sync] account updated: role_id={:?}, channel_id={:?}", info.role_id, info.channel_id);
    }

    // 4. Get last seq_ids for incremental mode
    let mut last_seq_map: HashMap<String, String> = HashMap::new();
    if mode == "incremental" {
        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT pool_type, seq_id FROM gacha_pulls WHERE uid = ? AND seq_id IS NOT NULL ORDER BY pulled_at DESC LIMIT 1000"
        )
        .bind(&uid)
        .fetch_all(pool.inner())
        .await
        .unwrap_or_default();

        for (pool_type, seq_id) in rows {
            last_seq_map.entry(pool_type).or_insert(seq_id);
        }
    }

    // 5. Delete invalid records if full mode
    if mode == "full" {
        sqlx::query("DELETE FROM gacha_pulls WHERE uid = ? AND pulled_at = 0")
            .bind(&uid)
            .execute(pool.inner())
            .await
            .ok();
    }

    // 6. Fetch all gacha records
    let pool_types = [
        "E_CharacterGachaPoolType_Special",
        "E_CharacterGachaPoolType_Standard",
        "E_CharacterGachaPoolType_Beginner",
    ];

    let mut all_records: Vec<GachaRecord> = Vec::new();

    for pt in pool_types {
        let stop_at = last_seq_map.get(pt).map(|s| s.as_str());
        match fetch_char_records_internal(&client, &u8_token, server_id, pt, stop_at, &provider).await {
            Ok(records) => all_records.extend(records),
            Err(e) => log_dev!("[sync] fetch char {} failed: {}", pt, e),
        }
    }

    // Fetch weapon pools and records
    if let Ok(weapon_pools) = fetch_weapon_pools_internal(&client, &u8_token, server_id, &provider).await {
        for (pool_id, _pool_name) in weapon_pools {
            let stop_at = last_seq_map.get(&pool_id).map(|s| s.as_str());
            match fetch_weapon_records_internal(&client, &u8_token, server_id, &pool_id, stop_at, &provider).await {
                Ok(records) => all_records.extend(records),
                Err(e) => log_dev!("[sync] fetch weapon {} failed: {}", pool_id, e),
            }
        }
    }

    log_dev!("[sync] fetched {} total records", all_records.len());

    // 7. Save to database
    if !all_records.is_empty() {
        let api_records: Vec<ApiGachaRecord> = all_records.iter().cloned().map(gacha_to_api_record).collect();
        save_gacha_records_internal(pool.inner(), &uid, api_records).await?;
    }

    Ok(SyncResult {
        count: all_records.len(),
        account_updated,
    })
}

/// Internal function to save gacha records (mirrors db_save_gacha_records logic)
async fn save_gacha_records_internal(
    pool: &DbPool,
    uid: &str,
    records: Vec<ApiGachaRecord>,
) -> Result<(), String> {
    if records.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    for r in records {
        let affected = sqlx::query(
            "UPDATE gacha_pulls SET 
                banner_id = ?, banner_name = ?, item_name = ?, item_id = ?, rarity = ?, pulled_at = ?, is_free = ?, is_new = ?
             WHERE uid = ? AND seq_id = ? AND pool_type = ?"
        )
        .bind(&r.pool_id)
        .bind(&r.pool_name)
        .bind(&r.name)
        .bind(&r.item_id)
        .bind(r.rarity)
        .bind(r.pulled_at)
        .bind(r.is_free)
        .bind(r.is_new)
        .bind(uid)
        .bind(&r.seq_id)
        .bind(&r.pool_type)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected();

        if affected == 0 {
            sqlx::query(
                "INSERT INTO gacha_pulls (uid, banner_id, banner_name, item_name, item_id, rarity, pulled_at, seq_id, pool_type, is_free, is_new)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(uid)
            .bind(&r.pool_id)
            .bind(&r.pool_name)
            .bind(&r.name)
            .bind(&r.item_id)
            .bind(r.rarity)
            .bind(r.pulled_at)
            .bind(&r.seq_id)
            .bind(&r.pool_type)
            .bind(r.is_free)
            .bind(r.is_new)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────
// sync_gacha_from_log - Sync using game log file
// ───────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogSyncResult {
    pub uid: String,
    pub count: usize,
}

/// Sync gacha records by parsing game log file.
#[tauri::command]
pub async fn sync_gacha_from_log(
    pool: State<'_, DbPool>,
    client: State<'_, reqwest::Client>,
    log_path: Option<String>,
    mode: String,
) -> Result<LogSyncResult, String> {
    use std::path::PathBuf;
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};

    log_dev!("[sync] sync_gacha_from_log mode={}", mode);

    fn default_log_path() -> Result<PathBuf, String> {
        let home = std::env::var("USERPROFILE").map_err(|_| "无法获取 USERPROFILE")?;
        Ok(PathBuf::from(home).join("AppData/LocalLow/Hypergryph/Endfield/sdklogs/HGWebview.log"))
    }

    fn read_tail(path: &std::path::Path, max: u64) -> Result<String, String> {
        let mut f = File::open(path).map_err(|e| format!("无法打开日志: {}", e))?;
        let len = f.metadata().map_err(|e| e.to_string())?.len();
        f.seek(SeekFrom::Start(len.saturating_sub(max))).ok();
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        Ok(String::from_utf8_lossy(&buf).to_string())
    }

    fn extract_url(text: &str) -> Option<String> {
        for line in text.lines().rev() {
            if line.contains("/page/gacha_") && line.contains("https://ef-webview.") {
                if let Some(start) = line.find("https://ef-webview.") {
                    let mut end = line.len();
                    for (i, ch) in line[start..].char_indices() {
                        if ch.is_whitespace() { end = start + i; break; }
                    }
                    return Some(line[start..end].trim_end_matches(|c: char| matches!(c, '"' | '\'' | ')' | ']' | '}')).to_string());
                }
            }
        }
        None
    }

    let path = match log_path {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => default_log_path()?,
    };

    let text = read_tail(&path, 2 * 1024 * 1024)?;
    let url_str = extract_url(&text).ok_or("未在日志中找到抽卡链接")?;
    let parsed = tauri::Url::parse(&url_str).map_err(|e| format!("链接解析失败: {}", e))?;

    let q: HashMap<String, String> = parsed.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    let u8_token = q.get("u8_token").cloned().ok_or("缺少 u8_token")?;
    let server_id = q.get("server_id").cloned().unwrap_or_else(|| "1".to_owned());

    let provider = parsed.host_str()
        .and_then(|h| h.strip_prefix("ef-webview."))
        .and_then(|r| r.strip_suffix(".com"))
        .unwrap_or("hypergryph");

    if provider != "hypergryph" {
        return Err(format!("日志暂只支持国服，检测到 provider={}", provider));
    }

    let role_info = query_role_list(&client, &u8_token, &server_id).await?;
    let uid = role_info.uid.clone();

    // Upsert account
    //
    // Note: older DB schemas may have NOT NULL constraints on token columns.
    // Log sync only provides `u8_token`, so we fill `user_token`/`oauth_token` with empty strings
    // to satisfy those constraints while avoiding overwriting existing non-empty tokens.
    sqlx::query(
        "INSERT INTO accounts (uid, role_id, nick_name, server_id, channel_id, user_token, oauth_token, u8_token, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, '', '', ?, unixepoch(), unixepoch())
         ON CONFLICT(uid) DO UPDATE SET
           role_id = COALESCE(excluded.role_id, accounts.role_id),
           nick_name = COALESCE(excluded.nick_name, accounts.nick_name),
           server_id = COALESCE(excluded.server_id, accounts.server_id),
           channel_id = COALESCE(excluded.channel_id, accounts.channel_id),
           user_token = CASE WHEN excluded.user_token != '' THEN excluded.user_token ELSE accounts.user_token END,
           oauth_token = CASE WHEN excluded.oauth_token != '' THEN excluded.oauth_token ELSE accounts.oauth_token END,
           u8_token = COALESCE(excluded.u8_token, accounts.u8_token),
           updated_at = unixepoch()"
    )
    .bind(&uid)
    .bind(&role_info.role_id)
    .bind(&role_info.nick_name)
    .bind(&server_id)
    .bind(role_info.channel_id)
    .bind(&u8_token)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let mut last_seq_map: HashMap<String, String> = HashMap::new();
    if mode == "incremental" {
        for (pt, sid) in sqlx::query_as::<_, (String, String)>("SELECT pool_type, seq_id FROM gacha_pulls WHERE uid=? AND seq_id IS NOT NULL ORDER BY pulled_at DESC LIMIT 1000").bind(&uid).fetch_all(pool.inner()).await.unwrap_or_default() {
            last_seq_map.entry(pt).or_insert(sid);
        }
    }
    if mode == "full" {
        sqlx::query("DELETE FROM gacha_pulls WHERE uid=? AND pulled_at=0").bind(&uid).execute(pool.inner()).await.ok();
    }

    let pts = ["E_CharacterGachaPoolType_Special", "E_CharacterGachaPoolType_Standard", "E_CharacterGachaPoolType_Beginner"];
    let mut all: Vec<GachaRecord> = Vec::new();
    for pt in pts {
        if let Ok(recs) = fetch_char_records_internal(&client, &u8_token, &server_id, pt, last_seq_map.get(pt).map(|s| s.as_str()), provider).await { all.extend(recs); }
    }
    if let Ok(pools) = fetch_weapon_pools_internal(&client, &u8_token, &server_id, provider).await {
        for (pid, _) in pools {
            if let Ok(recs) = fetch_weapon_records_internal(&client, &u8_token, &server_id, &pid, last_seq_map.get(&pid).map(|s| s.as_str()), provider).await { all.extend(recs); }
        }
    }

    if !all.is_empty() {
        save_gacha_records_internal(pool.inner(), &uid, all.iter().cloned().map(gacha_to_api_record).collect()).await?;
    }

    Ok(LogSyncResult { uid, count: all.len() })
}

// ───────────────────────────────────────────────────────────────────────────
// add_account_by_token - Add account using user token
// ───────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddedAccount { pub uid: String, pub role_id: String, pub nick_name: String, pub server_id: String }

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddAccountResult { pub accounts: Vec<AddedAccount> }

fn app_code(provider: &str) -> &'static str {
    if provider == "gryphline" { "3dacefa138426cfe" } else { "be36d44aa36bfb5b" }
}

#[tauri::command]
pub async fn add_account_by_token(
    pool: State<'_, DbPool>,
    client: State<'_, reqwest::Client>,
    user_token: String,
    provider: Option<String>,
) -> Result<AddAccountResult, String> {
    let provider = normalize_provider(provider)?;
    let user_token = user_token.trim();
    if user_token.is_empty() { return Err("missing token".into()); }

    let grant = client.post(format!("https://as.{provider}.com/user/oauth2/v2/grant"))
        .json(&serde_json::json!({"type": 1, "appCode": app_code(&provider), "token": user_token}))
        .send().await.map_err(|e| e.to_string())?
        .json::<serde_json::Value>().await.map_err(|e| e.to_string())?;

    let code = json_i64(&grant, "code").or_else(|| json_i64(&grant, "status")).unwrap_or(-1);
    if code != 0 { return Err(grant.get("msg").and_then(|v| v.as_str()).unwrap_or("OAuth 换取失败").into()); }

    let oauth = json_str(&grant, "/data/token").or_else(|| json_str(&grant, "/token")).ok_or("OAuth 响应缺少 token")?;

    let bind = client.get(format!("https://binding-api-account-prod.{provider}.com/account/binding/v1/binding_list"))
        .query(&[("token", oauth.as_str()), ("appCode", "endfield")])
        .send().await.map_err(|e| e.to_string())?
        .json::<serde_json::Value>().await.map_err(|e| e.to_string())?;

    if json_i64(&bind, "status").unwrap_or(-1) != 0 {
        return Err(bind.get("msg").and_then(|v| v.as_str()).unwrap_or("绑定列表获取失败").into());
    }

    let mut added = Vec::new();
    for app in bind.pointer("/data/list").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
        let ac = app.get("appCode").and_then(|v| v.as_str()).unwrap_or("");
        let an = app.get("appName").and_then(|v| v.as_str()).unwrap_or("");
        if !ac.to_lowercase().contains("endfield") && !an.contains("终末地") && !an.to_lowercase().contains("endfield") { continue; }

        for binding in app.get("bindingList").or_else(|| app.get("binding_list")).and_then(|v| v.as_array()).cloned().unwrap_or_default() {
            let uid = binding.get("uid").and_then(|v| v.as_str()).unwrap_or("").to_owned();
            if uid.is_empty() { continue; }
            let cmi = binding.get("channelMasterId").or_else(|| binding.get("channel_master_id")).and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())));

            for role in binding.get("roles").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
                let rid = role.get("roleId").or_else(|| role.get("role_id")).and_then(|v| v.as_str()).unwrap_or("").to_owned();
                let nn = role.get("nickName").or_else(|| role.get("nick_name")).and_then(|v| v.as_str()).unwrap_or("").to_owned();
                let sid = role.get("serverId").or_else(|| role.get("server_id")).and_then(|v| v.as_str()).unwrap_or("1").to_owned();
                if rid.is_empty() { continue; }

                let u8t = get_u8_token(&client, &uid, &oauth, &provider).await.ok();

                sqlx::query(
                    "INSERT INTO accounts (uid, role_id, nick_name, server_id, channel_id, user_token, oauth_token, u8_token, created_at, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, COALESCE(?, ''), unixepoch(), unixepoch())
                     ON CONFLICT(uid) DO UPDATE SET
                       role_id = COALESCE(excluded.role_id, role_id),
                       nick_name = COALESCE(excluded.nick_name, nick_name),
                       server_id = COALESCE(excluded.server_id, server_id),
                       channel_id = COALESCE(excluded.channel_id, channel_id),
                       user_token = CASE WHEN excluded.user_token != '' THEN excluded.user_token ELSE user_token END,
                       oauth_token = CASE WHEN excluded.oauth_token != '' THEN excluded.oauth_token ELSE oauth_token END,
                       u8_token = CASE WHEN excluded.u8_token != '' THEN excluded.u8_token ELSE u8_token END,
                       updated_at = unixepoch()"
                )
                .bind(&uid)
                .bind(&rid)
                .bind(&nn)
                .bind(&sid)
                .bind(cmi)
                .bind(user_token)
                .bind(&oauth)
                .bind(&u8t)
                .execute(pool.inner())
                .await
                .map_err(|e| e.to_string())?;

                added.push(AddedAccount { uid: uid.clone(), role_id: rid, nick_name: nn, server_id: sid });
            }
        }
    }

    if added.is_empty() { return Err("绑定列表中未解析到有效账户".into()); }
    Ok(AddAccountResult { accounts: added })
}
