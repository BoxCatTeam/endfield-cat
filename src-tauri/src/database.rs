use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite, Row};
// std::collections imported inline where needed
use tauri::{State, AppHandle};

use std::fs;

pub type DbPool = Pool<Sqlite>;

// Initialize the database pool
pub async fn init_db(_app: &AppHandle) -> Result<DbPool, Box<dyn std::error::Error>> {
    let mut exe_path = std::env::current_exe()?;
    exe_path.pop(); // Remove executable name
    
    let db_dir = exe_path.join("data").join("database");
    let config_dir = exe_path.join("data").join("config");
    let old_user_data_dir = exe_path.join("userData");
    
    // Create new directories
    if !db_dir.exists() {
        fs::create_dir_all(&db_dir)?;
    }
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    
    let db_path = db_dir.join("endcat.db");
    
    // Check migration from old location
    if !db_path.exists() {
        let old_db_path = old_user_data_dir.join("endcat.db");
        if old_db_path.exists() {
            println!("[database] Migrating DB from {:?} to {:?}", old_db_path, db_path);
            let _ = fs::rename(&old_db_path, &db_path);
            // Optional: remove empty userData dir
        }
    }

    let db_path_str = db_path.to_str().ok_or("Invalid db path")?;
    
    println!("[database] Opening DB at: {}", db_path_str);
    
    let database_url = format!("sqlite:{}?mode=rwc", db_path_str);
    
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
        
    // Manual Migrations (ensure tables exist)
    sqlx::query(r#"
CREATE TABLE IF NOT EXISTS gacha_pulls (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  uid TEXT NOT NULL,
  banner_id TEXT NOT NULL,
  banner_name TEXT NOT NULL,
  item_name TEXT NOT NULL,
  rarity INTEGER NOT NULL,
  pulled_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_gacha_pulls_uid ON gacha_pulls(uid);
CREATE INDEX IF NOT EXISTS idx_gacha_pulls_uid_time ON gacha_pulls(uid, pulled_at DESC);

CREATE TABLE IF NOT EXISTS accounts (
  uid TEXT PRIMARY KEY,
  user_token TEXT NOT NULL,
  oauth_token TEXT NOT NULL,
  u8_token TEXT NOT NULL,
  created_at INTEGER NOT NULL DEFAULT (unixepoch()),
  updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX IF NOT EXISTS idx_accounts_updated_at ON accounts(updated_at DESC);
"#).execute(&pool).await.map_err(|e| e.to_string())?;

    // Column additions (Migrations)
    let columns = vec![
        ("accounts", "role_id", "TEXT"),
        ("accounts", "nick_name", "TEXT"),
        ("gacha_pulls", "seq_id", "TEXT"),
        ("gacha_pulls", "item_id", "TEXT"),
        ("gacha_pulls", "pool_type", "TEXT"),
        ("gacha_pulls", "is_free", "INTEGER"),
        ("gacha_pulls", "is_new", "INTEGER"),
    ];
    
    for (table, col, ty) in columns {
        let check_sql = format!("SELECT count(*) FROM pragma_table_info('{}') WHERE name = '{}'", table, col);
        let count: i32 = sqlx::query_scalar(&check_sql).fetch_one(&pool).await.unwrap_or(0);
        if count == 0 {
            let alter_sql = format!("ALTER TABLE {} ADD COLUMN {} {}", table, col, ty);
            sqlx::query(&alter_sql).execute(&pool).await.ok();
        }
    }

    // Indices for seq_id
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_gacha_pulls_seq_id ON gacha_pulls(seq_id)")
        .execute(&pool).await.ok();
        
    Ok(pool)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GachaPull {
    pub uid: String,
    pub banner_id: String,
    pub banner_name: String,
    pub item_name: String,
    pub item_id: Option<String>,
    pub rarity: i64,
    pub pulled_at: i64,
    pub seq_id: Option<String>,
    pub pool_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct GachaRow {
    uid: String,
    banner_id: String,
    banner_name: String,
    item_name: String,
    item_id: Option<String>,
    rarity: i64,
    pulled_at: i64,
    seq_id: Option<String>,
    pool_type: Option<String>,
}

#[tauri::command]
pub async fn db_delete_invalid_gacha_records(
    pool: State<'_, DbPool>,
    uid: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM gacha_pulls WHERE uid = ? AND pulled_at = 0")
        .bind(uid)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn db_list_gacha_pulls(
    pool: State<'_, DbPool>,
    uid: String,
    limit: i64,
) -> Result<Vec<GachaPull>, String> {
    let rows = sqlx::query_as::<_, GachaRow>(
        "SELECT uid, banner_id, banner_name, item_name, item_id, rarity, pulled_at, seq_id, pool_type 
         FROM gacha_pulls 
         WHERE uid = ? 
         ORDER BY pulled_at DESC 
         LIMIT ?"
    )
    .bind(uid)
    .bind(limit)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let pulls = rows.into_iter().map(|r| {
        GachaPull {
            uid: r.uid,
            banner_id: r.banner_id,
            banner_name: r.banner_name,
            item_name: r.item_name,
            item_id: r.item_id,
            rarity: r.rarity,
            pulled_at: r.pulled_at,
            seq_id: r.seq_id,
            pool_type: r.pool_type,
        }
    }).collect();

    Ok(pulls)
}

#[derive(Deserialize)]
pub struct ApiGachaRecord {
    pub name: String,
    pub item_id: Option<String>,
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
pub async fn db_save_gacha_records(
    pool: State<'_, DbPool>,
    uid: String,
    records: Vec<ApiGachaRecord>,
) -> Result<(), String> {
    if records.is_empty() {
        return Ok(());
    }
    
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // We now rely on seq_id column for deduplication
    // 1. Get existing seq_ids for this UID to filtering insesrts/updates
    // Actually, `INSERT OR REPLACE` or `ON CONFLICT` strategy involves UNIQUE constraint on seq_id?
    // We don't have a UNIQUE constraint on seq_id globally (or (uid, seq_id)).
    // The current schema index is just INDEX.
    // So we should check existence manually or use UPSERT with explicit check if we can add constraint.
    // But we can't easily add constraint to existing table in SQLite without full copy-migration.
    // So manual check is safer.

    // Get existing seq_ids to avoid duplicates.
    // Since we fetch records in batches, we can just query for existence of these specifc seq_ids?
    // But API usually returns new pages.
    // Let's optimize: query all seq_id for this user? Might be large.
    // Better: For each record, try update, if not affected, insert.
    // Or: Query existing seq_ids that match the input list.
    
    let incoming_seq_ids: Vec<String> = records.iter().map(|r| r.seq_id.clone()).collect();
    // SQLite has limit on bound variables (usually 999 or 32766). 
    // If records len is small (<500), we can use `seq_id IN (...)`.
    
    let mut existing_seq_ids = std::collections::HashSet::new();
    if incoming_seq_ids.len() < 500 {
        // Construct query
        let placeholders: Vec<_> = incoming_seq_ids.iter().map(|_| "?").collect();
        let query = format!("SELECT seq_id FROM gacha_pulls WHERE uid = ? AND seq_id IN ({})", placeholders.join(","));
        let mut q = sqlx::query(&query).bind(&uid);
        for sid in &incoming_seq_ids {
            q = q.bind(sid);
        }
        
        let rows = q.fetch_all(&mut *tx).await.map_err(|e| e.to_string())?;
        for row in rows {
            let s: String = row.get("seq_id");
            existing_seq_ids.insert(s);
        }
    } else {
        // Fallback for large batches: just check one by one or fetch all user's latest (not safe)
        // Or fetch all seq_ids (only strings) if not too massive.
        // Let's assume batch size is usually small (page size 10-100).
        // If > 500, we proceed one-by-one check inside loop or chunk it. 
        // Let's just handle inside loop for robustness if list is huge, 
        // though `hg_fetch_char_records` fetches all pages before saving? 
        // Ah, `saveGachaRecords` is called with `allFetched`.
        // If user pulls 1000 items, `existing_seq_ids` query might fail if we bind all.
        // Let's skip the batch check if it's too large and rely on check-per-item or `INSERT ... WHERE NOT EXISTS`.
        // But we want to UPDATE if exists (to update pool_type etc).
    }

    // Actually, since we removed meta merging logic, we just want to ensure the record is up to date.
    // `seq_id` is the unique key from API.
    
    for r in records {
        // Try UPDATE first
        // IMPORTANT: seq_id is only unique within the same pool_type, not globally!
        // So we must include pool_type in the WHERE clause.
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
        .bind(&uid)
        .bind(&r.seq_id)
        .bind(&r.pool_type)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?
        .rows_affected();
        
        if affected == 0 {
            // INSERT
            sqlx::query(
                "INSERT INTO gacha_pulls (uid, banner_id, banner_name, item_name, item_id, rarity, pulled_at, seq_id, pool_type, is_free, is_new)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&uid)
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

// ─────────────── Account API ───────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub uid: String,
    pub role_id: Option<String>,
    pub nick_name: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AccountWithTokens {
    pub uid: String,
    pub role_id: Option<String>,
    pub nick_name: Option<String>,
    pub user_token: String,
    pub oauth_token: String,
    pub u8_token: String,
}

#[tauri::command]
pub async fn db_list_accounts(pool: State<'_, DbPool>) -> Result<Vec<Account>, String> {
    sqlx::query_as::<_, Account>(
        "SELECT uid, role_id, nick_name, updated_at FROM accounts ORDER BY updated_at DESC"
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_upsert_account(
    pool: State<'_, DbPool>,
    uid: String,
    role_id: Option<String>,
    nick_name: Option<String>,
    user_token: String,
    oauth_token: String,
    u8_token: String,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO accounts (uid, role_id, nick_name, user_token, oauth_token, u8_token, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, unixepoch(), unixepoch())
         ON CONFLICT(uid) DO UPDATE SET
           role_id = excluded.role_id,
           nick_name = excluded.nick_name,
           user_token = excluded.user_token,
           oauth_token = excluded.oauth_token,
           u8_token = excluded.u8_token,
           updated_at = unixepoch()"
    )
    .bind(uid)
    .bind(role_id)
    .bind(nick_name)
    .bind(user_token)
    .bind(oauth_token)
    .bind(u8_token)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn db_delete_account(pool: State<'_, DbPool>, uid: String) -> Result<(), String> {
    sqlx::query("DELETE FROM accounts WHERE uid = ?")
        .bind(uid)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn db_get_account_tokens(
    pool: State<'_, DbPool>,
    uid: String,
) -> Result<Option<AccountWithTokens>, String> {
    let account = sqlx::query_as::<_, AccountWithTokens>(
        "SELECT uid, role_id, nick_name, user_token, oauth_token, u8_token FROM accounts WHERE uid = ? LIMIT 1"
    )
    .bind(uid)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(account)
}
