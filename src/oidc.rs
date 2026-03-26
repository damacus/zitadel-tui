use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DeviceAuthResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    /// Minimum polling interval in seconds.
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 {
    5
}

#[derive(Debug)]
pub struct OidcTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
}

/// Returned by poll_for_token when not yet approved.
#[derive(Debug)]
pub enum PollError {
    /// User hasn't approved yet — keep polling.
    Pending,
    /// Slow down — increase interval.
    SlowDown,
    /// Fatal error.
    Fatal(anyhow::Error),
}

impl From<anyhow::Error> for PollError {
    fn from(e: anyhow::Error) -> Self {
        PollError::Fatal(e)
    }
}

const SCOPES: &str =
    "openid profile email offline_access urn:zitadel:iam:org:project:id:zitadel:aud";

pub async fn device_authorize(
    client: &Client,
    host: &str,
    client_id: &str,
) -> Result<DeviceAuthResponse> {
    let url = format!(
        "{}/oauth/v2/device_authorization",
        host.trim_end_matches('/')
    );
    let params = [("client_id", client_id), ("scope", SCOPES)];
    let resp = client.post(&url).form(&params).send().await?;
    let status = resp.status();
    let body = resp.bytes().await?;
    if !status.is_success() {
        anyhow::bail!("device authorization request failed ({status})");
    }
    serde_json::from_slice(&body).context("failed to parse device authorization response")
}

pub async fn poll_for_token(
    client: &Client,
    host: &str,
    client_id: &str,
    device_code: &str,
) -> std::result::Result<OidcTokens, PollError> {
    let url = format!("{}/oauth/v2/token", host.trim_end_matches('/'));
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("client_id", client_id),
        ("device_code", device_code),
    ];
    let resp = client
        .post(&url)
        .form(&params)
        .send()
        .await
        .context("poll request failed")?;
    let status = resp.status();
    let body = resp.bytes().await.context("failed to read poll response")?;

    if status.is_success() {
        return parse_tokens(&body).map_err(PollError::Fatal);
    }

    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
    match json.get("error").and_then(|e| e.as_str()) {
        Some("authorization_pending") => Err(PollError::Pending),
        Some("slow_down") => Err(PollError::SlowDown),
        Some(other) => Err(PollError::Fatal(anyhow!("device poll error: {other}"))),
        None => Err(PollError::Fatal(anyhow!("device poll failed ({status})"))),
    }
}

pub async fn refresh_access_token(
    client: &Client,
    host: &str,
    client_id: &str,
    refresh_token: &str,
) -> Result<OidcTokens> {
    let url = format!("{}/oauth/v2/token", host.trim_end_matches('/'));
    let params = [
        ("grant_type", "refresh_token"),
        ("client_id", client_id),
        ("refresh_token", refresh_token),
    ];
    let resp = client.post(&url).form(&params).send().await?;
    let status = resp.status();
    let body = resp.bytes().await?;
    if !status.is_success() {
        anyhow::bail!("token refresh failed ({status})");
    }
    parse_tokens(&body)
}

fn parse_tokens(body: &[u8]) -> Result<OidcTokens> {
    let json: serde_json::Value =
        serde_json::from_slice(body).context("failed to parse token response")?;
    let access_token = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("token response missing access_token"))?;
    Ok(OidcTokens {
        access_token,
        refresh_token: json
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        expires_in: json
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600),
    })
}

/// Compute the unix timestamp when a token will expire,
/// with a 30-second safety margin.
pub fn expires_at_from_now(expires_in: u64) -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now + expires_in.saturating_sub(30)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn device_authorize_returns_codes() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/oauth/v2/device_authorization")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "device_code": "dev-code-1",
                "user_code": "ABCD-1234",
                "verification_uri": "https://example.com/activate",
                "verification_uri_complete": "https://example.com/activate?user_code=ABCD-1234",
                "expires_in": 300,
                "interval": 5
            }"#,
            )
            .create_async()
            .await;

        let http = reqwest::Client::new();
        let resp = device_authorize(&http, &server.url(), "client-1")
            .await
            .unwrap();
        assert_eq!(resp.device_code, "dev-code-1");
        assert_eq!(resp.user_code, "ABCD-1234");
        assert_eq!(resp.interval, 5);
    }

    #[tokio::test]
    async fn poll_returns_tokens_on_success() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/oauth/v2/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "access_token": "acc-tok",
                "refresh_token": "ref-tok",
                "expires_in": 3600
            }"#,
            )
            .create_async()
            .await;

        let http = reqwest::Client::new();
        let tokens = poll_for_token(&http, &server.url(), "client-1", "dev-code-1")
            .await
            .unwrap();
        assert_eq!(tokens.access_token, "acc-tok");
        assert_eq!(tokens.refresh_token.as_deref(), Some("ref-tok"));
        assert_eq!(tokens.expires_in, 3600);
    }

    #[tokio::test]
    async fn refresh_returns_new_access_token() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/oauth/v2/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token": "new-acc", "expires_in": 3600}"#)
            .create_async()
            .await;

        let http = reqwest::Client::new();
        let tokens = refresh_access_token(&http, &server.url(), "client-1", "ref-tok")
            .await
            .unwrap();
        assert_eq!(tokens.access_token, "new-acc");
    }

    #[tokio::test]
    async fn poll_returns_pending_error_on_authorization_pending() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/oauth/v2/token")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "authorization_pending"}"#)
            .create_async()
            .await;

        let http = reqwest::Client::new();
        let err = poll_for_token(&http, &server.url(), "client-1", "dev-code-1")
            .await
            .unwrap_err();
        assert!(matches!(err, PollError::Pending));
    }
}
