//! Credentials layer: fetch gateway key and model list from the distribution API.

use crate::error::LeproError;

/// Tier → upstream-model mapping supplied by the portal.
///
/// Claude Code exposes fixed tiers (Opus/Sonnet/Haiku/Fable) and a default
/// model. The portal maps each tier to a real gateway model so Claude Code's
/// `/model` picker transparently routes to the distribution API's models.
/// All fields optional + `#[serde(default)]` so the portal can omit any (and
/// older portals that don't return `model_mapping` at all still deserialize).
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct ModelMapping {
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub opus: Option<String>,
    #[serde(default)]
    pub sonnet: Option<String>,
    #[serde(default)]
    pub haiku: Option<String>,
    #[serde(default)]
    pub fable: Option<String>,
}

/// Gateway credentials returned from the distribution API.
///
/// Note: `models` and `group` are for the integrator/UI to display. `base_url`,
/// `token`, and `model_mapping` are consumed when writing the local config.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Credentials {
    pub base_url: String,
    pub token: String,
    pub group: String,
    pub models: Vec<String>,
    #[serde(default)]
    pub model_mapping: ModelMapping,
}

/// Fetch credentials from the distribution API endpoint.
///
/// Sends a GET request to `{portal_base}/api/me/llm-credentials` with Bearer authentication.
///
/// # Parameters
/// - `http`: The reqwest HTTP client.
/// - `portal_base`: The portal base URL.
/// - `access_token`: The OAuth2 access token for Bearer authentication.
///
/// # Errors
/// - `LeproError::Http` on network failure.
/// - `LeproError::Oidc` on non-2xx HTTP status.
/// - `LeproError::Parse` on JSON deserialization failure.
pub async fn fetch(
    http: &reqwest::Client,
    portal_base: &str,
    access_token: &str,
) -> Result<Credentials, LeproError> {
    let url = format!("{}/api/me/llm-credentials", portal_base);

    let resp = http
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| LeproError::Http(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(LeproError::Oidc(format!(
            "credentials endpoint returned {}",
            resp.status()
        )));
    }

    resp.json::<Credentials>()
        .await
        .map_err(|e| LeproError::Parse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fetch_returns_credentials_with_bearer() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/api/me/llm-credentials"))
            .and(wiremock::matchers::header("authorization", "Bearer AT"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "base_url":"http://192.168.33.13:18050/v1","token":"sk-x","group":"default","models":["qwen-max","gpt-5.4"]
            })))
            .mount(&server).await;
        let c = fetch(&reqwest::Client::new(), &server.uri(), "AT")
            .await
            .unwrap();
        assert_eq!(c.token, "sk-x");
        assert_eq!(c.models, vec!["qwen-max", "gpt-5.4"]);
    }

    #[tokio::test]
    async fn fetch_401_is_unauthorized_error() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(wiremock::ResponseTemplate::new(401))
            .mount(&server)
            .await;
        assert!(fetch(&reqwest::Client::new(), &server.uri(), "BAD")
            .await
            .is_err());
    }
}
