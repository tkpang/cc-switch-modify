//! 飞书 OIDC 登录编排 + token 安全存储（keyring）+ 自动续期。
//!
//! 登录走系统浏览器 + loopback 回调 + PKCE（RFC 8252），全部 OIDC 细节复用
//! `lepro-core`；本文件只负责开浏览器、存 token、续期。

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use lepro_core::{loopback, oidc, pkce, LeproError};

use super::ISSUER;

const KEYRING_SERVICE: &str = "lepro-connect";
const KEYRING_USER: &str = "oidc-tokens";

/// 持久化到 OS keychain 的 token 集合。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// 取得时刻（unix 秒），用于判断 access_token 是否临过期。
    pub obtained_at: i64,
    /// 服务端返回的 expires_in；0 视为“未知”，不据此判过期。
    pub expires_in: i64,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn entry() -> Result<keyring::Entry, LeproError> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| LeproError::Io(e.to_string()))
}

pub fn save_tokens(t: &StoredTokens) -> Result<(), LeproError> {
    let json = serde_json::to_string(t).map_err(|e| LeproError::Parse(e.to_string()))?;
    entry()?
        .set_password(&json)
        .map_err(|e| LeproError::Io(e.to_string()))
}

pub fn load_tokens() -> Option<StoredTokens> {
    let s = entry().ok()?.get_password().ok()?;
    serde_json::from_str(&s).ok()
}

pub fn clear_tokens() -> Result<(), LeproError> {
    let e = entry()?;
    let _ = e.delete_password(); // 不存在视为已清除
    Ok(())
}

pub fn is_logged_in() -> bool {
    load_tokens().is_some()
}

/// 完整登录流程：PKCE → loopback 监听 → 打开系统浏览器 → 等回调 → 换 token → 存储。
///
/// `app` 用于通过 opener 插件打开系统浏览器（飞书登录在浏览器里完成）。
pub async fn login(app: &AppHandle, http: &reqwest::Client) -> Result<StoredTokens, LeproError> {
    let verifier = pkce::generate_verifier();
    let challenge = pkce::challenge_s256(&verifier);
    let state = pkce::generate_verifier(); // 复用随机生成器作 CSRF state

    let handle = loopback::listen_once().await?;
    let redirect_uri = handle.redirect_uri.clone();
    let url = oidc::build_authorize_url(ISSUER, &redirect_uri, &challenge, &state);

    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| LeproError::Io(format!("open browser: {e}")))?;

    let cb = handle.wait().await?;
    if cb.state != state {
        return Err(LeproError::Oidc("回调 state 不匹配（CSRF）".into()));
    }

    let ts = oidc::exchange_code(http, ISSUER, &cb.code, &redirect_uri, &verifier).await?;
    let stored = StoredTokens {
        access_token: ts.access_token,
        refresh_token: ts.refresh_token,
        obtained_at: now_secs(),
        expires_in: ts.expires_in,
    };
    save_tokens(&stored)?;
    Ok(stored)
}

/// 返回可用的 access_token；临过期（剩余 <60s）且有 refresh_token 时先续期并轮换存储。
pub async fn valid_access_token(http: &reqwest::Client) -> Result<String, LeproError> {
    let t = load_tokens().ok_or_else(|| LeproError::Oidc("未登录".into()))?;
    let near_expiry = t.expires_in > 0 && (now_secs() - t.obtained_at) >= (t.expires_in - 60);
    if near_expiry {
        if let Some(rt) = t.refresh_token.clone() {
            let ns = oidc::refresh(http, ISSUER, &rt).await?;
            let stored = StoredTokens {
                access_token: ns.access_token.clone(),
                refresh_token: ns.refresh_token.or(Some(rt)),
                obtained_at: now_secs(),
                expires_in: ns.expires_in,
            };
            save_tokens(&stored)?;
            return Ok(stored.access_token);
        }
    }
    Ok(t.access_token)
}
