use anyhow::Result;
use serde_json::Value;

use crate::{
    auth::{resolve_access_token, validate_login_session_token},
    cli::{AuthAction, AuthCommand, Cli},
    client::ZitadelClient,
    config::AppConfig,
    oidc, token_cache,
};

use super::shared::{resolved_host, resolved_host_or_cache};

pub async fn execute_auth_command(
    command: &AuthCommand,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value> {
    match &command.action {
        AuthAction::Login(login_args) => {
            let host = resolved_host(args, config)?;
            let client_id = if let Some(id) = &login_args.client_id {
                id.clone()
            } else if let Some(id) = &config.device_client_id {
                id.clone()
            } else {
                let id = prompt_for_client_id()?;
                let mut updated = config.clone();
                updated.device_client_id = Some(id.clone());
                let _ = updated.save_to_canonical_path();
                id
            };

            let http = reqwest::Client::new();
            let auth_resp = oidc::device_authorize(&http, &host, &client_id).await?;

            eprintln!("\nOpen this URL in your browser:");
            eprintln!(
                "  {}",
                auth_resp
                    .verification_uri_complete
                    .as_deref()
                    .unwrap_or(&auth_resp.verification_uri)
            );
            eprintln!(
                "\nOr go to {} and enter code: {}\n",
                auth_resp.verification_uri, auth_resp.user_code
            );

            let mut interval = auth_resp.interval;
            let tokens = loop {
                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                match oidc::poll_for_token(&http, &host, &client_id, &auth_resp.device_code).await {
                    Ok(tokens) => break tokens,
                    Err(oidc::PollError::Pending) => {
                        eprint!(".");
                    }
                    Err(oidc::PollError::SlowDown) => {
                        interval += 5;
                        eprint!(".");
                    }
                    Err(oidc::PollError::Fatal(error)) => return Err(error),
                }
            };
            eprintln!("\nAuthenticated.");

            persist_login_session(&http, &host, &client_id, tokens).await?;

            Ok(serde_json::json!({
                "status": "authenticated",
                "host": host,
                "token_cache": token_cache::TokenCache::path()?.display().to_string(),
            }))
        }
        AuthAction::Logout => {
            token_cache::TokenCache::clear()?;
            Ok(serde_json::json!({ "status": "logged out" }))
        }
        AuthAction::Status => {
            let host = resolved_host_or_cache(args, config)?;
            let http = reqwest::Client::new();
            let auth = resolve_access_token(
                &http,
                &host,
                args.token.clone(),
                args.service_account_file.clone(),
                config,
            )
            .await?;
            let client = ZitadelClient::new(host.clone(), auth.token)?;
            let me = client.whoami().await?;
            let (user_id, login_name) = auth_status_identity(&me);
            Ok(serde_json::json!({
                "host": host,
                "auth_source": auth.source,
                "user_id": user_id,
                "login_name": login_name,
            }))
        }
    }
}

fn auth_status_identity(me: &Value) -> (Value, Value) {
    let user = me.get("user").unwrap_or(&Value::Null);
    let user_id = user
        .get("userId")
        .or_else(|| user.get("id"))
        .cloned()
        .unwrap_or(Value::Null);
    let login_name = user
        .get("preferredLoginName")
        .or_else(|| user.get("userName"))
        .or_else(|| user.get("loginName"))
        .cloned()
        .unwrap_or(Value::Null);

    (user_id, login_name)
}

fn prompt_for_client_id() -> Result<String> {
    eprint!("Zitadel native app client ID: ");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let id = input.trim().to_string();
    if id.is_empty() {
        anyhow::bail!("client ID cannot be empty");
    }
    Ok(id)
}

pub async fn persist_login_session(
    http: &reqwest::Client,
    host: &str,
    client_id: &str,
    tokens: oidc::OidcTokens,
) -> Result<()> {
    validate_login_session_token(http, host, client_id, &tokens.access_token).await?;

    let cache = token_cache::TokenCache {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_at: Some(oidc::expires_at_from_now(tokens.expires_in)),
        client_id: client_id.to_string(),
        host: host.to_string(),
    };
    cache.save()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::Parser;
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
        env::temp_dir().join(format!("zitadel-tui-main-test-tokens-{unique}.json"))
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn persist_login_session_saves_cache_after_successful_userinfo_probe() {
        let _guard = crate::test_support::env_lock();
        let cache_path = temp_cache_path();
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);

        let mut server = Server::new_async().await;
        let userinfo_probe = server
            .mock("GET", "/oidc/v1/userinfo")
            .match_header("authorization", "Bearer header.payload.signature")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"sub":"u1","email":"admin@example.com"}"#)
            .create_async()
            .await;

        let http = reqwest::Client::new();
        persist_login_session(
            &http,
            &server.url(),
            "native-app-id",
            oidc::OidcTokens {
                access_token: "header.payload.signature".to_string(),
                refresh_token: Some("refresh-token".to_string()),
                expires_in: 3600,
            },
        )
        .await
        .unwrap();

        userinfo_probe.assert_async().await;
        let cache = crate::token_cache::TokenCache::load()
            .unwrap()
            .expect("cache entry");
        assert_eq!(cache.access_token, "header.payload.signature");
        assert_eq!(cache.refresh_token.as_deref(), Some("refresh-token"));
        assert_eq!(cache.client_id, "native-app-id");
        assert_eq!(cache.host, server.url());

        env::remove_var("ZITADEL_TUI_TOKEN_CACHE");
        let _ = fs::remove_file(cache_path);
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn persist_login_session_allows_userinfo_valid_jwt_when_auth_api_rejects_it() {
        let _guard = crate::test_support::env_lock();
        let cache_path = temp_cache_path();
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);

        let mut server = Server::new_async().await;
        let userinfo_probe = server
            .mock("GET", "/oidc/v1/userinfo")
            .match_header("authorization", "Bearer header.payload.signature")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"sub":"355235054814759983","email":"dan.m.webb@gmail.com"}"#)
            .create_async()
            .await;

        let auth_api_probe = server
            .mock("GET", "/auth/v1/users/me")
            .match_header("authorization", "Bearer header.payload.signature")
            .with_status(403)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "code": 7,
                    "message": "authentication required",
                    "details": [{"id": "AUTHZ-Kl3p0"}]
                }"#,
            )
            .expect(0)
            .create_async()
            .await;

        let http = reqwest::Client::new();
        persist_login_session(
            &http,
            &server.url(),
            "native-app-id",
            oidc::OidcTokens {
                access_token: "header.payload.signature".to_string(),
                refresh_token: Some("refresh-token".to_string()),
                expires_in: 3600,
            },
        )
        .await
        .unwrap();

        userinfo_probe.assert_async().await;
        auth_api_probe.assert_async().await;
        assert!(crate::token_cache::TokenCache::load().unwrap().is_some());

        env::remove_var("ZITADEL_TUI_TOKEN_CACHE");
        let _ = fs::remove_file(cache_path);
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn persist_login_session_rejects_unusable_device_tokens_without_writing_cache() {
        let _guard = crate::test_support::env_lock();
        let cache_path = temp_cache_path();
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);

        let http = reqwest::Client::new();
        let error = persist_login_session(
            &http,
            "https://zitadel.example.com",
            "native-app-id",
            oidc::OidcTokens {
                access_token: "opaque-token".to_string(),
                refresh_token: Some("refresh-token".to_string()),
                expires_in: 3600,
            },
        )
        .await
        .unwrap_err()
        .to_string();

        assert!(error.contains("JWT access tokens"));
        assert!(crate::token_cache::TokenCache::load().unwrap().is_none());

        env::remove_var("ZITADEL_TUI_TOKEN_CACHE");
        let _ = fs::remove_file(cache_path);
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn persist_login_session_rejects_userinfo_probe_failures_without_writing_cache() {
        let _guard = crate::test_support::env_lock();
        let cache_path = temp_cache_path();
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);

        let mut server = Server::new_async().await;
        let userinfo_probe = server
            .mock("GET", "/oidc/v1/userinfo")
            .match_header("authorization", "Bearer header.payload.signature")
            .with_status(401)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message":"bad token"}"#)
            .create_async()
            .await;

        let http = reqwest::Client::new();
        let error = persist_login_session(
            &http,
            &server.url(),
            "native-app-id",
            oidc::OidcTokens {
                access_token: "header.payload.signature".to_string(),
                refresh_token: Some("refresh-token".to_string()),
                expires_in: 3600,
            },
        )
        .await
        .unwrap_err()
        .to_string();

        userinfo_probe.assert_async().await;
        assert!(error.contains("OIDC userinfo validation failed"));
        assert!(crate::token_cache::TokenCache::load().unwrap().is_none());

        env::remove_var("ZITADEL_TUI_TOKEN_CACHE");
        let _ = fs::remove_file(cache_path);
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn auth_status_uses_valid_cached_session_tokens() {
        let _guard = crate::test_support::env_lock();
        let original_host = env::var("ZITADEL_URL").ok();
        env::remove_var("ZITADEL_URL");
        let cache_path = temp_cache_path();
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);

        let mut server = Server::new_async().await;
        let _whoami = server
            .mock("GET", "/auth/v1/users/me")
            .match_header("authorization", "Bearer header.payload.signature")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"user":{"userId":"u-123","preferredLoginName":"admin@example.com"}}"#)
            .create_async()
            .await;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let cache = crate::token_cache::TokenCache {
            access_token: "header.payload.signature".to_string(),
            refresh_token: None,
            expires_at: Some(now + 3600),
            client_id: "native-app-id".to_string(),
            host: server.url(),
        };
        cache.save().unwrap();

        let args = Cli::parse_from(["zitadel-tui", "--once", "auth", "status"]);
        let command = match &args.command {
            Some(crate::cli::Command::Auth(command)) => command.clone(),
            other => panic!("unexpected command: {other:?}"),
        };

        let result = execute_auth_command(&command, &args, &AppConfig::default())
            .await
            .unwrap();

        assert_eq!(result["host"], server.url());
        assert_eq!(result["auth_source"], "session token");
        assert_eq!(result["user_id"], "u-123");
        assert_eq!(result["login_name"], "admin@example.com");

        env::remove_var("ZITADEL_TUI_TOKEN_CACHE");
        let _ = fs::remove_file(cache_path);
        if let Some(host) = original_host {
            env::set_var("ZITADEL_URL", host);
        }
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn auth_status_extracts_machine_user_id_from_id_field() {
        let _guard = crate::test_support::env_lock();
        let original_host = env::var("ZITADEL_URL").ok();
        let original_token = env::var("ZITADEL_TOKEN").ok();
        let original_service_account = env::var("ZITADEL_SERVICE_ACCOUNT_FILE").ok();
        env::remove_var("ZITADEL_TOKEN");
        env::remove_var("ZITADEL_SERVICE_ACCOUNT_FILE");

        let mut server = Server::new_async().await;
        let _whoami = server
            .mock("GET", "/auth/v1/users/me")
            .match_header("authorization", "Bearer service-account-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"user":{"id":"355223427969778852","preferredLoginName":"zitadel-admin-sa","machine":{"name":"Admin"}}}"#,
            )
            .create_async()
            .await;

        env::set_var("ZITADEL_URL", server.url());

        let args = Cli::parse_from([
            "zitadel-tui",
            "--once",
            "--token",
            "service-account-token",
            "auth",
            "status",
        ]);
        let command = match &args.command {
            Some(crate::cli::Command::Auth(command)) => command.clone(),
            other => panic!("unexpected command: {other:?}"),
        };

        let result = execute_auth_command(&command, &args, &AppConfig::default())
            .await
            .unwrap();

        assert_eq!(result["host"], server.url());
        assert_eq!(result["auth_source"], "cli PAT");
        assert_eq!(result["user_id"], "355223427969778852");
        assert_eq!(result["login_name"], "zitadel-admin-sa");

        if let Some(host) = original_host {
            env::set_var("ZITADEL_URL", host);
        } else {
            env::remove_var("ZITADEL_URL");
        }
        if let Some(token) = original_token {
            env::set_var("ZITADEL_TOKEN", token);
        }
        if let Some(path) = original_service_account {
            env::set_var("ZITADEL_SERVICE_ACCOUNT_FILE", path);
        }
    }
}
