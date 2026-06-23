//! Gemini provider settings 构建（`~/.gemini/.env`）。
//!
//! cc-switch 的 Gemini provider 的 `settings_config` 形如:
//! `{ "env": { "GOOGLE_GEMINI_BASE_URL": "...", "GEMINI_API_KEY": "...", "GEMINI_MODEL": "..." } }`，
//! 切换时写入 `~/.gemini/.env`。
//!
//! gemini-cli 会在 base 后自行拼 `/v1beta/models/<model>:generateContent`，故
//! `GOOGLE_GEMINI_BASE_URL` **去掉尾部 `/v1`**。中转站按模型名路由。

use serde_json::{json, Value};

/// Gemini 默认模型。中转站按模型名路由。
pub const DEFAULT_GEMINI_MODEL: &str = "gpt-5.5";

/// 去掉尾部 `/v1`（gemini-cli 自己拼 `/v1beta/...`）。
fn gemini_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    trimmed.strip_suffix("/v1").unwrap_or(trimmed).to_string()
}

/// 构建 Gemini provider 的 `settings_config`（`{ env: {...} }`）。
pub fn build(base_url: &str, token: &str, model: &str) -> Value {
    json!({
        "env": {
            "GOOGLE_GEMINI_BASE_URL": gemini_base_url(base_url),
            "GEMINI_API_KEY": token,
            "GEMINI_MODEL": model,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_sets_env() {
        let v = build("http://h:18050/v1", "sk-gem", "gpt-5.5");
        assert_eq!(v["env"]["GOOGLE_GEMINI_BASE_URL"], "http://h:18050"); // /v1 去掉
        assert_eq!(v["env"]["GEMINI_API_KEY"], "sk-gem");
        assert_eq!(v["env"]["GEMINI_MODEL"], "gpt-5.5");
    }

    #[test]
    fn base_url_without_v1_kept() {
        let v = build("http://h:18050", "t", "m");
        assert_eq!(v["env"]["GOOGLE_GEMINI_BASE_URL"], "http://h:18050");
    }
}
