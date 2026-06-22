//! Lepro 集成：飞书登录 + 自动同步 Lepro API key/模型到本地 Claude Code。
//!
//! 可移植逻辑（PKCE/OIDC/loopback/凭据/同步决策）在 `lepro-core` crate；
//! 本层只做 IO/编排：keyring token 存储、系统浏览器、写 provider、Tauri 命令。

pub mod auth;
pub mod sync;

/// lepro_aio 自建 OIDC Provider 的 issuer。
pub const ISSUER: &str = "http://192.168.33.13:18000/sso";

/// lepro_aio 门户 base（分发 API `/api/me/llm-credentials` 在其下）。
#[allow(dead_code)]
pub const PORTAL_BASE: &str = "http://192.168.33.13:18000";
