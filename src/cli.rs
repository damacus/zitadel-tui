use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(version, about = "A terminal UI for managing Zitadel resources")]
pub struct Cli {
    #[arg(long, env = "ZITADEL_URL", global = true)]
    pub host: Option<String>,

    #[arg(long, env = "ZITADEL_PROJECT_ID", global = true)]
    pub project_id: Option<String>,

    #[arg(long, env = "ZITADEL_TOKEN", hide_env_values = true, global = true)]
    pub token: Option<String>,

    #[arg(long, env = "ZITADEL_SERVICE_ACCOUNT_FILE", global = true)]
    pub service_account_file: Option<PathBuf>,

    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long, global = true)]
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
    Login(LoginArgs),
    Logout,
    Status,
}

#[derive(clap::Args, Debug, Clone)]
pub struct LoginArgs {
    #[arg(long)]
    pub client_id: Option<String>,
}

#[derive(clap::Args, Debug, Clone)]
pub struct AuthCommand {
    #[command(subcommand)]
    pub action: AuthAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigAction {
    Show,
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
                action: AuthAction::Status
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

    #[test]
    fn parses_host_flag() {
        let cli = Cli::parse_from(["zitadel-tui", "--host", "https://zitadel.example.com"]);
        assert_eq!(cli.host.as_deref(), Some("https://zitadel.example.com"));
    }

    #[test]
    fn host_defaults_to_none_when_absent() {
        let cli = Cli::parse_from(["zitadel-tui"]);
        assert!(cli.host.is_none());
    }

    #[test]
    fn parses_once_flag() {
        let cli = Cli::parse_from(["zitadel-tui", "--once", "apps", "list"]);
        assert!(cli.once);
    }

    #[test]
    fn parses_once_with_host_and_subcommand() {
        let cli = Cli::parse_from([
            "zitadel-tui",
            "--once",
            "--host",
            "https://zitadel.example.com",
            "apps",
            "list",
        ]);
        assert!(cli.once);
        assert_eq!(cli.host.as_deref(), Some("https://zitadel.example.com"));
        assert!(matches!(
            cli.command,
            Some(Command::Apps(AppsCommand {
                action: AppsAction::List
            }))
        ));
    }

    #[test]
    fn parses_token_flag() {
        let cli = Cli::parse_from(["zitadel-tui", "--token", "my-secret-token"]);
        assert_eq!(cli.token.as_deref(), Some("my-secret-token"));
    }

    #[test]
    fn parses_project_id_flag() {
        let cli = Cli::parse_from(["zitadel-tui", "--project-id", "proj-123"]);
        assert_eq!(cli.project_id.as_deref(), Some("proj-123"));
    }

    #[test]
    fn host_flag_requires_a_value() {
        let result = Cli::try_parse_from(["zitadel-tui", "--host"]);
        assert!(result.is_err());
    }

    #[test]
    fn parses_auth_login() {
        let cli = Cli::parse_from(["zitadel-tui", "--once", "auth", "login"]);
        assert!(matches!(
            cli.command,
            Some(Command::Auth(AuthCommand {
                action: AuthAction::Login(LoginArgs { client_id: None })
            }))
        ));
    }

    #[test]
    fn parses_auth_login_with_client_id() {
        let cli = Cli::parse_from([
            "zitadel-tui",
            "--once",
            "auth",
            "login",
            "--client-id",
            "my-client",
        ]);
        match cli.command {
            Some(Command::Auth(AuthCommand {
                action: AuthAction::Login(args),
            })) => assert_eq!(args.client_id.as_deref(), Some("my-client")),
            _ => panic!("expected auth login"),
        }
    }

    #[test]
    fn parses_auth_logout() {
        let cli = Cli::parse_from(["zitadel-tui", "--once", "auth", "logout"]);
        assert!(matches!(
            cli.command,
            Some(Command::Auth(AuthCommand {
                action: AuthAction::Logout
            }))
        ));
    }

    // Global arg tests: flags may appear after the subcommand
    #[test]
    fn host_after_subcommand_is_accepted() {
        let cli = Cli::parse_from([
            "zitadel-tui",
            "--once",
            "apps",
            "list",
            "--host",
            "https://zitadel.example.com",
        ]);
        assert_eq!(cli.host.as_deref(), Some("https://zitadel.example.com"));
    }

    #[test]
    fn token_after_subcommand_is_accepted() {
        let cli = Cli::parse_from([
            "zitadel-tui",
            "--once",
            "apps",
            "list",
            "--token",
            "my-token",
        ]);
        assert_eq!(cli.token.as_deref(), Some("my-token"));
    }

    #[test]
    fn project_id_after_subcommand_is_accepted() {
        let cli = Cli::parse_from([
            "zitadel-tui",
            "--once",
            "apps",
            "list",
            "--project-id",
            "proj-456",
        ]);
        assert_eq!(cli.project_id.as_deref(), Some("proj-456"));
    }

    #[test]
    fn json_after_subcommand_is_accepted() {
        let cli = Cli::parse_from(["zitadel-tui", "--once", "apps", "list", "--json"]);
        assert!(cli.json);
    }

    #[test]
    fn once_after_subcommand_is_accepted() {
        let cli = Cli::parse_from(["zitadel-tui", "apps", "list", "--once"]);
        assert!(cli.once);
    }

    #[test]
    fn all_global_flags_after_subcommand() {
        let cli = Cli::parse_from([
            "zitadel-tui",
            "apps",
            "list",
            "--once",
            "--host",
            "https://zitadel.example.com",
            "--token",
            "tok",
            "--project-id",
            "proj-1",
            "--json",
        ]);
        assert!(cli.once);
        assert!(cli.json);
        assert_eq!(cli.host.as_deref(), Some("https://zitadel.example.com"));
        assert_eq!(cli.token.as_deref(), Some("tok"));
        assert_eq!(cli.project_id.as_deref(), Some("proj-1"));
    }
}
