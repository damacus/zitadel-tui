use std::path::PathBuf;

use anyhow::{bail, Result};
use reqwest::Client;

use crate::config::AppConfig;

use super::{
    service_account::exchange_service_account,
    session::{ensure_cached_session_is_usable, same_host},
    ResolvedAuth,
};

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

    if let Ok(Some(cache)) = crate::token_cache::TokenCache::load() {
        if !same_host(&cache.host, zitadel_url) {
            bail!(
                "no credentials available; use --token, --service-account-file, env vars, config, or `auth login`"
            );
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

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Client;
    use std::{
        env,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_cache_path() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("zitadel-tui-test-tokens-{unique}.json"))
    }

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
    async fn resolves_from_valid_token_cache() {
        let cache_path = temp_cache_path();
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);

        let _guard = crate::test_support::env_lock();
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
        let mut server = mockito::Server::new_async().await;
        server
            .mock("POST", "/oauth/v2/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"access_token":"header.payload.signature","expires_in":3600}"#)
            .create_async()
            .await;

        let cache_path = temp_cache_path();
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);

        let _guard = crate::test_support::env_lock();
        let original_token = env::var("ZITADEL_TOKEN").ok();
        let original_sa = env::var("ZITADEL_SERVICE_ACCOUNT_FILE").ok();
        env::remove_var("ZITADEL_TOKEN");
        env::remove_var("ZITADEL_SERVICE_ACCOUNT_FILE");

        let cache = crate::token_cache::TokenCache {
            access_token: "expired-token".to_string(),
            refresh_token: Some("my-refresh-token".to_string()),
            expires_at: Some(0),
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
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);

        let _guard = crate::test_support::env_lock();
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
}
