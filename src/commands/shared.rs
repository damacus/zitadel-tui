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

    #[test]
    fn resolved_host_uses_cli_arg() {
        let args = Cli::parse_from(["zitadel-tui", "--host", "https://cli.example.com"]);
        let config = AppConfig {
            zitadel_url: Some("https://config.example.com".to_string()),
            ..AppConfig::default()
        };

        let host = resolved_host(&args, &config).unwrap();
        assert_eq!(host, "https://cli.example.com");
    }

    #[test]
    fn resolved_host_falls_back_to_config() {
        let args = Cli::parse_from(["zitadel-tui"]);
        let config = AppConfig {
            zitadel_url: Some("https://config.example.com".to_string()),
            ..AppConfig::default()
        };

        let host = resolved_host(&args, &config).unwrap();
        assert_eq!(host, "https://config.example.com");
    }

    #[test]
    fn resolved_host_cli_arg_takes_precedence_over_config() {
        let args = Cli::parse_from(["zitadel-tui", "--host", "https://cli.example.com"]);
        let config = AppConfig {
            zitadel_url: Some("https://config.example.com".to_string()),
            ..AppConfig::default()
        };

        let host = resolved_host(&args, &config).unwrap();
        assert_eq!(host, "https://cli.example.com");
    }

    #[test]
    fn resolved_host_errors_when_absent() {
        let args = Cli::parse_from(["zitadel-tui"]);
        let error = resolved_host(&args, &AppConfig::default())
            .unwrap_err()
            .to_string();

        assert!(error.contains("Zitadel URL is required"));
    }
}
