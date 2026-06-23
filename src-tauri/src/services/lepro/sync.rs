//! 拉取 Lepro API 凭据 + 写入/切换本地 "Lepro" Claude Code provider。
//!
//! 异步部分（续期 token + 拉凭据）与同步部分（写 provider，ProviderService 内部
//! 有 block_on）分离：命令在 tauri 同步线程里先 `block_on(fetch_creds)` 再调
//! `write_lepro_provider`，避免在 async 运行时里嵌套 block_on 造成死锁。

use lepro_core::{credentials, sync as core_sync, LeproError};

use crate::app_config::AppType;
use crate::provider::Provider;
use crate::services::provider::ProviderService;
use crate::store::AppState;

use super::{auth, PORTAL_BASE};

pub const LEPRO_PROVIDER_ID: &str = "lepro";

/// 同步结果，回前端展示。
#[derive(serde::Serialize)]
pub struct SyncOutcome {
    pub base_url: String,
    pub group: String,
    pub models: Vec<String>,
}

/// 异步：续期 access_token + 拉取分发 API 凭据。
pub async fn fetch_creds(http: &reqwest::Client) -> Result<credentials::Credentials, LeproError> {
    let token = auth::valid_access_token(http).await?;
    credentials::fetch(http, PORTAL_BASE, &token).await
}

/// 在指定 app 下新建/更新 "Lepro" provider（不切换）。切换交给前端总开关
/// （前端持有各 app 当前供应商,负责记忆「上次非 Lepro」以便关闭时回退）。
fn upsert_lepro_provider(
    state: &AppState,
    app: AppType,
    settings: serde_json::Value,
) -> Result<(), LeproError> {
    let provider = Provider::with_id(
        LEPRO_PROVIDER_ID.to_string(),
        "Lepro".to_string(),
        settings,
        None,
    );
    // 新建；若已存在则改为更新（兼容 save 非 upsert 的实现）。
    if ProviderService::add(state, app.clone(), provider.clone(), false).is_err() {
        ProviderService::update(state, app, Some(LEPRO_PROVIDER_ID), provider)
            .map_err(|e| LeproError::Io(e.to_string()))?;
    }
    Ok(())
}

/// 同步：为 **全部 4 个 app**（Claude CLI / Claude App / Codex / Gemini）写入/更新
/// "Lepro" provider（各自对应格式）。**不切换** —— 由前端总开关决定开/关。
/// 必须在**非 async-runtime 线程**调用（ProviderService 内部 block_on）。
pub fn write_lepro_provider(
    state: &AppState,
    creds: &credentials::Credentials,
) -> Result<(), LeproError> {
    // Claude CLI 与 Claude App 共用同一套 env(ANTHROPIC_*);切换时 ClaudeDesktop
    // 由 ProviderService 内部转成 3P profile 写入。
    let claude = core_sync::to_provider_settings(creds, None);
    upsert_lepro_provider(state, AppType::Claude, claude.clone())?;
    upsert_lepro_provider(state, AppType::ClaudeDesktop, claude)?;
    upsert_lepro_provider(
        state,
        AppType::Codex,
        core_sync::to_codex_provider_settings(creds),
    )?;
    upsert_lepro_provider(
        state,
        AppType::Gemini,
        core_sync::to_gemini_provider_settings(creds),
    )?;
    Ok(())
}
