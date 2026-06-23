//! Sync decision logic: determine whether credentials changed and map to provider settings.

use crate::credentials::Credentials;
use serde_json::Value;

/// Determine whether credentials need updating.
///
/// Returns `true` if:
/// - `old` is `None` (first-time sync)
/// - `old` and `new` differ in any field (base_url, token, or models)
///
/// # Arguments
///
/// * `old` - Previous credentials (None if never synced)
/// * `new` - Current credentials
///
/// # Returns
///
/// `true` if update is needed, `false` if unchanged.
pub fn needs_update(old: Option<&Credentials>, new: &Credentials) -> bool {
    old.map_or(true, |o| o != new)
}

/// Map credentials to Claude provider settings JSON.
///
/// Delegates to `crate::claude_settings::build()` to merge base_url and token
/// into the existing provider settings without affecting other keys.
///
/// # Arguments
///
/// * `creds` - The credentials to map
/// * `existing` - Optional existing settings JSON to preserve
///
/// # Returns
///
/// A merged settings JSON value with ANTHROPIC_BASE_URL and ANTHROPIC_AUTH_TOKEN set.
pub fn to_provider_settings(creds: &Credentials, existing: Option<Value>) -> Value {
    crate::claude_settings::build(
        existing,
        &creds.base_url,
        &creds.token,
        &creds.model_mapping,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::ModelMapping;
    use serde_json::json;

    #[test]
    fn needs_update_true_on_token_change() {
        let a = Credentials {
            base_url: "b".into(),
            token: "t1".into(),
            group: "default".into(),
            models: vec![],
            model_mapping: ModelMapping::default(),
        };
        let b = Credentials {
            token: "t2".into(),
            ..a.clone()
        };
        assert!(needs_update(Some(&a), &b));
        assert!(!needs_update(Some(&a), &a.clone()));
        assert!(needs_update(None, &a));
    }

    #[test]
    fn to_provider_settings_sets_env_keys() {
        let creds = Credentials {
            base_url: "http://localhost:18050".into(),
            token: "sk-test-123".into(),
            group: "default".into(),
            models: vec!["qwen-max".into()],
            model_mapping: ModelMapping::default(),
        };
        let existing = json!({"env": {"FOO": "bar"}, "theme": "dark"});
        let result = to_provider_settings(&creds, Some(existing));

        assert_eq!(
            result["env"]["ANTHROPIC_BASE_URL"],
            "http://localhost:18050"
        );
        assert_eq!(result["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-test-123");
        assert_eq!(result["env"]["FOO"], "bar"); // Preserve existing env keys
        assert_eq!(result["theme"], "dark"); // Preserve top-level keys
                                             // 空映射不写档位键
        assert!(result["env"]
            .get("ANTHROPIC_DEFAULT_SONNET_MODEL")
            .is_none());
    }

    #[test]
    fn to_provider_settings_writes_model_mapping() {
        let creds = Credentials {
            base_url: "http://h:18050/v1".into(),
            token: "sk-x".into(),
            group: "default".into(),
            models: vec![],
            model_mapping: ModelMapping {
                default: Some("Qwen3.6".into()),
                opus: Some("gpt-5.5".into()),
                sonnet: Some("gpt-5.4".into()),
                haiku: Some("Qwen3.6".into()),
                fable: None,
            },
        };
        let r = to_provider_settings(&creds, None);
        assert_eq!(r["env"]["ANTHROPIC_BASE_URL"], "http://h:18050"); // /v1 stripped
        assert_eq!(r["env"]["ANTHROPIC_MODEL"], "Qwen3.6");
        assert_eq!(r["env"]["ANTHROPIC_DEFAULT_OPUS_MODEL"], "gpt-5.5");
        assert_eq!(r["env"]["ANTHROPIC_DEFAULT_SONNET_MODEL"], "gpt-5.4");
        assert_eq!(r["env"]["ANTHROPIC_DEFAULT_HAIKU_MODEL"], "Qwen3.6");
        assert_eq!(r["env"]["ANTHROPIC_SMALL_FAST_MODEL"], "Qwen3.6");
        assert!(r["env"].get("ANTHROPIC_DEFAULT_FABLE_MODEL").is_none());
    }

    #[test]
    fn to_provider_settings_with_none_creates_fresh() {
        let creds = Credentials {
            base_url: "http://h".into(),
            token: "sk-x".into(),
            group: "default".into(),
            models: vec![],
            model_mapping: ModelMapping::default(),
        };
        let result = to_provider_settings(&creds, None);

        assert_eq!(result["env"]["ANTHROPIC_BASE_URL"], "http://h");
        assert_eq!(result["env"]["ANTHROPIC_AUTH_TOKEN"], "sk-x");
    }
}
