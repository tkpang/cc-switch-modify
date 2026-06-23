//! Claude settings.json 并入式构建。
//!
//! Provides `build()` function to merge Anthropic API configuration into existing
//! settings without overwriting other keys.

use crate::credentials::ModelMapping;
use serde_json::Value;

/// 归一化 Anthropic/Claude Code 的 base URL。
///
/// 分发 API 返回的 base 是 OpenAI 风格、带尾部 `/v1`（如
/// `http://host:18050/v1`），适用于 Codex 等 OpenAI 客户端。但 Claude Code
/// 用的 Anthropic SDK 会自行在 base 后拼 `/v1/messages`，若 base 也带 `/v1`
/// 就变成 `/v1/v1/messages` → 404。故为 `ANTHROPIC_BASE_URL` 去掉尾部 `/v1`。
fn normalize_anthropic_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    match trimmed.strip_suffix("/v1") {
        Some(stripped) => stripped.to_string(),
        None => trimmed.to_string(),
    }
}

/// Build a settings.json Value by merging base_url and token into existing config.
///
/// Ensures that:
/// - An `env` object exists in the output
/// - `env.ANTHROPIC_BASE_URL` is set to `base_url`
/// - `env.ANTHROPIC_AUTH_TOKEN` is set to `token`
/// - All other top-level keys from `existing` are preserved
/// - All other `env` keys from `existing` are preserved
///
/// # Arguments
///
/// * `existing` - Optional existing settings JSON (None treated as empty object)
/// * `base_url` - The Anthropic API base URL to set
/// * `token` - The Anthropic API token to set
///
/// # Returns
///
/// A new `Value` with the merged configuration.
pub fn build(
    existing: Option<Value>,
    base_url: &str,
    token: &str,
    mapping: &ModelMapping,
) -> Value {
    // Start with existing or empty object
    let mut settings = match existing {
        Some(Value::Object(obj)) => obj,
        Some(_) => {
            // If existing is not an object, start fresh
            serde_json::Map::new()
        }
        None => serde_json::Map::new(),
    };

    // Ensure env exists and is an object
    let env = settings
        .entry("env".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));

    // If env is not an object, replace it with a new object
    if !env.is_object() {
        *env = Value::Object(serde_json::Map::new());
    }

    // Now we know env is an object, set the required keys
    if let Some(env_obj) = env.as_object_mut() {
        env_obj.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            Value::String(normalize_anthropic_base_url(base_url)),
        );
        env_obj.insert(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            Value::String(token.to_string()),
        );

        // 档位映射：仅写 portal 提供的非空键；未提供的键保留现有值（不清空）。
        // Claude Code 选 Opus/Sonnet/Haiku 时分别发对应模型；默认/小快模型同理。
        let mut set = |key: &str, val: &Option<String>| {
            if let Some(v) = val {
                if !v.is_empty() {
                    env_obj.insert(key.to_string(), Value::String(v.clone()));
                }
            }
        };
        set("ANTHROPIC_MODEL", &mapping.default);
        set("ANTHROPIC_DEFAULT_OPUS_MODEL", &mapping.opus);
        set("ANTHROPIC_DEFAULT_SONNET_MODEL", &mapping.sonnet);
        set("ANTHROPIC_DEFAULT_HAIKU_MODEL", &mapping.haiku);
        set("ANTHROPIC_DEFAULT_FABLE_MODEL", &mapping.fable);
        // 小/快模型用 haiku 档（缺则用 default）。
        let fast = if mapping.haiku.is_some() {
            &mapping.haiku
        } else {
            &mapping.default
        };
        set("ANTHROPIC_SMALL_FAST_MODEL", fast);
    }

    Value::Object(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn nomap() -> ModelMapping {
        ModelMapping::default()
    }

    #[test]
    fn build_sets_env_and_preserves_other_keys() {
        let existing = json!({"env":{"FOO":"bar"},"theme":"dark"});
        let out = build(Some(existing), "http://h:18050", "sk-1", &nomap());
        assert_eq!(out["env"]["ANTHROPIC_BASE_URL"], "http://h:18050");
        assert_eq!(out["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-1");
        assert_eq!(out["env"]["FOO"], "bar"); // 保留
        assert_eq!(out["theme"], "dark"); // 保留顶层
    }

    #[test]
    fn build_from_none_creates_env() {
        let out = build(None, "http://h", "sk-2", &nomap());
        assert_eq!(out["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-2");
    }

    #[test]
    fn strips_trailing_v1_from_base_url() {
        // Anthropic SDK 会自拼 /v1/messages,base 不能带 /v1
        let out = build(None, "http://192.168.33.13:18050/v1", "sk-3", &nomap());
        assert_eq!(
            out["env"]["ANTHROPIC_BASE_URL"],
            "http://192.168.33.13:18050"
        );
    }

    #[test]
    fn strips_trailing_v1_with_slash() {
        let out = build(None, "http://h:18050/v1/", "sk-4", &nomap());
        assert_eq!(out["env"]["ANTHROPIC_BASE_URL"], "http://h:18050");
    }

    #[test]
    fn keeps_base_url_without_v1() {
        let out = build(None, "http://h:18050", "sk-5", &nomap());
        assert_eq!(out["env"]["ANTHROPIC_BASE_URL"], "http://h:18050");
    }

    #[test]
    fn does_not_strip_non_v1_suffix() {
        let out = build(None, "http://h:18050/v1beta", "sk-6", &nomap());
        assert_eq!(out["env"]["ANTHROPIC_BASE_URL"], "http://h:18050/v1beta");
    }

    #[test]
    fn writes_mapping_env_and_keeps_absent() {
        let m = ModelMapping {
            default: Some("Qwen3.6".into()),
            opus: Some("gpt-5.5".into()),
            sonnet: Some("gpt-5.4".into()),
            haiku: Some("Qwen3.6".into()),
            fable: None,
        };
        let out = build(None, "http://h:18050/v1", "sk-7", &m);
        assert_eq!(out["env"]["ANTHROPIC_MODEL"], "Qwen3.6");
        assert_eq!(out["env"]["ANTHROPIC_DEFAULT_OPUS_MODEL"], "gpt-5.5");
        assert_eq!(out["env"]["ANTHROPIC_DEFAULT_SONNET_MODEL"], "gpt-5.4");
        assert_eq!(out["env"]["ANTHROPIC_DEFAULT_HAIKU_MODEL"], "Qwen3.6");
        assert_eq!(out["env"]["ANTHROPIC_SMALL_FAST_MODEL"], "Qwen3.6");
        assert!(out["env"].get("ANTHROPIC_DEFAULT_FABLE_MODEL").is_none());
    }
}
