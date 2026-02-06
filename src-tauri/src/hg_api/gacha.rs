use serde::Serialize;
use serde_json::Value;
use super::utils::json_i64;

macro_rules! log_dev {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            println!($($arg)*);
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

#[derive(Serialize, Clone)]
pub struct GachaRecord {
    pub name: String,
    pub item_id: String,
    pub rarity: i64,
    pub pool_id: String,
    pub pool_name: String,
    pub seq_id: String,
    pub pulled_at: i64,
    pub pool_type: String,
    pub is_free: bool,
    pub is_new: bool,
}

#[tauri::command]
pub async fn hg_fetch_char_records(
    client: tauri::State<'_, reqwest::Client>,
    token: String,
    server_id: String,
    pool_type: String,
    last_seq_id_stop: Option<String>,
    provider: Option<String>,
) -> Result<Vec<GachaRecord>, String> {
    log_dev!("[hg-gacha] fetching char records: pool_type={}, stop_at={:?}", pool_type, last_seq_id_stop);

    let provider = normalize_provider(provider)?;
    let url = format!("https://ef-webview.{provider}.com/api/record/char");
    let mut all_records = Vec::new();
    let mut next_seq_id: Option<String> = None;

    'outer: loop {
        let mut params = vec![
            ("token", token.as_str()),
            ("server_id", server_id.as_str()),
            ("lang", "zh-cn"),
            ("pool_type", pool_type.as_str()),
        ];
        if let Some(seq) = &next_seq_id {
            params.push(("seq_id", seq));
        }

        log_dev!("[hg-gacha] fetching page seq_id={:?}", next_seq_id);

        let json = client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<Value>()
            .await
            .map_err(|e| e.to_string())?;

        let code = json_i64(&json, "code")
            .or_else(|| json_i64(&json, "status"))
            .unwrap_or(-1);
        if code != 0 {
            let msg = json.get("msg").and_then(|v| v.as_str()).unwrap_or("获取寻访记录失败");
            return Err(msg.to_owned());
        }

        let list = json.pointer("/data/list").and_then(|v| v.as_array());
        let Some(list) = list else {
            break;
        };
        if list.is_empty() {
            break;
        }

        for item in list {
            let seq_id = item.get("seqId").and_then(|v| v.as_str()).unwrap_or("").to_owned();
            
            // Incremental stop check
            if let Some(stop_id) = &last_seq_id_stop {
                if &seq_id == stop_id {
                    log_dev!("[hg-gacha] reached last_seq_id={}, stopping", stop_id);
                    break 'outer;
                }
            }

            let record = GachaRecord {
                name: item.get("charName").or(item.get("charId")).and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                item_id: item.get("charId").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                rarity: item.get("rarity").and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0),
                pool_id: item.get("poolId").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                pool_name: item.get("poolName").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                seq_id,
                pulled_at: item.get("gachaTs").and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0),
                pool_type: pool_type.clone(),
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
            log_dev!("[hg-gacha] too many records, breaking");
            break;
        }
        
        if let Some(has_more) = json.pointer("/data/hasMore").and_then(|v| v.as_bool()) {
            if !has_more {
                break;
            }
        }
        
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    log_dev!("[hg-gacha] fetched total {} char records", all_records.len());
    Ok(all_records)
}

#[derive(Serialize)]
pub struct WeaponPool {
    pub pool_id: String,
    pub pool_name: String,
}

#[tauri::command]
pub async fn hg_fetch_weapon_pools(
    client: tauri::State<'_, reqwest::Client>,
    token: String,
    server_id: String,
    provider: Option<String>,
) -> Result<Vec<WeaponPool>, String> {
    log_dev!("[hg-gacha] fetching weapon pools");

    let provider = normalize_provider(provider)?;
    let url = format!("https://ef-webview.{provider}.com/api/record/weapon/pool");
    let params = [
        ("token", token),
        ("server_id", server_id),
        ("lang", "zh-cn".to_string()),
    ];

    let json = client
        .get(&url)
        .query(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())?;

    let code = json_i64(&json, "code")
        .or_else(|| json_i64(&json, "status"))
        .unwrap_or(-1);
    if code != 0 {
        let msg = json.get("msg").and_then(|v| v.as_str()).unwrap_or("获取武器池失败");
        return Err(msg.to_owned());
    }

    let data = json.get("data").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let pools: Vec<WeaponPool> = data.iter().map(|item| {
        WeaponPool {
            pool_id: item.get("poolId").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
            pool_name: item.get("poolName").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
        }
    }).collect();

    log_dev!("[hg-gacha] fetched {} weapon pools", pools.len());
    Ok(pools)
}

#[tauri::command]
pub async fn hg_fetch_weapon_records(
    client: tauri::State<'_, reqwest::Client>,
    token: String,
    server_id: String,
    pool_id: String,
    last_seq_id_stop: Option<String>,
    provider: Option<String>,
) -> Result<Vec<GachaRecord>, String> {
    log_dev!("[hg-gacha] fetching weapon records: pool_id={}, stop_at={:?}", pool_id, last_seq_id_stop);

    let provider = normalize_provider(provider)?;
    let url = format!("https://ef-webview.{provider}.com/api/record/weapon");
    let mut all_records = Vec::new();
    let mut next_seq_id: Option<String> = None;

    'outer: loop {
        let mut params = vec![
            ("token", token.as_str()),
            ("server_id", server_id.as_str()),
            ("pool_id", pool_id.as_str()),
            ("lang", "zh-cn"),
        ];
        if let Some(seq) = &next_seq_id {
            params.push(("seq_id", seq));
        }

        log_dev!("[hg-gacha] fetching weapon page seq_id={:?}", next_seq_id);

        let json = client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<Value>()
            .await
            .map_err(|e| e.to_string())?;

        let code = json_i64(&json, "code")
            .or_else(|| json_i64(&json, "status"))
            .unwrap_or(-1);
        if code != 0 {
            let msg = json.get("msg").and_then(|v| v.as_str()).unwrap_or("获取武器记录失败");
            return Err(msg.to_owned());
        }

        let list = json.pointer("/data/list").and_then(|v| v.as_array());
        let Some(list) = list else {
            break;
        };
        if list.is_empty() {
            break;
        }

        for item in list {
            let seq_id = item.get("seqId").and_then(|v| v.as_str()).unwrap_or("").to_owned();

            // Incremental stop check
            if let Some(stop_id) = &last_seq_id_stop {
                if &seq_id == stop_id {
                    log_dev!("[hg-gacha] reached weapon last_seq_id={}, stopping", stop_id);
                    break 'outer;
                }
            }

            let record = GachaRecord {
                name: item.get("weaponName").or(item.get("weaponId")).and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                item_id: item.get("weaponId").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                rarity: item.get("rarity").and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0),
                pool_id: item.get("poolId").and_then(|v| v.as_str()).unwrap_or(&pool_id).to_owned(),
                pool_name: item.get("poolName").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                seq_id,
                pulled_at: item.get("gachaTs").and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0),
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

    log_dev!("[hg-gacha] fetched total {} weapon records", all_records.len());
    Ok(all_records)
}
