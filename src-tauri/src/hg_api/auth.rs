use serde::Serialize;
use serde_json::Value;

use super::utils::{json_str, json_i64};

#[derive(Serialize)]
pub struct HgExchangeResult {
    pub oauth_token: String,
    pub uids: Vec<String>,      // For API requests (bindingList[].uid)
    pub role_ids: Vec<String>,  // For display (bindingList[].roles[].roleId)
    pub nick_names: Vec<String>, // For display (bindingList[].roles[].nickName)
}

/// Extract (uids, role_ids, nick_names) from binding list
/// - uids: bindingList[].uid - used for API requests
/// - role_ids: bindingList[].roles[].roleId - used for display
/// - nick_names: bindingList[].roles[].nickName - used for display
fn extract_binding_info(binding_list_json: &Value) -> (Vec<String>, Vec<String>, Vec<String>) {
    // We use vectors to keep order consistent across uids/roles/nicks if possible, 
    // but the previous implementation used sets which destroys order relative to bindings?
    // Actually the previous implementation used BTreeSet which sorts them.
    // If we have multiple roles per binding, mapping becomes tricky if we flatten them.
    // Ideally we should return a list of objects.
    // However keeping with the current pattern:
    // We will collect triplets. But `uids` are per binding, `roles` are per binding.
    // If one binding has multiple roles, we might have 1 uid vs N roles.
    // The previous code flattened ALL uids and ALL role_ids found in the entire response.
    // If we want to pair them (username for roleId), we need to ensure they align.
    // BUT `HgExchangeResult` returns flat lists. Frontend assumes index mapping?
    // Let's look at `AddAccountDialog.vue`: `uids.map((uid, i) => ({ label: roleIds[i] ?? uid, value: uid }))`
    // It assumes 1:1 mapping by index.
    // If `uids` and `role_ids` count mismatch, it breaks.
    // The previous implementation used BTreeSet so they were sorted alphabetically, breaking any index relationship!
    // That was a bug/flaw in the previous code if the user had multiple accounts.
    // I should fix this to return a list of structs or consistent vectors.
    // But to minimize disruption, I will change to Vec and preserve insertion order (if 1:1).
    // Structure: binding -> uid. binding -> roles -> roleId.
    // Usually 1 binding has 1 uid and 1 role list (often 1 role).
    // I will flatten: For each role found, I will output the corresponding uid and nickName.
    // So if 1 binding has 2 roles, I output the same UID twice?
    // The `uids` list currently is used for `hg_u8_token_by_uid`.
    // It seems `uids` in `HgExchangeResult` is just a list of valid UIDs to try to switch to?
    // Frontend uses `uids` for values.
    // I will change the logic to: Iterate apps -> bindings -> roles.
    // For each role, we push (uid, role_id, nick_name).
    // This ensures they are aligned by index.
    
    let mut results: Vec<(String, String, String)> = Vec::new();

    let Some(list) = binding_list_json.pointer("/data/list").and_then(|v| v.as_array()) else {
        println!("[hg-exchange] no /data/list in binding response");
        return (vec![], vec![], vec![]);
    };

    for app in list {
        // ... (app check logic same as before) ...
        let app_code = app.get("appCode").and_then(|v| v.as_str()).unwrap_or("");
        let app_name = app.get("appName").and_then(|v| v.as_str()).unwrap_or("");
        
        let is_endfield = app_code.to_lowercase().contains("endfield") 
            || app_name.contains("终末地")
            || app_name.to_lowercase().contains("endfield");
        
        if !is_endfield {
            continue;
        }

        let Some(binding_list) = app
            .get("bindingList")
            .or_else(|| app.get("binding_list"))
            .and_then(|v| v.as_array())
        else {
            continue;
        };

        for binding in binding_list {
            let uid = binding.get("uid").and_then(|v| v.as_str()).unwrap_or("").to_owned();
            if uid.trim().is_empty() {
                continue;
            }

            let Some(roles) = binding.get("roles").and_then(|v| v.as_array()) else {
                continue;
            };

            for role in roles {
                let role_id = role.get("roleId")
                    .or_else(|| role.get("role_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("").to_owned();
                    
                let nick_name = role.get("nickName")
                    .or_else(|| role.get("nick_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("").to_owned();

                if !role_id.trim().is_empty() {
                    results.push((uid.clone(), role_id, nick_name));
                }
            }
        }
    }

    let uids = results.iter().map(|(u, _, _)| u.clone()).collect();
    let role_ids = results.iter().map(|(_, r, _)| r.clone()).collect();
    let nick_names = results.iter().map(|(_, _, n)| n.clone()).collect();
    
    (uids, role_ids, nick_names)
}

#[tauri::command]
pub async fn hg_exchange_user_token(token: String) -> Result<HgExchangeResult, String> {
    let token = token.trim();
    println!("[hg-exchange] called with token len={}", token.len());

    if token.is_empty() {
        return Err("missing token".to_owned());
    }

    let client = reqwest::Client::builder()
        .user_agent("endfield-cat")
        .build()
        .map_err(|e| e.to_string())?;

    let grant_json = client
        .post("https://as.hypergryph.com/user/oauth2/v2/grant")
        .json(&serde_json::json!({
            "type": 1,
            "appCode": "be36d44aa36bfb5b",
            "token": token,
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())?;

    let code = json_i64(&grant_json, "code")
        .or_else(|| json_i64(&grant_json, "status"))
        .unwrap_or(-1);
    if code != 0 {
        let msg = grant_json
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("OAuth 换取失败");
        println!(
            "[hg-exchange] grant failed code={} msg={} body={:?}",
            code, msg, grant_json
        );
        return Err(msg.to_owned());
    }

    let oauth_token = json_str(&grant_json, "/data/token")
        .or_else(|| json_str(&grant_json, "/token"))
        .unwrap_or_default();
    if oauth_token.trim().is_empty() {
        println!("[hg-exchange] oauth_token missing in grant body {:?}", grant_json);
        return Err("OAuth 响应缺少 token".to_owned());
    }
    println!(
        "[hg-exchange] oauth_token len={} uids? pending binding_list",
        oauth_token.len()
    );

    let binding_json = client
        .get("https://binding-api-account-prod.hypergryph.com/account/binding/v1/binding_list")
        .query(&[("token", oauth_token.as_str()), ("appCode", "endfield")])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())?;
    
    println!("[hg-exchange] binding_list response: {:?}", binding_json);

    let status = json_i64(&binding_json, "status").unwrap_or(-1);
    if status != 0 {
        let msg = binding_json
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("绑定列表获取失败");
        return Err(msg.to_owned());
    }

    let (uids, role_ids, nick_names) = extract_binding_info(&binding_json);
    if uids.is_empty() {
        return Err("绑定列表中未解析到 uid".to_owned());
    }

    Ok(HgExchangeResult { oauth_token, uids, role_ids, nick_names })
}

#[tauri::command]
pub async fn hg_u8_token_by_uid(uid: String, oauth_token: String) -> Result<String, String> {
    println!("[hg-u8] called with uid={}, oauth_token len={}", uid, oauth_token.len());
    
    if uid.trim().is_empty() {
        return Err("missing uid".to_owned());
    }
    if oauth_token.trim().is_empty() {
        return Err("missing oauth_token".to_owned());
    }

    let client = reqwest::Client::builder()
        .user_agent("endfield-cat")
        .build()
        .map_err(|e| e.to_string())?;

    let request_body = serde_json::json!({
        "uid": uid,
        "token": oauth_token,
    });
    println!("[hg-u8] request body: {:?}", request_body);

    let u8_json = client
        .post("https://binding-api-account-prod.hypergryph.com/account/binding/v1/u8_token_by_uid")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())?;

    println!("[hg-u8] response: {:?}", u8_json);

    let status = json_i64(&u8_json, "status").unwrap_or(-1);
    if status != 0 {
        let msg = u8_json
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("u8_token 获取失败");
        return Err(msg.to_owned());
    }

    let Some(u8_token) = json_str(&u8_json, "/data/token") else {
        return Err("u8_token 响应缺少 data.token".to_owned());
    };

    println!("[hg-u8] got u8_token len={}", u8_token.len());
    Ok(u8_token)
}
