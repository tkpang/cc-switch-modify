//! Credentials layer: fetch gateway key and model list from the distribution API.

use crate::error::LeproError;

/// Gateway credentials returned from the distribution API.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Credentials {
    pub base_url: String,
    pub token: String,
    pub group: String,
    pub models: Vec<String>,
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
        return Err(LeproError::Oidc("unauthorized".to_string()));
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
        let c = fetch(&reqwest::Client::new(), &server.uri(), "AT").await.unwrap();
        assert_eq!(c.token, "sk-x");
        assert_eq!(c.models, vec!["qwen-max","gpt-5.4"]);
    }

    #[tokio::test]
    async fn fetch_401_is_unauthorized_error() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::any())
            .respond_with(wiremock::ResponseTemplate::new(401)).mount(&server).await;
        assert!(fetch(&reqwest::Client::new(), &server.uri(), "BAD").await.is_err());
    }
}
