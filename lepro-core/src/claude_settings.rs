//! Claude settings.json 并入式构建。
//!
//! Provides `build()` function to merge Anthropic API configuration into existing
//! settings without overwriting other keys.

use serde_json::Value;

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
pub fn build(existing: Option<Value>, base_url: &str, token: &str) -> Value {
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
            Value::String(base_url.to_string()),
        );
        env_obj.insert(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            Value::String(token.to_string()),
        );
    }

    Value::Object(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_sets_env_and_preserves_other_keys() {
        let existing = json!({"env":{"FOO":"bar"},"theme":"dark"});
        let out = build(Some(existing), "http://h:18050", "sk-1");
        assert_eq!(out["env"]["ANTHROPIC_BASE_URL"], "http://h:18050");
        assert_eq!(out["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-1");
        assert_eq!(out["env"]["FOO"], "bar"); // 保留
        assert_eq!(out["theme"], "dark"); // 保留顶层
    }

    #[test]
    fn build_from_none_creates_env() {
        let out = build(None, "http://h", "sk-2");
        assert_eq!(out["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-2");
    }
}
