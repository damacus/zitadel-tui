use anyhow::{bail, Context, Result};
use reqwest::{Client, StatusCode};

const SESSION_TOKEN_REMEDIATION: &str = "OIDC device-login session tokens must be JWT access tokens that Zitadel APIs accept. Create or reconfigure a native app with Device Code and JWT access tokens, then run `auth login` again.";

pub async fn validate_login_session_token(
    client: &Client,
    zitadel_url: &str,
    client_id: &str,
    access_token: &str,
) -> Result<()> {
    ensure_session_token_is_usable("device-login session", access_token, client_id, zitadel_url)?;

    let url = format!("{}/auth/v1/users/me", zitadel_url.trim_end_matches('/'));
    let response = client
        .get(url)
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .send()
        .await
        .context("failed to validate OIDC device-login session token")?;
    let status = response.status();
    let body = response.bytes().await?;

    if status.is_success() {
        return Ok(());
    }

    if status == StatusCode::FORBIDDEN && is_authentication_required_response(&body) {
        bail!(session_token_error(
            "device-login session",
            client_id,
            zitadel_url
        ));
    }

    bail!("session token validation failed ({status})");
}

pub(crate) fn ensure_cached_session_is_usable(
    access_token: &str,
    client_id: &str,
    host: &str,
) -> Result<()> {
    ensure_session_token_is_usable("cached device-login session", access_token, client_id, host)
}

fn ensure_session_token_is_usable(
    label: &str,
    access_token: &str,
    client_id: &str,
    host: &str,
) -> Result<()> {
    if token_looks_like_jwt(access_token) {
        return Ok(());
    }

    bail!(session_token_error(label, client_id, host));
}

pub(crate) fn same_host(left: &str, right: &str) -> bool {
    left.trim_end_matches('/') == right.trim_end_matches('/')
}

fn session_token_error(label: &str, client_id: &str, host: &str) -> String {
    format!(
        "{label} for client `{client_id}` on {host} is not usable for Zitadel APIs. {SESSION_TOKEN_REMEDIATION}"
    )
}

fn token_looks_like_jwt(token: &str) -> bool {
    token.split('.').count() == 3
}

fn is_authentication_required_response(body: &[u8]) -> bool {
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(body) else {
        return false;
    };

    let code_matches = json.get("code").and_then(|value| value.as_i64()) == Some(7);
    let message_matches = json
        .get("message")
        .and_then(|value| value.as_str())
        .map(|message| message.contains("authentication required"))
        .unwrap_or(false);
    let detail_matches = json
        .get("details")
        .and_then(|value| value.as_array())
        .map(|details| {
            details.iter().any(|detail| {
                detail
                    .get("id")
                    .and_then(|value| value.as_str())
                    .map(|id| id == "AUTHZ-Kl3p0")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    code_matches || message_matches || detail_matches
}
