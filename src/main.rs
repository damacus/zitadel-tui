mod auth;
mod cli;
mod client;
mod config;
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
use config::AppConfig;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use serde::Serialize;
use serde_json::Value;
use tui::{draw, App, Focus, Record, TuiBootstrap};

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

    let bootstrap = bootstrap_tui_state(&args, &config).await;
    run_tui(bootstrap)
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
        ConfigAction::ImportLegacy => {
            if let Some(path) = dirs::home_dir().map(|home| home.join(".zitadel-tui.yml")) {
                let imported = AppConfig::import_legacy(&path)?;
                let saved_to = imported.save_to_canonical_path()?;
                Ok(serde_json::json!({
                    "saved_to": saved_to,
                    "config": imported
                }))
            } else {
                Ok(serde_json::json!({
                    "message": "No home directory found for legacy import"
                }))
            }
        }
    }
}

async fn execute_auth_command(
    command: &AuthCommand,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value> {
    let host = resolved_host(args, config)?;
    let http = reqwest::Client::new();
    match command.action {
        AuthAction::Validate => {
            let auth = resolve_access_token(
                &http,
                &host,
                args.token.clone(),
                args.service_account_file.clone(),
                config,
            )
            .await?;
            let client = ZitadelClient::new(host.clone(), auth.token)?;
            let projects = client.list_projects().await?;
            Ok(serde_json::json!({
                "host": host,
                "auth_source": auth.source,
                "project_count": projects.len(),
            }))
        }
    }
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

async fn bootstrap_tui_state(args: &Cli, config: &AppConfig) -> TuiBootstrap {
    let host = args
        .host
        .clone()
        .or_else(|| config.zitadel_url.clone())
        .unwrap_or_else(|| "https://zitadel.example.com".to_string());
    let project = args
        .project_id
        .clone()
        .or_else(|| config.project_id.clone())
        .unwrap_or_else(|| "default".to_string());
    let mut bootstrap = TuiBootstrap {
        host: host.clone(),
        project,
        auth_label: if config.pat.is_some() {
            "PAT".to_string()
        } else if config.service_account_file.is_some() {
            "Service account".to_string()
        } else {
            "Setup required".to_string()
        },
        templates_path: config
            .apps_config_file
            .as_ref()
            .map(|path| path.display().to_string()),
        setup_required: config.zitadel_url.is_none()
            || (config.pat.is_none() && config.service_account_file.is_none()),
        app_records: Vec::new(),
        user_records: Vec::new(),
        idp_records: Vec::new(),
    };

    if let Ok(auth) = resolve_access_token(
        &reqwest::Client::new(),
        &host,
        args.token.clone(),
        args.service_account_file.clone(),
        config,
    )
    .await
    {
        bootstrap.auth_label = auth.source.to_string();
        bootstrap.setup_required = false;
    }

    if let Ok(client) = authenticated_client(args, config).await {
        if let Ok(project_id) = resolved_project_id(&client, args, config).await {
            bootstrap.project = project_id.clone();
            if let Ok(apps) = client.list_apps(&project_id).await {
                bootstrap.app_records = apps
                    .into_iter()
                    .map(|app| Record {
                        name: string_field(&app, "name", "unnamed"),
                        kind: app
                            .get("oidcConfig")
                            .and_then(|oidc| oidc.get("authMethodType"))
                            .and_then(|value| value.as_str())
                            .map(|value| {
                                if value == "OIDC_AUTH_METHOD_TYPE_NONE" {
                                    "public".to_string()
                                } else {
                                    "confidential".to_string()
                                }
                            })
                            .unwrap_or_else(|| string_field(&app, "state", "unknown")),
                        redirects: format!(
                            "{} configured",
                            app.get("oidcConfig")
                                .and_then(|oidc| oidc.get("redirectUris"))
                                .and_then(|value| value.as_array())
                                .map(|uris| uris.len())
                                .unwrap_or(0)
                        ),
                        changed_at: string_field(&app, "state", "loaded"),
                    })
                    .collect::<Vec<_>>();
            }

            if let Ok(users) = client.list_users(100).await {
                bootstrap.user_records = users
                    .into_iter()
                    .map(|user| Record {
                        name: string_field(&user, "userName", "unknown-user"),
                        kind: string_field(&user, "state", "unknown"),
                        redirects: user
                            .get("human")
                            .and_then(|human| human.get("email"))
                            .and_then(|email| email.get("email"))
                            .and_then(|email| email.as_str())
                            .unwrap_or("no email")
                            .to_string(),
                        changed_at: user
                            .get("human")
                            .and_then(|human| human.get("profile"))
                            .and_then(|profile| profile.get("displayName"))
                            .and_then(|display_name| display_name.as_str())
                            .unwrap_or("human user")
                            .to_string(),
                    })
                    .collect::<Vec<_>>();
            }

            if let Ok(idps) = client.list_idps().await {
                bootstrap.idp_records = idps
                    .into_iter()
                    .map(|idp| Record {
                        name: string_field(&idp, "name", "unnamed-idp"),
                        kind: string_field(&idp, "state", "unknown"),
                        redirects: string_field(&idp, "type", "provider"),
                        changed_at: string_field(&idp, "id", "configured"),
                    })
                    .collect::<Vec<_>>();
            }
        }
    }

    bootstrap
}

fn run_tui(bootstrap: TuiBootstrap) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let terminal = ratatui::Terminal::new(CrosstermBackend::new(stdout))?;
    let result = run_app(terminal, bootstrap);
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    result
}

fn run_app(
    mut terminal: ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    bootstrap: TuiBootstrap,
) -> Result<()> {
    let mut app = App::from_bootstrap(bootstrap);

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
                    Focus::Form | Focus::Records => app.next_record(),
                },
                KeyCode::Char('k') | KeyCode::Up => match app.focus {
                    Focus::Resources => app.previous_resource(),
                    Focus::Actions => app.previous_action(),
                    Focus::Form | Focus::Records => app.previous_record(),
                },
                KeyCode::Char('h') | KeyCode::Left => app.previous_resource(),
                KeyCode::Char('l') | KeyCode::Right => app.next_resource(),
                KeyCode::Char('i') => app.toggle_inspector(),
                KeyCode::Char('n') => app.focus = Focus::Actions,
                KeyCode::Char('g') => app.focus = Focus::Resources,
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

fn string_field(value: &Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(|field| field.as_str())
        .unwrap_or(fallback)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
