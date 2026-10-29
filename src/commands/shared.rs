use serde_json::Value;

use crate::{
    api::{client::ZitadelClient, models::Summarize},
    auth::resolve_access_token,
    cli::Cli,
    config::AppConfig,
    error::CommandError,
};

/// Builds an API client, mapping failures to contract error kinds: a
/// missing Zitadel URL is `Usage`, credential resolution is `Auth`, and
/// anything else falls back to `Api` via `From<anyhow::Error>`.
pub async fn authenticated_client(
    args: &Cli,
    config: &AppConfig,
) -> Result<ZitadelClient, CommandError> {
    let host = resolved_host(args, config)?;
    let http = reqwest::Client::new();
    let auth = resolve_access_token(
        &http,
        &host,
        args.token.clone(),
        args.service_account_file.clone(),
        config,
    )
    .await
    .map_err(|error| CommandError::auth(error.to_string()))?;
    auth.ensure_api_credential()
        .map_err(|error| CommandError::auth(error.to_string()))?;
    ZitadelClient::new(host, auth.token).map_err(CommandError::from)
}

/// Serializes a fetched list for the envelope: compact summary projections
/// by default, each model's verbatim `raw` API payload under `--full`.
/// Single switch for every list command so the flag behaves consistently.
pub fn list_result<T: Summarize>(items: &[T], full: bool) -> Result<Value, CommandError> {
    if full {
        return Ok(serde_json::to_value(
            items.iter().map(T::raw).collect::<Vec<_>>(),
        )?);
    }
    Ok(serde_json::to_value(
        items.iter().map(T::summary).collect::<Vec<_>>(),
    )?)
}

/// Serializes a single resolved item for `get` commands: the compact
/// summary projection by default, the retained raw API payload under
/// `--full` — the same switch [`list_result`] uses.
pub fn get_result<T: Summarize>(item: &T, full: bool) -> Result<Value, CommandError> {
    if full {
        return Ok(item.raw().clone());
    }
    Ok(serde_json::to_value(item.summary())?)
}

pub fn resolved_host(args: &Cli, config: &AppConfig) -> Result<String, CommandError> {
    args.host
        .clone()
        .or_else(|| config.zitadel_url.clone())
        .ok_or_else(|| {
            CommandError::usage("Zitadel URL is required via --host, ZITADEL_URL, or config")
        })
}

pub fn resolved_host_or_cache(args: &Cli, config: &AppConfig) -> Result<String, CommandError> {
    resolved_host(args, config).or_else(|_| {
        crate::token_cache::TokenCache::load()
            .ok()
            .flatten()
            .map(|cache| cache.host)
            .ok_or_else(|| {
                CommandError::usage(
                    "Zitadel URL is required via --host, ZITADEL_URL, config, or a cached login session",
                )
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::{
        env, fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_cache_path() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("zitadel-tui-shared-test-tokens-{unique}.json"))
    }

    fn clear_host_env() -> Option<String> {
        let original = env::var("ZITADEL_URL").ok();
        env::remove_var("ZITADEL_URL");
        original
    }

    fn restore_host_env(original: Option<String>) {
        if let Some(value) = original {
            env::set_var("ZITADEL_URL", value);
        } else {
            env::remove_var("ZITADEL_URL");
        }
    }

    #[test]
    fn resolved_host_uses_cli_arg() {
        let _guard = crate::test_support::env_lock();
        let original = clear_host_env();
        let args = Cli::parse_from(["zitadel-tui", "--host", "https://cli.example.com"]);
        let config = AppConfig {
            zitadel_url: Some("https://config.example.com".to_string()),
            ..AppConfig::default()
        };

        let host = resolved_host(&args, &config).unwrap();
        restore_host_env(original);
        assert_eq!(host, "https://cli.example.com");
    }

    #[test]
    fn resolved_host_falls_back_to_config() {
        let _guard = crate::test_support::env_lock();
        let original = clear_host_env();
        let args = Cli::parse_from(["zitadel-tui"]);
        let config = AppConfig {
            zitadel_url: Some("https://config.example.com".to_string()),
            ..AppConfig::default()
        };

        let host = resolved_host(&args, &config).unwrap();
        restore_host_env(original);
        assert_eq!(host, "https://config.example.com");
    }

    #[test]
    fn resolved_host_cli_arg_takes_precedence_over_config() {
        let _guard = crate::test_support::env_lock();
        let original = clear_host_env();
        let args = Cli::parse_from(["zitadel-tui", "--host", "https://cli.example.com"]);
        let config = AppConfig {
            zitadel_url: Some("https://config.example.com".to_string()),
            ..AppConfig::default()
        };

        let host = resolved_host(&args, &config).unwrap();
        restore_host_env(original);
        assert_eq!(host, "https://cli.example.com");
    }

    #[test]
    fn resolved_host_errors_when_absent() {
        let _guard = crate::test_support::env_lock();
        let original = clear_host_env();
        let args = Cli::parse_from(["zitadel-tui"]);
        let error = resolved_host(&args, &AppConfig::default()).unwrap_err();

        restore_host_env(original);
        assert_eq!(error.kind, crate::error::ErrorKind::Usage);
        assert!(error.to_string().contains("Zitadel URL is required"));
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn authenticated_client_rejects_oidc_session_tokens_for_api_commands() {
        let _guard = crate::test_support::env_lock();
        let original_host = clear_host_env();
        let original_token = env::var("ZITADEL_TOKEN").ok();
        let original_sa = env::var("ZITADEL_SERVICE_ACCOUNT_FILE").ok();
        env::remove_var("ZITADEL_TOKEN");
        env::remove_var("ZITADEL_SERVICE_ACCOUNT_FILE");
        let cache_path = temp_cache_path();
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        crate::token_cache::TokenCache {
            access_token: "header.payload.signature".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            expires_at: Some(now + 3600),
            client_id: "native-client".to_string(),
            host: "https://zitadel.example.com".to_string(),
        }
        .save()
        .unwrap();

        let args = Cli::parse_from(["zitadel-tui", "--host", "https://zitadel.example.com"]);
        let error = match authenticated_client(&args, &AppConfig::default()).await {
            Ok(_) => panic!("expected OIDC session token to be rejected for API commands"),
            Err(error) => error,
        };

        env::remove_var("ZITADEL_TUI_TOKEN_CACHE");
        let _ = fs::remove_file(cache_path);
        restore_host_env(original_host);
        if let Some(token) = original_token {
            env::set_var("ZITADEL_TOKEN", token);
        }
        if let Some(path) = original_sa {
            env::set_var("ZITADEL_SERVICE_ACCOUNT_FILE", path);
        }

        assert_eq!(error.kind, crate::error::ErrorKind::Auth);
        let message = error.to_string();
        assert!(message.contains("requires Zitadel API credentials"));
        assert!(message.contains("auth status"));
        assert!(message.contains("personal access token"));
    }
}
