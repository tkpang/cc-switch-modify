//! lepro-core: lepro-connect 桌面端的可移植核心逻辑（不依赖 tauri，可在任意平台测试）。
//!
//! 模块在 Plan 2a Task 1-6 逐步填充：pkce / oidc / loopback / credentials / claude_settings / sync。

pub mod claude_settings;
pub mod credentials;
pub mod error;
pub mod loopback;
pub mod oidc;
pub mod pkce;
pub mod sync;

pub use error::LeproError;
