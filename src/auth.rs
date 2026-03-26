use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

#[derive(Debug, Clone)]
pub struct ResolvedAuth {
    pub token: String,
    pub source: &'static str,
}

const SESSION_TOKEN_REMEDIATION: &str = "OIDC device-login session tokens must be JWT access tokens that Zitadel APIs accept. Reconfigure the native app to enable Device Code and JWT access tokens, then run `auth login` again, or use `auth logout` to clear the cached session.";

#[derive(Debug, Deserialize)]
struct ServiceAccountKey {
    #[serde(rename = "keyId")]
    key_id: String,
    #[serde(rename = "userId")]
    user_id: String,
    key: String,
}

#[derive(Debug, Serialize)]
struct Claims {
    iss: String,
    sub: String,
    aud: String,
    iat: usize,
    exp: usize,
}

pub async fn resolve_access_token(
    client: &Client,
    zitadel_url: &str,
    cli_pat: Option<String>,
    cli_service_account_file: Option<PathBuf>,
    config: &AppConfig,
) -> Result<ResolvedAuth> {
    if let Some(pat) = cli_pat {
        return Ok(ResolvedAuth {
            token: pat,
            source: "cli PAT",
        });
    }

    if let Ok(pat) = std::env::var("ZITADEL_TOKEN") {
        if !pat.trim().is_empty() {
            return Ok(ResolvedAuth {
                token: pat,
                source: "env PAT",
            });
        }
    }

    if let Some(pat) = config.pat.clone() {
        return Ok(ResolvedAuth {
            token: pat,
            source: "config PAT",
        });
    }

    if let Some(path) = cli_service_account_file {
        let token = exchange_service_account(client, zitadel_url, path).await?;
        return Ok(ResolvedAuth {
            token,
            source: "cli service account",
        });
    }

    if let Ok(path) = std::env::var("ZITADEL_SERVICE_ACCOUNT_FILE") {
        let token = exchange_service_account(client, zitadel_url, PathBuf::from(path)).await?;
        return Ok(ResolvedAuth {
            token,
            source: "env service account",
        });
    }

    if let Some(path) = config.service_account_file.clone() {
        let token = exchange_service_account(client, zitadel_url, path).await?;
        return Ok(ResolvedAuth {
            token,
            source: "config service account",
        });
    }

    // Token cache (OIDC session) — with auto-refresh
    if let Ok(Some(cache)) = crate::token_cache::TokenCache::load() {
        if !same_host(&cache.host, zitadel_url) {
            bail!("no credentials available; use --token, --service-account-file, env vars, config, or `auth login`");
        }
        if !cache.is_expired() {
            ensure_cached_session_is_usable(&cache.access_token, &cache.client_id, &cache.host)?;
            return Ok(ResolvedAuth {
                token: cache.access_token,
                source: "session token",
            });
        }
        if let Some(refresh_token) = &cache.refresh_token {
            if let Ok(tokens) = crate::oidc::refresh_access_token(
                client,
                &cache.host,
                &cache.client_id,
                refresh_token,
            )
            .await
            {
                ensure_cached_session_is_usable(
                    &tokens.access_token,
                    &cache.client_id,
                    &cache.host,
                )?;
                let updated = crate::token_cache::TokenCache {
                    access_token: tokens.access_token.clone(),
                    refresh_token: tokens.refresh_token.or_else(|| Some(refresh_token.clone())),
                    expires_at: Some(crate::oidc::expires_at_from_now(tokens.expires_in)),
                    client_id: cache.client_id,
                    host: cache.host,
                };
                let _ = updated.save();
                return Ok(ResolvedAuth {
                    token: tokens.access_token,
                    source: "session token (refreshed)",
                });
            }
        }
    }

    bail!("no credentials available; use --token, --service-account-file, env vars, config, or `auth login`")
}

pub async fn validate_login_session_token(
    client: &Client,
    zitadel_url: &str,
    client_id: &str,
    access_token: &str,
) -> Result<()> {
    ensure_cached_session_is_usable(access_token, client_id, zitadel_url)?;

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
        bail!(session_token_error(client_id, zitadel_url));
    }

    bail!("session token validation failed ({status})");
}

fn ensure_cached_session_is_usable(access_token: &str, client_id: &str, host: &str) -> Result<()> {
    if token_looks_like_jwt(access_token) {
        return Ok(());
    }

    bail!(session_token_error(client_id, host));
}

fn session_token_error(client_id: &str, host: &str) -> String {
    format!(
        "cached device-login session for client `{client_id}` on {host} is not usable for Zitadel APIs. {SESSION_TOKEN_REMEDIATION}"
    )
}

fn token_looks_like_jwt(token: &str) -> bool {
    token.split('.').count() == 3
}

fn same_host(left: &str, right: &str) -> bool {
    left.trim_end_matches('/') == right.trim_end_matches('/')
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

async fn exchange_service_account(
    client: &Client,
    zitadel_url: &str,
    path: PathBuf,
) -> Result<String> {
    let contents =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let key: ServiceAccountKey = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let jwt = create_jwt(&key, zitadel_url)?;

    let token_url = format!("{}/oauth/v2/token", zitadel_url.trim_end_matches('/'));
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("scope", "openid urn:zitadel:iam:org:project:id:zitadel:aud"),
        ("assertion", jwt.as_str()),
    ];

    let response = client.post(token_url).form(&params).send().await?;
    let status = response.status();
    let body = response.bytes().await?;
    if !status.is_success() {
        bail!("service-account token exchange failed ({status})");
    }

    let body: serde_json::Value =
        serde_json::from_slice(&body).context("failed to decode service-account token response")?;
    body.get("access_token")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("token response missing access_token"))
}

fn create_jwt(key: &ServiceAccountKey, audience: &str) -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock before unix epoch")?
        .as_secs() as usize;

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(key.key_id.clone());

    let claims = Claims {
        iss: key.user_id.clone(),
        sub: key.user_id.clone(),
        aud: audience.to_string(),
        iat: now,
        exp: now + 3600,
    };

    encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(key.key.as_bytes()).context("invalid RSA private key")?,
    )
    .context("failed to sign JWT")
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use std::{
        env, fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_cache_path() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("zitadel-tui-test-tokens-{unique}.json"))
    }

    fn temp_cache_path() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("zitadel-tui-test-tokens-{unique}.json"))
    }

    fn temp_file(name: &str, contents: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("zitadel-tui-{name}-{unique}.json"));
        fs::write(&path, contents).unwrap();
        path
    }

    const TEST_PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCqgMTyfrfQTZd/
wndpcfCBDfG919Zi+Lz6H3MI48BuOT4T0MeTkjRMyY+gSWcOxwvtlMsQoB41xHeW
/fVrFKqgbsEZRoH5ZxAoN7+pNnfEfCXBhtLbs2Oi6Gt+TTCE9utv2qoXUkHCys82
Y4kZqddDo3AYm4lHXgyyfadiPoXiZx38XeiD2BLNIKglBhCrRLA2CU5l3rFhW32F
9rAyfGg00pQoehOLdtfJ0u1ZDH6rXa+FVIW5MBhCpUR4LvIePh3gT2W85QrRuTV2
rYgu6OlAIUsQODEvZx89KrqGMFVTe41Y1icXbUerebRhfQtgvR6c5LErkpoWfXbq
bhJjnqodAgMBAAECggEAHZ2bWAGhvPlVSNhG9JZZb5kz9cVBVFSfQpTm3tLskFi6
CygXGm9pTTMvkuhdEciKLlzLftpJFQ7ItP3svIpM7uv9931zQxZfTJUOYf53hDYK
OtjH1GiO9HOJhFk1Bct77qRdKgrcKFEg9/IHFOGW5gVECcouaKqR6wj/Y4W3rLDy
9ijNJBDqtWRWjsHsba8q9dNa6UMTqoCXSrZ5AfEGMbR9wmbTPRTip8meiZzZdgRK
51LTr7bFPJFRFm8g96lbWRsIn4NKdpSW33jzvlxedGIJLV4LMka1oBTyYM4tWVMs
4aA1/PtfouCBm98wYm2SPU019D5E9v2M6lphYkYREwKBgQDZV42qh5lfYCL+S6vc
n1kFYC58p3+/Mi84LXZYoORvCH5EGLwv0ftM2uT0amlv3Lcic23hf3SVtGRobo5n
wssT6P/LXiP9dEpcDKziAXjouXvm4fSDUvEgmXlaGfQmR2LfEIug/qusvZQq/F5p
hKb2xuRfQzitsZpxA4Pot5DcawKBgQDI1HMnmc+FmRzVfHaXUgnD406heBJOYtie
GgtW9qVW/aZf8N/G36eDlj0/u4uRFlPcp6Ad427eoMWHLqCcMgpXtdZ5SckMJ0pR
RCKxMJ37/SOhV0R8v/sp5q9gRVy1z4iomON0oUElqukuIJzG3Lp86dwhQR7OpZ7C
wMGIGT+1lwKBgQDRJbnD8n0bFM5X28Xkpsrpq2bQufbqrIZYDxelrh5k4s1vBkaB
1hV4HeTZd1VDOihZVK7Wouoz7cX54PnUy9TUshEFSPBlRHUSI3hyfGw3t9aNlb7Q
aQ51CnuGwxb5hxSUB732DVxy/HQK8ZSBAhARxc+aBHwUWaZ/Ppy/Y3ZZRQKBgQC1
dcDW91NbE43KGDvPXoEUTj6uByADU60GreGxIgsjHu1Fow+PUma5rvaIr5zb66C6
r3sthmKXJg1Up/zXJR/TQKoZzWGraZTs+POfxp35IjEfhwqK7ayzn4y3H/U1EeAY
9owOxeVnc5Zd53nA9ZBLbcNJCN4dOejJcAFuR+IY4QKBgEEIjBV+z/Nz6DInO8OD
zARHgiLinSZh85rgNytB9UYXvbrOwKwd7hQDbxXLKqCx1f52Y1TdsWq7H3ST8+YO
dfy4Xqt23aNLgPpM7pQa8J88yq7+YrwlGynnTiDUNoHQqmzDHjKlWIT4bQONX5aO
ehuWWRLZbrtEDcwsUeaYjDGj
-----END PRIVATE KEY-----
"#;

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn cli_pat_wins_over_env_and_config() {
        let _guard = crate::test_support::env_lock();
        let original_env = env::var("ZITADEL_TOKEN").ok();
        env::set_var("ZITADEL_TOKEN", "env-token");

        let config = AppConfig {
            pat: Some("config-token".to_string()),
            ..AppConfig::default()
        };
        let http = Client::new();
        let auth = resolve_access_token(
            &http,
            "https://zitadel.example.com",
            Some("cli-token".to_string()),
            None,
            &config,
        )
        .await
        .unwrap();

        if let Some(value) = original_env {
            env::set_var("ZITADEL_TOKEN", value);
        } else {
            env::remove_var("ZITADEL_TOKEN");
        }

        assert_eq!(auth.token, "cli-token");
        assert_eq!(auth.source, "cli PAT");
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn blank_env_token_falls_back_to_config_pat() {
        let _guard = crate::test_support::env_lock();
        let original_env = env::var("ZITADEL_TOKEN").ok();
        env::set_var("ZITADEL_TOKEN", "   ");
        env::remove_var("ZITADEL_SERVICE_ACCOUNT_FILE");

        let config = AppConfig {
            pat: Some("config-token".to_string()),
            ..AppConfig::default()
        };
        let http = Client::new();
        let auth = resolve_access_token(&http, "https://zitadel.example.com", None, None, &config)
            .await
            .unwrap();

        if let Some(value) = original_env {
            env::set_var("ZITADEL_TOKEN", value);
        } else {
            env::remove_var("ZITADEL_TOKEN");
        }

        assert_eq!(auth.token, "config-token");
        assert_eq!(auth.source, "config PAT");
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn exchanges_service_account_and_returns_access_token() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/oauth/v2/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token":"jwt-exchanged-token"}"#)
            .create_async()
            .await;

        let _guard = crate::test_support::env_lock();
        let original_token = env::var("ZITADEL_TOKEN").ok();
        let original_service_account = env::var("ZITADEL_SERVICE_ACCOUNT_FILE").ok();
        env::remove_var("ZITADEL_TOKEN");
        env::remove_var("ZITADEL_SERVICE_ACCOUNT_FILE");
        let server_url = server.url();
        let config = AppConfig::default();

        let key_path = temp_file(
            "service-account",
            &serde_json::json!({
                "keyId": "kid-1",
                "userId": "user-1",
                "key": TEST_PRIVATE_KEY
            })
            .to_string(),
        );
        let http = Client::new();
        let auth = resolve_access_token(&http, &server_url, None, Some(key_path), &config)
            .await
            .unwrap();

        if let Some(value) = original_token {
            env::set_var("ZITADEL_TOKEN", value);
        }
        if let Some(value) = original_service_account {
            env::set_var("ZITADEL_SERVICE_ACCOUNT_FILE", value);
        }

        mock.assert_async().await;
        assert_eq!(auth.token, "jwt-exchanged-token");
        assert_eq!(auth.source, "cli service account");
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn missing_access_token_is_rejected() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/oauth/v2/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create_async()
            .await;

        let _guard = crate::test_support::env_lock();
        let original_token = env::var("ZITADEL_TOKEN").ok();
        let original_service_account = env::var("ZITADEL_SERVICE_ACCOUNT_FILE").ok();
        env::remove_var("ZITADEL_TOKEN");
        env::remove_var("ZITADEL_SERVICE_ACCOUNT_FILE");
        let server_url = server.url();
        let config = AppConfig::default();

        let key_path = temp_file(
            "service-account-missing-token",
            &serde_json::json!({
                "keyId": "kid-1",
                "userId": "user-1",
                "key": TEST_PRIVATE_KEY
            })
            .to_string(),
        );
        let http = Client::new();
        let error = resolve_access_token(&http, &server_url, None, Some(key_path), &config)
            .await
            .unwrap_err()
            .to_string();

        if let Some(value) = original_token {
            env::set_var("ZITADEL_TOKEN", value);
        }
        if let Some(value) = original_service_account {
            env::set_var("ZITADEL_SERVICE_ACCOUNT_FILE", value);
        }

        mock.assert_async().await;
        assert!(error.contains("missing access_token"));
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn resolves_from_valid_token_cache() {
        let cache_path = temp_cache_path();
        let _guard = crate::test_support::env_lock();
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);
        let original_token = env::var("ZITADEL_TOKEN").ok();
        let original_sa = env::var("ZITADEL_SERVICE_ACCOUNT_FILE").ok();
        env::remove_var("ZITADEL_TOKEN");
        env::remove_var("ZITADEL_SERVICE_ACCOUNT_FILE");

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let cache = crate::token_cache::TokenCache {
            access_token: "header.payload.signature".to_string(),
            refresh_token: None,
            expires_at: Some(now + 3600),
            client_id: "c1".to_string(),
            host: "https://zitadel.example.com".to_string(),
        };
        cache.save().unwrap();

        let http = Client::new();
        let config = AppConfig::default();
        let auth = resolve_access_token(&http, "https://zitadel.example.com", None, None, &config)
            .await
            .unwrap();

        env::remove_var("ZITADEL_TUI_TOKEN_CACHE");
        let _ = std::fs::remove_file(&cache_path);
        if let Some(v) = original_token {
            env::set_var("ZITADEL_TOKEN", v);
        }
        if let Some(v) = original_sa {
            env::set_var("ZITADEL_SERVICE_ACCOUNT_FILE", v);
        }

        assert_eq!(auth.token, "header.payload.signature");
        assert_eq!(auth.source, "session token");
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn auto_refreshes_expired_token_cache() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/oauth/v2/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token":"header.payload.signature","expires_in":3600}"#)
            .create_async()
            .await;

        let cache_path = temp_cache_path();
        let _guard = crate::test_support::env_lock();
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);
        let original_token = env::var("ZITADEL_TOKEN").ok();
        let original_sa = env::var("ZITADEL_SERVICE_ACCOUNT_FILE").ok();
        env::remove_var("ZITADEL_TOKEN");
        env::remove_var("ZITADEL_SERVICE_ACCOUNT_FILE");

        let cache = crate::token_cache::TokenCache {
            access_token: "expired-token".to_string(),
            refresh_token: Some("my-refresh-token".to_string()),
            expires_at: Some(0), // expired
            client_id: "c1".to_string(),
            host: server.url(),
        };
        cache.save().unwrap();

        let http = Client::new();
        let config = AppConfig::default();
        let auth = resolve_access_token(&http, &server.url(), None, None, &config)
            .await
            .unwrap();

        env::remove_var("ZITADEL_TUI_TOKEN_CACHE");
        let _ = std::fs::remove_file(&cache_path);
        if let Some(v) = original_token {
            env::set_var("ZITADEL_TOKEN", v);
        }
        if let Some(v) = original_sa {
            env::set_var("ZITADEL_SERVICE_ACCOUNT_FILE", v);
        }

        assert_eq!(auth.token, "header.payload.signature");
        assert!(auth.source.contains("refreshed"));
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn rejects_cached_opaque_device_session_tokens() {
        let cache_path = temp_cache_path();
        let _guard = crate::test_support::env_lock();
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);
        let original_token = env::var("ZITADEL_TOKEN").ok();
        let original_sa = env::var("ZITADEL_SERVICE_ACCOUNT_FILE").ok();
        env::remove_var("ZITADEL_TOKEN");
        env::remove_var("ZITADEL_SERVICE_ACCOUNT_FILE");

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let cache = crate::token_cache::TokenCache {
            access_token: "opaque-token".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            expires_at: Some(now + 3600),
            client_id: "c1".to_string(),
            host: "https://zitadel.example.com".to_string(),
        };
        cache.save().unwrap();

        let http = Client::new();
        let config = AppConfig::default();
        let error = resolve_access_token(&http, "https://zitadel.example.com", None, None, &config)
            .await
            .unwrap_err()
            .to_string();

        env::remove_var("ZITADEL_TUI_TOKEN_CACHE");
        let _ = std::fs::remove_file(&cache_path);
        if let Some(v) = original_token {
            env::set_var("ZITADEL_TOKEN", v);
        }
        if let Some(v) = original_sa {
            env::set_var("ZITADEL_SERVICE_ACCOUNT_FILE", v);
        }

        assert!(error.contains("JWT access tokens"));
        assert!(error.contains("auth login"));
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn token_exchange_errors_do_not_echo_response_body() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/oauth/v2/token")
            .with_status(500)
            .with_header("content-type", "application/json")
            .with_body(r#"{"client_secret":"leaked"}"#)
            .create_async()
            .await;

        let _guard = crate::test_support::env_lock();
        let original_token = env::var("ZITADEL_TOKEN").ok();
        let original_service_account = env::var("ZITADEL_SERVICE_ACCOUNT_FILE").ok();
        env::remove_var("ZITADEL_TOKEN");
        env::remove_var("ZITADEL_SERVICE_ACCOUNT_FILE");
        let server_url = server.url();
        let config = AppConfig::default();

        let key_path = temp_file(
            "service-account-error",
            &serde_json::json!({
                "keyId": "kid-1",
                "userId": "user-1",
                "key": TEST_PRIVATE_KEY
            })
            .to_string(),
        );
        let http = Client::new();
        let error = resolve_access_token(&http, &server_url, None, Some(key_path), &config)
            .await
            .unwrap_err()
            .to_string();

        if let Some(value) = original_token {
            env::set_var("ZITADEL_TOKEN", value);
        }
        if let Some(value) = original_service_account {
            env::set_var("ZITADEL_SERVICE_ACCOUNT_FILE", value);
        }

        mock.assert_async().await;
        assert!(error.contains("service-account token exchange failed"));
        assert!(!error.contains("leaked"));
    }
}
