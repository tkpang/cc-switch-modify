//! OIDC layer: authorize URL construction, loopback callback parsing,
//! token exchange, and token refresh against the lepro_aio `/sso` endpoint.

use crate::error::LeproError;
use url::Url;

/// A parsed OAuth2 loopback callback containing code and state.
#[derive(Debug, PartialEq)]
pub struct Callback {
    pub code: String,
    pub state: String,
}

/// A token set returned from the OIDC token endpoint.
#[derive(serde::Deserialize, Debug)]
pub struct TokenSet {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: i64,
    #[serde(default)]
    pub scope: String,
}

/// Build the authorization URL for the OIDC authorization code flow with PKCE.
///
/// # Parameters
/// - `issuer`: The OIDC issuer base URL (e.g. `http://host/sso`).
/// - `redirect_uri`: The loopback redirect URI.
/// - `challenge`: The S256 code challenge (base64url of SHA-256 of verifier).
/// - `state`: The opaque state value for CSRF protection.
///
/// Returns the full authorization URL as a `String`.
pub fn build_authorize_url(issuer: &str, redirect_uri: &str, challenge: &str, state: &str) -> String {
    let mut url = Url::parse(&format!("{}/authorize", issuer))
        .expect("issuer must be a valid URL");

    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", "lepro-connect")
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", "openid offline_access")
        .append_pair("state", state)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256");

    url.to_string()
}

/// Parse a loopback callback URL and extract the authorization code and state.
///
/// Returns `Err(LeproError::Oidc)` if the `code` parameter is missing.
pub fn parse_callback(url: &str) -> Result<Callback, LeproError> {
    let parsed = Url::parse(url).map_err(|e| LeproError::Parse(e.to_string()))?;

    let mut code: Option<String> = None;
    let mut state: Option<String> = None;

    for (key, value) in parsed.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            _ => {}
        }
    }

    let code = code.ok_or_else(|| LeproError::Oidc("missing 'code' parameter in callback".to_string()))?;
    let state = state.unwrap_or_default();

    Ok(Callback { code, state })
}

/// Exchange an authorization code for a token set.
///
/// POST `{issuer}/token` with form fields per the authorization_code grant.
///
/// # Errors
/// - `LeproError::Http` on network failure.
/// - `LeproError::Oidc` on non-2xx HTTP status.
/// - `LeproError::Parse` on JSON deserialization failure.
pub async fn exchange_code(
    http: &reqwest::Client,
    issuer: &str,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<TokenSet, LeproError> {
    let resp = http
        .post(format!("{issuer}/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", "lepro-connect"),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .map_err(|e| LeproError::Http(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(LeproError::Oidc(format!("token {}", resp.status())));
    }

    resp.json::<TokenSet>()
        .await
        .map_err(|e| LeproError::Parse(e.to_string()))
}

/// Refresh an access token using a refresh token.
///
/// POST `{issuer}/token` with form fields per the refresh_token grant.
///
/// # Errors
/// - `LeproError::Http` on network failure.
/// - `LeproError::Oidc` on non-2xx HTTP status.
/// - `LeproError::Parse` on JSON deserialization failure.
pub async fn refresh(
    http: &reqwest::Client,
    issuer: &str,
    refresh_token: &str,
) -> Result<TokenSet, LeproError> {
    let resp = http
        .post(format!("{issuer}/token"))
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", "lepro-connect"),
        ])
        .send()
        .await
        .map_err(|e| LeproError::Http(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(LeproError::Oidc(format!("token {}", resp.status())));
    }

    resp.json::<TokenSet>()
        .await
        .map_err(|e| LeproError::Parse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_url_has_required_params() {
        let u = build_authorize_url("http://h/sso", "http://127.0.0.1:5/cb", "CH", "ST");
        assert!(u.contains("client_id=lepro-connect"));
        assert!(u.contains("code_challenge=CH"));
        assert!(u.contains("code_challenge_method=S256"));
        assert!(u.contains("scope=openid+offline_access") || u.contains("scope=openid%20offline_access"));
        assert!(u.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A5%2Fcb"));
    }

    #[test]
    fn parse_callback_extracts_code_state() {
        let c = parse_callback("http://127.0.0.1:5/cb?code=abc&state=st").unwrap();
        assert_eq!(c.code, "abc");
        assert_eq!(c.state, "st");
    }

    #[test]
    fn parse_callback_errors_without_code() {
        assert!(parse_callback("http://127.0.0.1:5/cb?state=st").is_err());
    }

    #[tokio::test]
    async fn exchange_code_parses_tokenset() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/sso/token"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token":"at","refresh_token":"rt","token_type":"Bearer",
                "expires_in":3600,"scope":"openid offline_access"
            })))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let ts = exchange_code(
            &http,
            &format!("{}/sso", server.uri()),
            "code",
            "http://127.0.0.1:5/cb",
            "ver",
        )
        .await
        .unwrap();
        assert_eq!(ts.access_token, "at");
        assert_eq!(ts.refresh_token.as_deref(), Some("rt"));
    }

    #[tokio::test]
    async fn refresh_parses_tokenset() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/sso/token"))
            .and(wiremock::matchers::body_string_contains("grant_type=refresh_token"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token":"at2","refresh_token":"rt2","token_type":"Bearer",
                "expires_in":3600,"scope":"openid offline_access"
            })))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let ts = refresh(&http, &format!("{}/sso", server.uri()), "old-rt").await.unwrap();
        assert_eq!(ts.access_token, "at2");
        assert_eq!(ts.refresh_token.as_deref(), Some("rt2"));
    }
}
