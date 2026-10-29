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

    /// Emit each item's raw API payload instead of the compact summary
    /// projection (list/get commands).
    #[arg(long, global = true)]
    pub full: bool,

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
    CreateNative(CreateNativeAppArgs),
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
pub struct CreateNativeAppArgs {
    /// Display name for the native application.
    #[arg(long)]
    pub name: String,
    /// Enable the Device Code grant and JWT access tokens for CLI login sessions.
    #[arg(long)]
    pub device_code: bool,
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

/// Dot-path identifying each command in the JSON envelope's `command`
/// field. Part of the agent contract — keep values stable.
pub fn command_path(command: &Command) -> &'static str {
    match command {
        Command::Apps(apps) => match &apps.action {
            AppsAction::List => "apps.list",
            AppsAction::Create(_) => "apps.create",
            AppsAction::CreateNative(_) => "apps.create-native",
            AppsAction::Delete(_) => "apps.delete",
            AppsAction::RegenerateSecret(_) => "apps.regenerate-secret",
            AppsAction::QuickSetup(_) => "apps.quick-setup",
        },
        Command::Users(users) => match &users.action {
            UsersAction::List => "users.list",
            UsersAction::Create(_) => "users.create",
            UsersAction::CreateAdmin(_) => "users.create-admin",
            UsersAction::GrantIamOwner(_) => "users.grant-iam-owner",
            UsersAction::QuickSetup => "users.quick-setup",
        },
        Command::Idps(idps) => match &idps.action {
            IdpsAction::List => "idps.list",
            IdpsAction::ConfigureGoogle(_) => "idps.configure-google",
        },
        Command::Auth(auth) => match &auth.action {
            AuthAction::Login(_) => "auth.login",
            AuthAction::Logout => "auth.logout",
            AuthAction::Status => "auth.status",
        },
        Command::Config(config) => match &config.action {
            ConfigAction::Show => "config.show",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::env;

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
        let cli = Cli::parse_from(["zitadel-tui", "apps", "create", "--template", "grafana"]);

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
    fn command_paths_cover_every_variant() {
        let cases: [(Command, &str); 17] = [
            (
                Command::Apps(AppsCommand {
                    action: AppsAction::List,
                }),
                "apps.list",
            ),
            (
                Command::Apps(AppsCommand {
                    action: AppsAction::Create(CreateAppArgs {
                        name: None,
                        redirect_uris: vec![],
                        public: false,
                        template: None,
                    }),
                }),
                "apps.create",
            ),
            (
                Command::Apps(AppsCommand {
                    action: AppsAction::CreateNative(CreateNativeAppArgs {
                        name: "x".to_string(),
                        device_code: false,
                    }),
                }),
                "apps.create-native",
            ),
            (
                Command::Apps(AppsCommand {
                    action: AppsAction::Delete(DeleteAppArgs {
                        app_id: "a".to_string(),
                    }),
                }),
                "apps.delete",
            ),
            (
                Command::Apps(AppsCommand {
                    action: AppsAction::RegenerateSecret(RegenerateSecretArgs {
                        app_id: "a".to_string(),
                        client_id: None,
                    }),
                }),
                "apps.regenerate-secret",
            ),
            (
                Command::Apps(AppsCommand {
                    action: AppsAction::QuickSetup(QuickSetupAppsArgs { names: vec![] }),
                }),
                "apps.quick-setup",
            ),
            (
                Command::Users(UsersCommand {
                    action: UsersAction::List,
                }),
                "users.list",
            ),
            (
                Command::Users(UsersCommand {
                    action: UsersAction::Create(CreateUserArgs {
                        email: "e".to_string(),
                        first_name: "f".to_string(),
                        last_name: "l".to_string(),
                        username: None,
                    }),
                }),
                "users.create",
            ),
            (
                Command::Users(UsersCommand {
                    action: UsersAction::CreateAdmin(CreateAdminArgs {
                        username: "u".to_string(),
                        first_name: "f".to_string(),
                        last_name: "l".to_string(),
                        email: "e".to_string(),
                        password: None,
                    }),
                }),
                "users.create-admin",
            ),
            (
                Command::Users(UsersCommand {
                    action: UsersAction::GrantIamOwner(GrantUserArgs {
                        user_id: "u".to_string(),
                    }),
                }),
                "users.grant-iam-owner",
            ),
            (
                Command::Users(UsersCommand {
                    action: UsersAction::QuickSetup,
                }),
                "users.quick-setup",
            ),
            (
                Command::Idps(IdpsCommand {
                    action: IdpsAction::List,
                }),
                "idps.list",
            ),
            (
                Command::Idps(IdpsCommand {
                    action: IdpsAction::ConfigureGoogle(ConfigureGoogleArgs {
                        client_id: "c".to_string(),
                        client_secret: None,
                        name: "Google".to_string(),
                    }),
                }),
                "idps.configure-google",
            ),
            (
                Command::Auth(AuthCommand {
                    action: AuthAction::Login(LoginArgs { client_id: None }),
                }),
                "auth.login",
            ),
            (
                Command::Auth(AuthCommand {
                    action: AuthAction::Logout,
                }),
                "auth.logout",
            ),
            (
                Command::Auth(AuthCommand {
                    action: AuthAction::Status,
                }),
                "auth.status",
            ),
            (
                Command::Config(ConfigCommand {
                    action: ConfigAction::Show,
                }),
                "config.show",
            ),
        ];

        for (command, expected) in cases {
            assert_eq!(command_path(&command), expected);
        }
    }

    #[test]
    fn parses_host_flag() {
        let cli = Cli::parse_from(["zitadel-tui", "--host", "https://zitadel.example.com"]);
        assert_eq!(cli.host.as_deref(), Some("https://zitadel.example.com"));
    }

    #[test]
    fn host_defaults_to_none_when_absent() {
        let _guard = crate::test_support::env_lock();
        let original = clear_host_env();
        let cli = Cli::parse_from(["zitadel-tui"]);
        restore_host_env(original);
        assert!(cli.host.is_none());
    }

    #[test]
    fn removed_once_flag_is_rejected() {
        // BREAKING: --once was removed; subcommands are non-interactive.
        assert!(Cli::try_parse_from(["zitadel-tui", "--once", "apps", "list"]).is_err());
    }

    #[test]
    fn removed_json_flag_is_rejected() {
        // BREAKING: --json was removed; output is always the JSON envelope.
        assert!(Cli::try_parse_from(["zitadel-tui", "--json", "apps", "list"]).is_err());
    }

    #[test]
    fn parses_full_flag() {
        let cli = Cli::parse_from(["zitadel-tui", "--full", "apps", "list"]);
        assert!(cli.full);
    }

    #[test]
    fn full_flag_after_subcommand_is_accepted() {
        let cli = Cli::parse_from(["zitadel-tui", "apps", "list", "--full"]);
        assert!(cli.full);
    }

    #[test]
    fn full_defaults_to_false() {
        let cli = Cli::parse_from(["zitadel-tui", "apps", "list"]);
        assert!(!cli.full);
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
        let cli = Cli::parse_from(["zitadel-tui", "auth", "login"]);
        assert!(matches!(
            cli.command,
            Some(Command::Auth(AuthCommand {
                action: AuthAction::Login(LoginArgs { client_id: None })
            }))
        ));
    }

    #[test]
    fn parses_auth_login_with_client_id() {
        let cli = Cli::parse_from(["zitadel-tui", "auth", "login", "--client-id", "my-client"]);
        match cli.command {
            Some(Command::Auth(AuthCommand {
                action: AuthAction::Login(args),
            })) => assert_eq!(args.client_id.as_deref(), Some("my-client")),
            _ => panic!("expected auth login"),
        }
    }

    #[test]
    fn parses_auth_logout() {
        let cli = Cli::parse_from(["zitadel-tui", "auth", "logout"]);
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
            "apps",
            "list",
            "--host",
            "https://zitadel.example.com",
        ]);
        assert_eq!(cli.host.as_deref(), Some("https://zitadel.example.com"));
    }

    #[test]
    fn token_after_subcommand_is_accepted() {
        let cli = Cli::parse_from(["zitadel-tui", "apps", "list", "--token", "my-token"]);
        assert_eq!(cli.token.as_deref(), Some("my-token"));
    }

    #[test]
    fn project_id_after_subcommand_is_accepted() {
        let cli = Cli::parse_from(["zitadel-tui", "apps", "list", "--project-id", "proj-456"]);
        assert_eq!(cli.project_id.as_deref(), Some("proj-456"));
    }

    #[test]
    fn all_global_flags_after_subcommand() {
        let cli = Cli::parse_from([
            "zitadel-tui",
            "apps",
            "list",
            "--host",
            "https://zitadel.example.com",
            "--token",
            "tok",
            "--project-id",
            "proj-1",
        ]);
        assert_eq!(cli.host.as_deref(), Some("https://zitadel.example.com"));
        assert_eq!(cli.token.as_deref(), Some("tok"));
        assert_eq!(cli.project_id.as_deref(), Some("proj-1"));
    }
}
