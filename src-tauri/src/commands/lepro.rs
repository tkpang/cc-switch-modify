//! Lepro 集成的 Tauri 命令：飞书登录 / 登出 / 登录态 / 强制同步。

use std::time::Duration;

use tauri::{AppHandle, State};

use crate::services::lepro::{auth, sync};
use crate::store::AppState;

/// 带超时的 HTTP 客户端：凭据/令牌端点不可达时，命令不会无限阻塞
/// （loopback 等待浏览器回调的阻塞不在此列，见 PR 说明的后续硬化项）。
fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// 飞书登录：开系统浏览器走 OIDC（loopback+PKCE），成功后 token 存 keychain。
#[tauri::command]
pub async fn lepro_login(app: AppHandle) -> Result<bool, String> {
    let http = http_client();
    auth::login(&app, &http).await.map_err(|e| e.to_string())?;
    Ok(true)
}

/// 登出：清除本地 token。
#[tauri::command]
pub fn lepro_logout() -> Result<(), String> {
    auth::clear_tokens().map_err(|e| e.to_string())
}

/// 是否已登录（本地是否存有 token）。
#[tauri::command]
pub fn lepro_auth_status() -> Result<bool, String> {
    Ok(auth::is_logged_in())
}

/// 强制同步：拉取凭据 → 写入并切换 "Lepro" provider（更新 ~/.claude/settings.json）。
/// 同步命令（跑在 tauri 阻塞线程）：先 block_on 拉凭据，再做同步写入。
#[tauri::command]
pub fn lepro_force_sync(state: State<'_, AppState>) -> Result<sync::SyncOutcome, String> {
    let http = http_client();
    let creds =
        tauri::async_runtime::block_on(sync::fetch_creds(&http)).map_err(|e| e.to_string())?;
    sync::write_lepro_provider(state.inner(), &creds).map_err(|e| e.to_string())?;
    Ok(sync::SyncOutcome {
        base_url: creds.base_url,
        group: creds.group,
        models: creds.models,
    })
}
