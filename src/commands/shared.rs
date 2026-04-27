use anyhow::{Context, Result};

use crate::{auth::resolve_access_token, cli::Cli, client::ZitadelClient, config::AppConfig};

pub async fn authenticated_client(args: &Cli, config: &AppConfig) -> Result<ZitadelClient> {
    let host = resolved_host(args, config)?;
    let http = reqwest::Client::new();
    let auth = resolve_access_token(
        &http,
        &host,
        args.token.clone(),
        args.service_account_file.clone(),
        config,
    )
    .await?;
    auth.ensure_api_credential()?;
    ZitadelClient::new(host, auth.token)
}

pub fn resolved_host(args: &Cli, config: &AppConfig) -> Result<String> {
    args.host
        .clone()
        .or_else(|| config.zitadel_url.clone())
        .context("Zitadel URL is required via --host, ZITADEL_URL, or config")
}

pub fn resolved_host_or_cache(args: &Cli, config: &AppConfig) -> Result<String> {
    resolved_host(args, config).or_else(|_| {
        crate::token_cache::TokenCache::load()
            .ok()
            .flatten()
            .map(|cache| cache.host)
            .context(
                "Zitadel URL is required via --host, ZITADEL_URL, config, or a cached login session",
            )
    })
}

pub async fn resolved_project_id(
    client: &ZitadelClient,
    args: &Cli,
    config: &AppConfig,
) -> Result<String> {
    if let Some(project_id) = args
        .project_id
        .clone()
        .or_else(|| config.project_id.clone())
    {
        return Ok(project_id);
    }

    client
        .get_default_project()
        .await?
        .get("id")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .context("failed to determine default project id")
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
        let error = resolved_host(&args, &AppConfig::default())
            .unwrap_err()
            .to_string();

        restore_host_env(original);
        assert!(error.contains("Zitadel URL is required"));
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
            Err(error) => error.to_string(),
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

        assert!(error.contains("requires Zitadel API credentials"));
        assert!(error.contains("auth status"));
        assert!(error.contains("personal access token"));
    }
}
