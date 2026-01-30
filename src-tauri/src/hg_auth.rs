use tauri::utils::config::WebviewUrl;
use tauri::WebviewWindowBuilder;
use tauri::{AppHandle, Emitter, Manager, Url, WebviewWindow};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use reqwest::header;

macro_rules! log_dev {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            println!($($arg)*);
        }
    };
}

const HG_LOGIN_URL: &str = "https://user.hypergryph.com/";
const HG_TOKEN_URL: &str = "https://web-api.hypergryph.com/account/info/hg";
const ENDCAT_SCHEME: &str = "endcat";
const AUTH_UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0";

fn clear_hg_webview(win: &WebviewWindow) {
    if let Err(e) = win.clear_all_browsing_data() {
        log_dev!("[hg-auth] clear_all_browsing_data failed: {e}");
    }
    let _ = win.eval(
        "try { localStorage.clear?.(); sessionStorage.clear?.(); if (window.indexedDB?.databases) { indexedDB.databases().then(dbs => dbs.forEach(db => indexedDB.deleteDatabase(db.name))).catch(() => {}); } } catch (_) {}",
    );
}
const HG_AUTH_INIT_JS: &str = r#"
(() => {
  // Minimal auto-token extraction script
  // Does NOT modify DOM or add overlays - just monitors URL and extracts token
  const USERINFO_URL = 'https://user.hypergryph.com/userInfo';
  const TOKEN_URL = 'https://web-api.hypergryph.com/account/info/hg';
  let redirected = false;
  let extracted = false;

  function extractToken() {
    if (extracted) return;
    if (!location.href.startsWith(TOKEN_URL)) return;
    
    const text = document.body?.innerText || '';
    if (!text || text.trim().length < 2) return;
    
    try {
      const json = JSON.parse(text);
      const token = json?.token || json?.data?.token || json?.data?.content || json?.content || '';
      if (token && typeof token === 'string' && token.trim()) {
        extracted = true;
        console.log('[hg-auth] token extracted, sending to app');
        location.href = 'endcat://hg-auto-token?token=' + encodeURIComponent(token);
      }
    } catch (e) {
      console.log('[hg-auth] parse token failed', e);
    }
  }

  function checkAndRedirect() {
    // When on userInfo page, redirect to token URL
    if (!redirected && location.href.startsWith(USERINFO_URL)) {
      redirected = true;
      console.log('[hg-auth] detected userInfo, redirecting to token URL');
      location.href = TOKEN_URL;
      return;
    }
    // When on token URL, extract token
    extractToken();
  }

  function tick() {
    checkAndRedirect();
    if (!extracted) {
      setTimeout(tick, 500);
    }
  }

  // Start monitoring after DOM is ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', tick, { once: true });
  } else {
    tick();
  }
})();
"#;

#[cfg(target_os = "windows")]
fn maybe_set_disable_gpu() {
    use std::env;
    // Allow opt-in to disable GPU (can help on some Intel/Nvidia drivers), but leave it off by default
    // because it can also cause blank/black screens.
    let force = env::var("ENDCAT_FORCE_WEBVIEW_DISABLE_GPU")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !force {
        if env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS")
            .map(|v| v.contains("--disable-gpu"))
            .unwrap_or(false)
        {
            log_dev!("[hg-auth] clearing WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS (contained --disable-gpu)");
            env::remove_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS");
        } else {
            log_dev!("[hg-auth] WEBVIEW2 disable-gpu not forced (set ENDCAT_FORCE_WEBVIEW_DISABLE_GPU=1 to enable)");
        }
        return;
    }

    let args = env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
    if args.contains("--disable-gpu") {
        log_dev!("[hg-auth] WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS already has --disable-gpu");
        return;
    }

    let merged = if args.is_empty() {
        "--disable-gpu".to_string()
    } else {
        format!("{args} --disable-gpu")
    };
    env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", &merged);
    log_dev!("[hg-auth] set WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS={}", merged);
}

#[cfg(not(target_os = "windows"))]
fn maybe_set_disable_gpu() {}

async fn fetch_token_with_cookie(cookie_header: String) -> Option<String> {
    log_dev!(
        "[hg-auth] fetch_token_with_cookie: len={} preview={}",
        cookie_header.len(),
        cookie_header
            .chars()
            .take(120)
            .collect::<String>()
            .replace('\n', "")
    );
    let client = reqwest::Client::builder()
        .user_agent(AUTH_UA)
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;

    let res = client
        .get(HG_TOKEN_URL)
        .header(reqwest::header::COOKIE, cookie_header)
        .send()
        .await
        .ok()?;

    if !res.status().is_success() {
        log_dev!("[hg-auth] token fetch failed status {}", res.status());
        return None;
    }

    let json: serde_json::Value = res.json().await.ok()?;
    let token = json
        .get("token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            json.get("data")
                .and_then(|d| d.get("token"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .or_else(|| {
            json.get("data")
                .and_then(|d| d.get("content"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .or_else(|| json.get("content").and_then(|v| v.as_str()).map(|s| s.to_string()));
    if token.as_deref().unwrap_or("").is_empty() {
        log_dev!("[hg-auth] token fetch json missing token: {:?}", json);
    }
    token
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

static LAST_COOKIE_FETCH_MS: AtomicU64 = AtomicU64::new(0);
static LAST_REQ_LOG_MS: AtomicU64 = AtomicU64::new(0);
static LAST_USERINFO_NAV_MS: AtomicU64 = AtomicU64::new(0);

fn open_hg_auth_window(app: &AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("hg-auth") {
        let _ = win.show();
        let _ = win.set_focus();
        if cfg!(debug_assertions) {
            let _ = win.eval(
                "try { window.__TAURI_INTERNALS__?.invoke?.('plugin:webview|internal_toggle_devtools'); } catch (_) {}",
            );
        }
        return Ok(());
    }

    maybe_set_disable_gpu();

    let login_url = Url::parse(HG_LOGIN_URL).map_err(|e| e.to_string())?;
    let login_url_str = login_url.to_string();
    let app_for_nav = app.clone();

    log_dev!(
        "[hg-auth] building webview: target={}, gpu_flag={:?}",
        login_url_str,
        std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").ok()
    );

    let app_for_req = app.clone();
    let mut builder = WebviewWindowBuilder::new(app, "hg-auth", WebviewUrl::External(login_url.clone()))
        .title("获取 token")
        .inner_size(375.0, 650.0)
        .resizable(true)
        .decorations(true)
        .closable(true)
        .user_agent(AUTH_UA)
        .initialization_script_for_all_frames(HG_AUTH_INIT_JS)
        .on_web_resource_request(move |request, _response| {
            let uri = request.uri();
            let host = uri.host().unwrap_or_default();
            if host.is_empty() {
                return;
            }
            // Only watch Hypergryph domains.
            if !(host.contains("hypergryph.com") || host.contains("hycdn.cn")) {
                return;
            }

            let path = uri.path();
            let is_token_req =
                host.contains("web-api.hypergryph.com") && path.starts_with("/account/info/hg");

            // Periodic debug log so we know the hook is firing.
            let log_now = now_millis();
            if log_now.saturating_sub(LAST_REQ_LOG_MS.load(Ordering::Relaxed)) > 1500 {
                LAST_REQ_LOG_MS.store(log_now, Ordering::Relaxed);
                log_dev!("[hg-auth] web_request {}{}", host, path);
            }

            // Throttle to avoid hammering.
            let now = now_millis();
            // If we've already landed on userInfo, force a token fetch by jumping to the token URL from inside the webview.
            if host.contains("user.hypergryph.com") && path.starts_with("/userInfo") {
                let last_nav = LAST_USERINFO_NAV_MS.load(Ordering::Relaxed);
                if now.saturating_sub(last_nav) > 1200 {
                    LAST_USERINFO_NAV_MS.store(now, Ordering::Relaxed);
                    log_dev!("[hg-auth] detected userInfo navigation, forcing token URL");
                    if let Some(win) = app_for_req.get_webview_window("hg-auth") {
                        let _ = win.eval("try { location.href = 'https://web-api.hypergryph.com/account/info/hg'; } catch (_) {}");
                    }
                }
            }

            let last = LAST_COOKIE_FETCH_MS.load(Ordering::Relaxed);
            if !is_token_req && now.saturating_sub(last) < 800 {
                return;
            }

            let mut cookies_combined = String::new();
            for cookie_hdr in request.headers().get_all(header::COOKIE).iter() {
                if let Ok(s) = cookie_hdr.to_str() {
                    if !cookies_combined.is_empty() {
                        cookies_combined.push_str("; ");
                    }
                    cookies_combined.push_str(s.trim());
                }
            }

            if cookies_combined.trim().is_empty() {
                if is_token_req {
                    log_dev!("[hg-auth] token request observed but cookie header empty");
                }
                return;
            }

            LAST_COOKIE_FETCH_MS.store(now, Ordering::Relaxed);
            log_dev!(
                "[hg-auth] on_web_resource_request cookies from {}{} len={} (token_req={})",
                host,
                path,
                cookies_combined.len(),
                is_token_req
            );
            let app_for_fetch = app_for_req.clone();
            tauri::async_runtime::spawn(async move {
                if let Some(token) = fetch_token_with_cookie(cookies_combined).await {
                    let _ = app_for_fetch.emit_to("main", "hg:auto-token", token);
                    if let Some(win) = app_for_fetch.get_webview_window("hg-auth") {
                        clear_hg_webview(&win);
                        let _ = win.close();
                    }
                }
            });
        })
        .on_navigation(move |url| {
            log_dev!("[hg-auth] navigating {}", url);
            if url.scheme() != ENDCAT_SCHEME {
                return true;
            }

            let host = url.host_str().unwrap_or_default();

            if host == "hg-login" {
                if let Some(win) = app_for_nav.get_webview_window("hg-auth") {
                    if let Ok(url) = Url::parse(HG_LOGIN_URL) {
                        let _ = win.navigate(url);
                    }
                }
                return false;
            }

            if host == "hg-auto-token" {
                let token = url
                    .query_pairs()
                    .find_map(|(k, v)| if k == "token" { Some(v.into_owned()) } else { None })
                    .unwrap_or_default();

                if !token.trim().is_empty() {
                    let _ = app_for_nav.emit_to("main", "hg:auto-token", token);
                    if let Some(win) = app_for_nav.get_webview_window("hg-auth") {
                        clear_hg_webview(&win);
                        let _ = win.close();
                    }
                }
            }
            if host == "hg-cookies" {
                let cookies = url
                    .query_pairs()
                    .find_map(|(k, v)| if k == "cookie" { Some(v.into_owned()) } else { None })
                    .unwrap_or_default();
                if !cookies.trim().is_empty() {
                    let app_for_fetch = app_for_nav.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(token) = fetch_token_with_cookie(cookies).await {
                            let _ = app_for_fetch.emit_to("main", "hg:auto-token", token);
                            if let Some(win) = app_for_fetch.get_webview_window("hg-auth") {
                                clear_hg_webview(&win);
                                let _ = win.close();
                            }
                        }
                    });
                }
            }
            if host == "close" {
                if let Some(win) = app_for_nav.get_webview_window("hg-auth") {
                    let _ = win.close();
                }
            }

            false
        })
        .on_page_load(move |window, payload| {
            let url = payload.url();
            let url_str = url.as_str();
            log_dev!("[hg-auth] page loaded {}", url_str);
            let _ = window.eval("window.__ENDCAT_PAGE_LOADED__ = true;");
        });

    // 仅在开发环境开启 devtools
    builder = builder.devtools(cfg!(debug_assertions));

    let win = builder.build().map_err(|e| e.to_string())?;

    match win.navigate(login_url) {
        Ok(()) => log_dev!("[hg-auth] navigate() issued to {}", login_url_str),
        Err(err) => log_dev!("[hg-auth] navigate() failed to {}: {}", login_url_str, err),
    }

    // Fallback: if stuck on about:blank, navigate to login page
    let _ = win.eval(
        "setTimeout(() => { try { if (!location.href || location.href === 'about:blank') { location.href = 'https://user.hypergryph.com/'; } } catch (e) { console.error('force nav failed', e); } }, 400);",
    );
    let _ = win.eval(
        "setInterval(() => { try { console.log('[hg-auth] heartbeat', location.href, document.readyState); } catch (_) {} }, 3000);",
    );

    // 开发环境下自动打开 devtools
    if cfg!(debug_assertions) {
        win.open_devtools();
    }

    Ok(())
}

#[tauri::command]
pub async fn hg_open_token_webview(app: AppHandle) -> Result<(), String> {
    let handle = app.clone();
    app.run_on_main_thread(move || {
        if let Err(e) = open_hg_auth_window(&handle) {
            log_dev!("[hg-auth] open window failed: {e}");
        }
    })
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hg_close_token_webview(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("hg-auth") {
        clear_hg_webview(&win);
        let _ = win.close();
    }
    #[cfg(desktop)]
    {
        let _ = app.emit_to("hg-auth", "force-close", ());
    }
    Ok(())
}

#[tauri::command]
pub async fn hg_push_cookies(app: AppHandle, cookie: String) -> Result<(), String> {
    if cookie.trim().is_empty() {
        return Err("cookie is empty".into());
    }
    log_dev!("[hg-auth] hg_push_cookies len={}", cookie.len());
    let app_for_fetch = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(token) = fetch_token_with_cookie(cookie).await {
            let _ = app_for_fetch.emit_to("main", "hg:auto-token", token);
            if let Some(win) = app_for_fetch.get_webview_window("hg-auth") {
                clear_hg_webview(&win);
                let _ = win.close();
            }
        }
    });
    Ok(())
}
