//! Codex provider settings 构建（`~/.codex/auth.json` + `config.toml`）。
//!
//! cc-switch 的 Codex provider 的 `settings_config` 形如:
//! `{ "auth": { "OPENAI_API_KEY": "..." }, "config": "<config.toml 文本>" }`，
//! 切换时分别写入 `~/.codex/auth.json` 与 `~/.codex/config.toml`。
//!
//! base URL 用 OpenAI 风格、**保留尾部 `/v1`**（Codex 会拼 `/responses`、
//! `/chat/completions`）。中转站按模型名路由，故 `model` 只是默认值，用户在
//! Codex 内 `/model` 仍可切到中转站的其它模型。

use serde_json::{json, Value};

/// Codex 默认模型。中转站按模型名路由,Codex 内可切其它模型。
pub const DEFAULT_CODEX_MODEL: &str = "gpt-5.5";

/// 归一化 Codex base URL:保证恰好一个尾部 `/v1`。
fn codex_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    let no_v1 = trimmed.strip_suffix("/v1").unwrap_or(trimmed);
    format!("{no_v1}/v1")
}

/// 极简 TOML 字符串字面量（base_url/model 不含控制字符,转义引号与反斜杠即可）。
fn toml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// 构建 Codex provider 的 `settings_config`（`{ auth, config }`）。
pub fn build(base_url: &str, token: &str, model: &str) -> Value {
    let config = format!(
        "model_provider = \"custom\"\n\
         model = {model}\n\
         model_reasoning_effort = \"high\"\n\
         disable_response_storage = true\n\
         \n\
         [model_providers.custom]\n\
         name = \"Lepro\"\n\
         base_url = {base}\n\
         wire_api = \"responses\"\n\
         requires_openai_auth = true",
        model = toml_string(model),
        base = toml_string(&codex_base_url(base_url)),
    );
    json!({
        "auth": { "OPENAI_API_KEY": token },
        "config": config,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_sets_auth_and_config() {
        let v = build("http://h:18050/v1", "sk-codex", "gpt-5.5");
        assert_eq!(v["auth"]["OPENAI_API_KEY"], "sk-codex");
        let cfg = v["config"].as_str().unwrap();
        assert!(cfg.contains("base_url = \"http://h:18050/v1\""));
        assert!(cfg.contains("model = \"gpt-5.5\""));
        assert!(cfg.contains("wire_api = \"responses\""));
        assert!(cfg.contains("[model_providers.custom]"));
    }

    #[test]
    fn base_url_gets_exactly_one_v1() {
        // 已带 /v1
        let v = build("http://h:18050/v1", "t", "m");
        assert!(v["config"].as_str().unwrap().contains("base_url = \"http://h:18050/v1\""));
        // 不带 /v1 → 补上
        let v = build("http://h:18050", "t", "m");
        assert!(v["config"].as_str().unwrap().contains("base_url = \"http://h:18050/v1\""));
        // 尾部斜杠
        let v = build("http://h:18050/v1/", "t", "m");
        assert!(v["config"].as_str().unwrap().contains("base_url = \"http://h:18050/v1\""));
    }
}
