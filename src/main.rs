mod auth;
mod cli;
mod client;
mod conductor;
mod config;
mod oidc;
#[cfg(test)]
mod test_support;
mod token_cache;
mod tui;

use std::io;

use anyhow::{Context, Result};
use auth::resolve_access_token;
use clap::Parser;
use cli::{
    command_name, AppsAction, AppsCommand, AuthAction, AuthCommand, Cli, Command, ConfigAction,
    ConfigCommand, IdpsAction, IdpsCommand, UsersAction, UsersCommand,
};
use client::ZitadelClient;
use conductor::TuiConductor;
use config::AppConfig;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use serde::Serialize;
use serde_json::Value;
use tui::{draw, App, CanvasMode, Focus};

#[derive(Debug, Serialize)]
struct CommandEnvelope<T: Serialize> {
    command: String,
    ok: bool,
    result: T,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    validate_cli(&args)?;
    let config = load_config(&args)?;

    if let Some(command) = &args.command {
        match execute_command(command, &args, &config).await {
            Ok(result) => {
                if args.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&CommandEnvelope {
                            command: command_name(command),
                            ok: true,
                            result,
                        })?
                    );
                } else {
                    print_human(command, &result);
                }
                return Ok(());
            }
            Err(error) => {
                if args.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "command": command_name(command),
                            "ok": false,
                            "error": error.to_string()
                        }))?
                    );
                    return Ok(());
                }
                return Err(error);
            }
        }
    }

    let conductor = TuiConductor::bootstrap(args.clone(), config).await;
    run_tui(conductor).await
}

fn validate_cli(args: &Cli) -> Result<()> {
    if args.command.is_some() && !args.once {
        anyhow::bail!("non-interactive commands require --once");
    }

    if args.once && args.command.is_none() {
        anyhow::bail!("--once requires a subcommand");
    }

    Ok(())
}

fn load_config(args: &Cli) -> Result<AppConfig> {
    if let Some(path) = &args.config {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let config: AppConfig = toml::from_str(&contents)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        return Ok(config);
    }

    AppConfig::load()
}

async fn execute_command(command: &Command, args: &Cli, config: &AppConfig) -> Result<Value> {
    match command {
        Command::Config(config_command) => execute_config_command(config_command, config),
        Command::Auth(auth_command) => execute_auth_command(auth_command, args, config).await,
        Command::Apps(apps) => execute_apps_command(apps, args, config).await,
        Command::Users(users) => execute_users_command(users, args, config).await,
        Command::Idps(idps) => execute_idps_command(idps, args, config).await,
    }
}

fn execute_config_command(command: &ConfigCommand, config: &AppConfig) -> Result<Value> {
    match command.action {
        ConfigAction::Show => Ok(serde_json::to_value(config)?),
    }
}

async fn execute_auth_command(
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
                    Err(oidc::PollError::Fatal(e)) => return Err(e),
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
            Ok(serde_json::json!({
                "host": host,
                "auth_source": auth.source,
                "user_id": me["user"]["userId"],
                "login_name": me["user"]["preferredLoginName"],
            }))
        }
    }
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

async fn persist_login_session(
    http: &reqwest::Client,
    host: &str,
    client_id: &str,
    tokens: oidc::OidcTokens,
) -> Result<()> {
    auth::validate_login_session_token(http, host, client_id, &tokens.access_token).await?;

    let cache = token_cache::TokenCache {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_at: Some(oidc::expires_at_from_now(tokens.expires_in)),
        client_id: client_id.to_string(),
        host: host.to_string(),
    };
    cache.save()
}
async fn execute_apps_command(
    command: &AppsCommand,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value> {
    let client = authenticated_client(args, config).await?;
    let project_id = resolved_project_id(&client, args, config).await?;

    match &command.action {
        AppsAction::List => Ok(serde_json::to_value(client.list_apps(&project_id).await?)?),
        AppsAction::Create(create) => {
            let templates = config.templates()?;
            let (name, redirect_uris, public) = if let Some(template_name) = &create.template {
                let template = templates
                    .apps
                    .get(template_name)
                    .with_context(|| format!("template {template_name} not found"))?;
                (
                    template_name.clone(),
                    template.redirect_uris.clone(),
                    template.public,
                )
            } else {
                if create.redirect_uris.is_empty() {
                    anyhow::bail!(
                        "--redirect-uris is required for apps create when not using --template"
                    );
                }
                (
                    create
                        .name
                        .clone()
                        .context("app name is required when not using --template")?,
                    create.redirect_uris.clone(),
                    create.public,
                )
            };
            client
                .create_oidc_app(&project_id, &name, redirect_uris, public)
                .await
        }
        AppsAction::CreateNative(native) => {
            client
                .create_native_app(&project_id, &native.name, native.device_code)
                .await
        }
        AppsAction::Delete(delete) => client.delete_app(&project_id, &delete.app_id).await,
        AppsAction::RegenerateSecret(regen) => {
            let mut result = client.regenerate_secret(&project_id, &regen.app_id).await?;
            if let Some(client_id) = &regen.client_id {
                result["clientId"] = Value::String(client_id.clone());
            }
            Ok(result)
        }
        AppsAction::QuickSetup(quick) => {
            let templates = config.templates()?;
            let names = if quick.names.is_empty() {
                templates.apps.keys().cloned().collect::<Vec<_>>()
            } else {
                quick.names.clone()
            };

            let mut created = Vec::new();
            for name in names {
                let template = templates
                    .apps
                    .get(&name)
                    .with_context(|| format!("template {name} not found"))?;
                created.push(
                    client
                        .create_oidc_app(
                            &project_id,
                            &name,
                            template.redirect_uris.clone(),
                            template.public,
                        )
                        .await?,
                );
            }
            Ok(Value::Array(created))
        }
    }
}

async fn execute_users_command(
    command: &UsersCommand,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value> {
    let client = authenticated_client(args, config).await?;
    match &command.action {
        UsersAction::List => Ok(serde_json::to_value(client.list_users(100).await?)?),
        UsersAction::Create(create) => {
            client
                .create_human_user(
                    &create.email,
                    &create.first_name,
                    &create.last_name,
                    create.username.as_deref(),
                )
                .await
        }
        UsersAction::CreateAdmin(create) => {
            let password = create
                .password
                .clone()
                .context("--password is required for users create-admin in headless mode")?;
            client
                .import_human_user(
                    &create.username,
                    &create.first_name,
                    &create.last_name,
                    &create.email,
                    &password,
                    true,
                )
                .await
        }
        UsersAction::GrantIamOwner(grant) => client.grant_iam_owner(&grant.user_id).await,
        UsersAction::QuickSetup => {
            let templates = config.templates()?;
            let mut created = Vec::new();
            for user in templates.users {
                let result = client
                    .create_human_user(&user.email, &user.first_name, &user.last_name, None)
                    .await?;
                if user.admin {
                    if let Some(user_id) = result.get("userId").and_then(|value| value.as_str()) {
                        client.grant_iam_owner(user_id).await?;
                    }
                }
                created.push(result);
            }
            Ok(Value::Array(created))
        }
    }
}

async fn execute_idps_command(
    command: &IdpsCommand,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value> {
    let client = authenticated_client(args, config).await?;
    match &command.action {
        IdpsAction::List => Ok(serde_json::to_value(client.list_idps().await?)?),
        IdpsAction::ConfigureGoogle(google) => {
            let secret = google.client_secret.clone().context(
                "--client-secret is required for idps configure-google in headless mode",
            )?;
            client
                .add_google_idp(&google.client_id, &secret, &google.name)
                .await
        }
    }
}

async fn authenticated_client(args: &Cli, config: &AppConfig) -> Result<ZitadelClient> {
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

fn resolved_host(args: &Cli, config: &AppConfig) -> Result<String> {
    args.host
        .clone()
        .or_else(|| config.zitadel_url.clone())
        .context("Zitadel URL is required via --host, ZITADEL_URL, or config")
}

fn resolved_host_or_cache(args: &Cli, config: &AppConfig) -> Result<String> {
    resolved_host(args, config).or_else(|_| {
        token_cache::TokenCache::load()
            .ok()
            .flatten()
            .map(|c| c.host)
            .context("Zitadel URL is required via --host, ZITADEL_URL, config, or a cached login session")
    })
}

async fn resolved_project_id(
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

async fn run_tui(mut conductor: TuiConductor) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let terminal = ratatui::Terminal::new(CrosstermBackend::new(stdout))?;
    let result = run_app(terminal, &mut conductor).await;
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    result
}

async fn run_app(
    mut terminal: ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    conductor: &mut TuiConductor,
) -> Result<()> {
    let mut app = App::from_bootstrap(conductor.bootstrap_state());

    loop {
        terminal.draw(|frame| draw(frame, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            let Event::Key(key) = event::read()? else {
                continue;
            };

            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('j') | KeyCode::Down => match app.focus {
                    Focus::Resources => app.next_resource(),
                    Focus::Actions => app.next_action(),
                    Focus::Form => app.form_next_field(),
                    Focus::Records => app.next_record(),
                },
                KeyCode::Char('k') | KeyCode::Up => match app.focus {
                    Focus::Resources => app.previous_resource(),
                    Focus::Actions => app.previous_action(),
                    Focus::Form => app.form_previous_field(),
                    Focus::Records => app.previous_record(),
                },
                KeyCode::Char('h') | KeyCode::Left => {
                    if app.focus == Focus::Form {
                        app.form_toggle_or_cycle(false);
                    } else {
                        app.previous_resource();
                    }
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    if app.focus == Focus::Form {
                        app.form_toggle_or_cycle(true);
                    } else {
                        app.next_resource();
                    }
                }
                KeyCode::Char('i') => app.toggle_inspector(),
                KeyCode::Char('n') => app.focus = Focus::Actions,
                KeyCode::Char('g') => app.focus = Focus::Resources,
                KeyCode::Enter => match &app.canvas_mode {
                    CanvasMode::Browse => {
                        let mode = conductor.begin_action(
                            app.active_resource(),
                            app.selected_action,
                            app.selected_record(),
                        );
                        app.set_canvas_mode(mode);
                    }
                    CanvasMode::EditForm(form) | CanvasMode::Setup(form) => {
                        let next = conductor.submit_form(form).await;
                        app.set_canvas_mode(next);
                        app.sync_runtime(conductor.bootstrap_state());
                    }
                    CanvasMode::Confirm(confirm) => {
                        let next = conductor.confirm(confirm.pending.clone()).await;
                        app.set_canvas_mode(next);
                        app.sync_runtime(conductor.bootstrap_state());
                    }
                    CanvasMode::Success(_) | CanvasMode::Error(_) => app.reset_to_browse(),
                },
                KeyCode::Esc => match app.canvas_mode {
                    CanvasMode::Browse => {}
                    CanvasMode::EditForm(_) | CanvasMode::Setup(_) | CanvasMode::Confirm(_) => {
                        app.reset_to_browse()
                    }
                    CanvasMode::Success(_) | CanvasMode::Error(_) => app.reset_to_browse(),
                },
                KeyCode::Backspace => {
                    if app.focus == Focus::Form {
                        app.form_backspace();
                    }
                }
                KeyCode::Char(' ') => {
                    if app.focus == Focus::Form {
                        app.form_toggle_or_cycle(true);
                    }
                }
                KeyCode::Char(ch) if app.focus == Focus::Form => app.form_insert_char(ch),
                KeyCode::Tab => app.advance_focus(),
                KeyCode::BackTab => app.reverse_focus(),
                _ => {}
            }
        }
    }

    Ok(())
}

fn print_human(command: &Command, result: &Value) {
    match command {
        Command::Apps(_) | Command::Users(_) | Command::Idps(_) => {
            if let Some(items) = result.as_array() {
                for item in items {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(item).unwrap_or_else(|_| item.to_string())
                    );
                }
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string())
                );
            }
        }
        Command::Auth(_) | Command::Config(_) => {
            println!(
                "{}",
                serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string())
            );
        }
    }
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
        env::temp_dir().join(format!("zitadel-tui-main-test-tokens-{unique}.json"))
    }

    #[test]
    fn rejects_command_without_once() {
        let cli = Cli::parse_from(["zitadel-tui", "apps", "list"]);
        assert!(validate_cli(&cli).is_err());
    }

    #[test]
    fn rejects_once_without_command() {
        let cli = Cli::parse_from(["zitadel-tui", "--once"]);
        assert!(validate_cli(&cli).is_err());
    }

    #[test]
    fn accepts_once_with_command() {
        let cli = Cli::parse_from(["zitadel-tui", "--once", "apps", "list"]);
        assert!(validate_cli(&cli).is_ok());
    }

    #[test]
    fn accepts_once_with_host_and_command() {
        let cli = Cli::parse_from([
            "zitadel-tui",
            "--once",
            "--host",
            "https://zitadel.example.com",
            "apps",
            "list",
        ]);
        assert!(validate_cli(&cli).is_ok());
    }

    #[test]
    fn resolved_host_uses_cli_arg() {
        let cli = Cli::parse_from(["zitadel-tui", "--host", "https://cli.example.com"]);
        let config = AppConfig::default();
        assert_eq!(
            resolved_host(&cli, &config).unwrap(),
            "https://cli.example.com"
        );
    }

    #[test]
    fn resolved_host_falls_back_to_config() {
        let cli = Cli::parse_from(["zitadel-tui"]);
        let config = AppConfig {
            zitadel_url: Some("https://config.example.com".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolved_host(&cli, &config).unwrap(),
            "https://config.example.com"
        );
    }

    #[test]
    fn resolved_host_cli_arg_takes_precedence_over_config() {
        let cli = Cli::parse_from(["zitadel-tui", "--host", "https://cli.example.com"]);
        let config = AppConfig {
            zitadel_url: Some("https://config.example.com".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolved_host(&cli, &config).unwrap(),
            "https://cli.example.com"
        );
    }

    #[test]
    fn resolved_host_errors_when_absent() {
        let cli = Cli::parse_from(["zitadel-tui"]);
        let config = AppConfig::default();
        assert!(resolved_host(&cli, &config).is_err());
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn persist_login_session_saves_cache_after_successful_auth_probe() {
        let mut server = Server::new_async().await;
        let whoami = server
            .mock("GET", "/auth/v1/users/me")
            .match_header("authorization", "Bearer header.payload.signature")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"user":{"userId":"u1","preferredLoginName":"alice@example.com"}}"#)
            .create_async()
            .await;

        let cache_path = temp_cache_path();
        let _guard = crate::test_support::env_lock();
        let original_cache = env::var("ZITADEL_TUI_TOKEN_CACHE").ok();
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);

        let http = reqwest::Client::new();
        let tokens = oidc::OidcTokens {
            access_token: "header.payload.signature".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            expires_in: 3600,
        };

        persist_login_session(&http, &server.url(), "client-1", tokens)
            .await
            .unwrap();

        whoami.assert_async().await;
        assert!(cache_path.exists());
        let saved = token_cache::TokenCache::load().unwrap().unwrap();
        assert_eq!(saved.client_id, "client-1");
        assert_eq!(saved.host, server.url());

        env::remove_var("ZITADEL_TUI_TOKEN_CACHE");
        let _ = fs::remove_file(&cache_path);
        if let Some(path) = original_cache {
            env::set_var("ZITADEL_TUI_TOKEN_CACHE", path);
        }
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn persist_login_session_rejects_unusable_device_tokens_without_writing_cache() {
        let cache_path = temp_cache_path();
        let _guard = crate::test_support::env_lock();
        let original_cache = env::var("ZITADEL_TUI_TOKEN_CACHE").ok();
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);

        let http = reqwest::Client::new();
        let tokens = oidc::OidcTokens {
            access_token: "opaque-token".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            expires_in: 3600,
        };

        let error = persist_login_session(&http, "https://zitadel.example.com", "client-1", tokens)
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("JWT access tokens"));
        assert!(!cache_path.exists());

        env::remove_var("ZITADEL_TUI_TOKEN_CACHE");
        let _ = fs::remove_file(&cache_path);
        if let Some(path) = original_cache {
            env::set_var("ZITADEL_TUI_TOKEN_CACHE", path);
        }
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn persist_login_session_rejects_auth_api_probe_failures_without_writing_cache() {
        let mut server = Server::new_async().await;
        let whoami = server
            .mock("GET", "/auth/v1/users/me")
            .match_header("authorization", "Bearer header.payload.signature")
            .with_status(403)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"code":7,"message":"authentication required (AUTHZ-Kl3p0)","details":[{"id":"AUTHZ-Kl3p0"}]}"#,
            )
            .create_async()
            .await;

        let cache_path = temp_cache_path();
        let _guard = crate::test_support::env_lock();
        let original_cache = env::var("ZITADEL_TUI_TOKEN_CACHE").ok();
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);

        let http = reqwest::Client::new();
        let tokens = oidc::OidcTokens {
            access_token: "header.payload.signature".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            expires_in: 3600,
        };

        let error = persist_login_session(&http, &server.url(), "client-1", tokens)
            .await
            .unwrap_err()
            .to_string();

        whoami.assert_async().await;
        assert!(error.contains("JWT access tokens"));
        assert!(!cache_path.exists());

        env::remove_var("ZITADEL_TUI_TOKEN_CACHE");
        let _ = fs::remove_file(&cache_path);
        if let Some(path) = original_cache {
            env::set_var("ZITADEL_TUI_TOKEN_CACHE", path);
        }
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn auth_status_uses_valid_cached_session_tokens() {
        let mut server = Server::new_async().await;
        let whoami = server
            .mock("GET", "/auth/v1/users/me")
            .match_header("authorization", "Bearer header.payload.signature")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"user":{"userId":"u1","preferredLoginName":"alice@example.com"}}"#)
            .create_async()
            .await;

        let cache_path = temp_cache_path();
        let _guard = crate::test_support::env_lock();
        let original_cache = env::var("ZITADEL_TUI_TOKEN_CACHE").ok();
        env::set_var("ZITADEL_TUI_TOKEN_CACHE", &cache_path);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let cache = token_cache::TokenCache {
            access_token: "header.payload.signature".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            expires_at: Some(now + 3600),
            client_id: "client-1".to_string(),
            host: server.url(),
        };
        cache.save().unwrap();

        let cli = Cli::parse_from(["zitadel-tui", "--once", "auth", "status"]);
        let command = AuthCommand {
            action: AuthAction::Status,
        };
        let config = AppConfig::default();
        let result = execute_auth_command(&command, &cli, &config).await.unwrap();

        whoami.assert_async().await;
        assert_eq!(result["auth_source"], "session token");
        assert_eq!(result["user_id"], "u1");
        assert_eq!(result["login_name"], "alice@example.com");

        env::remove_var("ZITADEL_TUI_TOKEN_CACHE");
        let _ = fs::remove_file(&cache_path);
        if let Some(path) = original_cache {
            env::set_var("ZITADEL_TUI_TOKEN_CACHE", path);
        }
    }
}
