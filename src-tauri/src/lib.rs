// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod app_cmd;
mod database;
mod hg_api;
mod hg_auth;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Directories are created in database::init_db now, ensuring they exist before DB access.
    // We can skip duplicate checks here or just ensure app starts cleanly.

    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let pool = tauri::async_runtime::block_on(async move {
                database::init_db(&handle).await
            }).expect("Failed to init db");
            app.manage(pool);
            
            // Create shared HTTP client to avoid blocking main thread
            let http_client = reqwest::Client::builder()
                .user_agent("endfield-cat")
                .build()
                .expect("Failed to build HTTP client");
            app.manage(http_client);
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_cmd::greet,
            app_cmd::quit,
            app_cmd::get_app_version,
            app_cmd::get_storage_paths,
            app_cmd::read_config,
            app_cmd::save_config,
            app_cmd::reset_metadata,
            app_cmd::fetch_metadata_manifest,
            app_cmd::check_metadata,
            app_cmd::fetch_latest_release,
            hg_api::auth::hg_exchange_user_token,
            hg_api::auth::hg_u8_token_by_uid,
            hg_api::gacha::hg_fetch_char_records,
            hg_api::gacha::hg_fetch_weapon_pools,
            hg_api::gacha::hg_fetch_weapon_records,
            hg_auth::hg_open_token_webview,
            hg_auth::hg_close_token_webview,
            hg_auth::hg_push_cookies,
            database::db_delete_invalid_gacha_records,
            database::db_list_gacha_pulls,
            database::db_save_gacha_records,
            database::db_list_accounts,
            database::db_upsert_account,
            database::db_delete_account,
            database::db_get_account_tokens
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
