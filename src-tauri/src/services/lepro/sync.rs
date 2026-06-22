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

/// 同步：把凭据写成 "Lepro" provider 并切换为当前（覆盖 `~/.claude/settings.json`）。
/// 必须在**非 async-runtime 线程**调用（ProviderService 内部 block_on）。
pub fn write_lepro_provider(
    state: &AppState,
    creds: &credentials::Credentials,
) -> Result<(), LeproError> {
    let settings = core_sync::to_provider_settings(creds, None);
    let provider = Provider::with_id(
        LEPRO_PROVIDER_ID.to_string(),
        "Lepro".to_string(),
        settings,
        None,
    );
    // 新建；若已存在则改为更新（兼容 save 非 upsert 的实现）。
    if ProviderService::add(state, AppType::Claude, provider.clone(), false).is_err() {
        ProviderService::update(state, AppType::Claude, Some(LEPRO_PROVIDER_ID), provider)
            .map_err(|e| LeproError::Io(e.to_string()))?;
    }
    // 切换为当前 → 写入 ~/.claude/settings.json。
    ProviderService::switch(state, AppType::Claude, LEPRO_PROVIDER_ID)
        .map_err(|e| LeproError::Io(e.to_string()))?;
    Ok(())
}
