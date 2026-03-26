use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(version, about = "A terminal UI for managing Zitadel resources")]
pub struct Cli {
    #[arg(long, env = "ZITADEL_URL")]
    pub host: Option<String>,

    #[arg(long, env = "ZITADEL_PROJECT_ID")]
    pub project_id: Option<String>,

    #[arg(long, env = "ZITADEL_TOKEN", hide_env_values = true)]
    pub token: Option<String>,

    #[arg(long, env = "ZITADEL_SERVICE_ACCOUNT_FILE")]
    pub service_account_file: Option<PathBuf>,

    #[arg(long)]
    pub config: Option<PathBuf>,

    #[arg(long)]
    pub json: bool,

    #[arg(long)]
    pub once: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    Apps(AppsCommand),
    Users(UsersCommand),
    Idps(IdpsCommand),
    Auth(AuthCommand),
    Config(ConfigCommand),
}

#[derive(Subcommand, Debug, Clone)]
pub enum AppsAction {
    List,
    Create(CreateAppArgs),
    Delete(DeleteAppArgs),
    RegenerateSecret(RegenerateSecretArgs),
    QuickSetup(QuickSetupAppsArgs),
}

#[derive(clap::Args, Debug, Clone)]
pub struct AppsCommand {
    #[command(subcommand)]
    pub action: AppsAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum UsersAction {
    List,
    Create(CreateUserArgs),
    CreateAdmin(CreateAdminArgs),
    GrantIamOwner(GrantUserArgs),
    QuickSetup,
}

#[derive(clap::Args, Debug, Clone)]
pub struct UsersCommand {
    #[command(subcommand)]
    pub action: UsersAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum IdpsAction {
    List,
    ConfigureGoogle(ConfigureGoogleArgs),
}

#[derive(clap::Args, Debug, Clone)]
pub struct IdpsCommand {
    #[command(subcommand)]
    pub action: IdpsAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum AuthAction {
    Validate,
}

#[derive(clap::Args, Debug, Clone)]
pub struct AuthCommand {
    #[command(subcommand)]
    pub action: AuthAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigAction {
    Show,
    ImportLegacy,
}

#[derive(clap::Args, Debug, Clone)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(clap::Args, Debug, Clone)]
pub struct CreateAppArgs {
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long, value_delimiter = ',')]
    pub redirect_uris: Vec<String>,
    #[arg(long)]
    pub public: bool,
    #[arg(long)]
    pub template: Option<String>,
}

#[derive(clap::Args, Debug, Clone)]
pub struct DeleteAppArgs {
    #[arg(long)]
    pub app_id: String,
}

#[derive(clap::Args, Debug, Clone)]
pub struct RegenerateSecretArgs {
    #[arg(long)]
    pub app_id: String,
    #[arg(long)]
    pub client_id: Option<String>,
}

#[derive(clap::Args, Debug, Clone)]
pub struct QuickSetupAppsArgs {
    #[arg(long, value_delimiter = ',')]
    pub names: Vec<String>,
}

#[derive(clap::Args, Debug, Clone)]
pub struct CreateUserArgs {
    #[arg(long)]
    pub email: String,
    #[arg(long)]
    pub first_name: String,
    #[arg(long)]
    pub last_name: String,
    #[arg(long)]
    pub username: Option<String>,
}

#[derive(clap::Args, Debug, Clone)]
pub struct CreateAdminArgs {
    #[arg(long)]
    pub username: String,
    #[arg(long)]
    pub first_name: String,
    #[arg(long)]
    pub last_name: String,
    #[arg(long)]
    pub email: String,
    #[arg(long)]
    pub password: Option<String>,
}

#[derive(clap::Args, Debug, Clone)]
pub struct GrantUserArgs {
    #[arg(long)]
    pub user_id: String,
}

#[derive(clap::Args, Debug, Clone)]
pub struct ConfigureGoogleArgs {
    #[arg(long)]
    pub client_id: String,
    #[arg(long)]
    pub client_secret: Option<String>,
    #[arg(long, default_value = "Google")]
    pub name: String,
}

pub fn command_name(command: &Command) -> String {
    match command {
        Command::Apps(_) => "apps".to_string(),
        Command::Users(_) => "users".to_string(),
        Command::Idps(_) => "idps".to_string(),
        Command::Auth(_) => "auth".to_string(),
        Command::Config(_) => "config".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_apps_list_command() {
        let cli = Cli::parse_from(["zitadel-tui", "apps", "list"]);
        assert!(matches!(
            cli.command,
            Some(Command::Apps(AppsCommand {
                action: AppsAction::List
            }))
        ));
    }

    #[test]
    fn parses_create_app_with_template() {
        let cli = Cli::parse_from([
            "zitadel-tui",
            "--json",
            "apps",
            "create",
            "--template",
            "grafana",
        ]);

        assert!(cli.json);
        match cli.command {
            Some(Command::Apps(AppsCommand {
                action: AppsAction::Create(create),
            })) => {
                assert_eq!(create.template.as_deref(), Some("grafana"));
                assert!(!create.public);
            }
            _ => panic!("expected apps create command"),
        }
    }

    #[test]
    fn parses_user_creation_flags() {
        let cli = Cli::parse_from([
            "zitadel-tui",
            "users",
            "create",
            "--email",
            "alice@example.com",
            "--first-name",
            "Alice",
            "--last-name",
            "Admin",
        ]);

        match cli.command {
            Some(Command::Users(UsersCommand {
                action: UsersAction::Create(create),
            })) => {
                assert_eq!(create.email, "alice@example.com");
                assert_eq!(create.first_name, "Alice");
                assert_eq!(create.last_name, "Admin");
            }
            _ => panic!("expected users create command"),
        }
    }

    #[test]
    fn command_name_matches_variants() {
        assert_eq!(
            command_name(&Command::Auth(AuthCommand {
                action: AuthAction::Validate
            })),
            "auth"
        );
        assert_eq!(
            command_name(&Command::Config(ConfigCommand {
                action: ConfigAction::Show
            })),
            "config"
        );
    }
}
