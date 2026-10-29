use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::ops::{AppSelector, IdpSelector, UserSelector};

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
    Projects(ProjectsCommand),
    Auth(AuthCommand),
    Config(ConfigCommand),
    /// Print the LLM agent usage guide (docs/AGENT.md) verbatim.
    Agent,
}

#[derive(Subcommand, Debug, Clone)]
pub enum AppsAction {
    List(ListArgs),
    Get(AppSelectorArgs),
    Create(CreateAppArgs),
    CreateNative(CreateNativeAppArgs),
    Delete(AppSelectorArgs),
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
    List(ListArgs),
    Get(UserSelectorArgs),
    Create(CreateUserArgs),
    CreateAdmin(CreateAdminArgs),
    GrantIamOwner(UserSelectorArgs),
    QuickSetup,
}

#[derive(clap::Args, Debug, Clone)]
pub struct UsersCommand {
    #[command(subcommand)]
    pub action: UsersAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum IdpsAction {
    List(ListArgs),
    Get(IdpSelectorArgs),
    ConfigureGoogle(ConfigureGoogleArgs),
}

#[derive(clap::Args, Debug, Clone)]
pub struct IdpsCommand {
    #[command(subcommand)]
    pub action: IdpsAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ProjectsAction {
    List(ListArgs),
}

#[derive(clap::Args, Debug, Clone)]
pub struct ProjectsCommand {
    #[command(subcommand)]
    pub action: ProjectsAction,
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

/// Shared `--limit` flag for every `list` action.
#[derive(clap::Args, Debug, Clone)]
pub struct ListArgs {
    /// Maximum number of items to return.
    #[arg(
        long,
        default_value_t = 100,
        value_parser = clap::value_parser!(u64).range(1..=1000)
    )]
    pub limit: u64,
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

/// Selects exactly one app: `--app-id` or `--name` (exact match). The
/// required `ArgGroup` rejects commands with zero or both flags (exit 2).
#[derive(clap::Args, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct AppSelectorArgs {
    /// Select the app by id.
    #[arg(long)]
    pub app_id: Option<String>,
    /// Select the app by exact name.
    #[arg(long)]
    pub name: Option<String>,
}

impl AppSelectorArgs {
    /// The parsed selector group as an ops selector; the `ArgGroup`
    /// guarantees exactly one flag was provided.
    pub fn selector(&self) -> AppSelector {
        if let Some(app_id) = &self.app_id {
            return AppSelector::Id(app_id.clone());
        }
        AppSelector::Name(
            self.name
                .clone()
                .expect("ArgGroup guarantees a selector flag"),
        )
    }
}

/// Selects exactly one user: `--user-id`, `--username` (or login name), or
/// `--email` (exact matches). The required `ArgGroup` rejects zero or
/// multiple flags (exit 2).
#[derive(clap::Args, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct UserSelectorArgs {
    /// Select the user by id.
    #[arg(long)]
    pub user_id: Option<String>,
    /// Select the user by exact username or login name.
    #[arg(long)]
    pub username: Option<String>,
    /// Select the user by exact email address.
    #[arg(long)]
    pub email: Option<String>,
}

impl UserSelectorArgs {
    /// The parsed selector group as an ops selector; the `ArgGroup`
    /// guarantees exactly one flag was provided.
    pub fn selector(&self) -> UserSelector {
        if let Some(user_id) = &self.user_id {
            return UserSelector::Id(user_id.clone());
        }
        if let Some(username) = &self.username {
            return UserSelector::Username(username.clone());
        }
        UserSelector::Email(
            self.email
                .clone()
                .expect("ArgGroup guarantees a selector flag"),
        )
    }
}

/// Selects exactly one identity provider: `--idp-id` or `--name` (exact
/// match). The required `ArgGroup` rejects zero or both flags (exit 2).
#[derive(clap::Args, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct IdpSelectorArgs {
    /// Select the identity provider by id.
    #[arg(long)]
    pub idp_id: Option<String>,
    /// Select the identity provider by exact name.
    #[arg(long)]
    pub name: Option<String>,
}

impl IdpSelectorArgs {
    /// The parsed selector group as an ops selector; the `ArgGroup`
    /// guarantees exactly one flag was provided.
    pub fn selector(&self) -> IdpSelector {
        if let Some(idp_id) = &self.idp_id {
            return IdpSelector::Id(idp_id.clone());
        }
        IdpSelector::Name(
            self.name
                .clone()
                .expect("ArgGroup guarantees a selector flag"),
        )
    }
}

#[derive(clap::Args, Debug, Clone)]
pub struct RegenerateSecretArgs {
    #[command(flatten)]
    pub selector: AppSelectorArgs,
    /// Optional client id echoed back in the result payload.
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
    #[arg(long, env = "ZITADEL_USER_PASSWORD", hide_env_values = true)]
    pub password: Option<String>,
}

#[derive(clap::Args, Debug, Clone)]
pub struct ConfigureGoogleArgs {
    #[arg(long)]
    pub client_id: String,
    #[arg(long, env = "ZITADEL_GOOGLE_CLIENT_SECRET", hide_env_values = true)]
    pub client_secret: Option<String>,
    #[arg(long, default_value = "Google")]
    pub name: String,
}

/// Dot-path identifying each command in the JSON envelope's `command`
/// field. Part of the agent contract — keep values stable.
pub fn command_path(command: &Command) -> &'static str {
    match command {
        Command::Apps(apps) => match &apps.action {
            AppsAction::List(_) => "apps.list",
            AppsAction::Get(_) => "apps.get",
            AppsAction::Create(_) => "apps.create",
            AppsAction::CreateNative(_) => "apps.create-native",
            AppsAction::Delete(_) => "apps.delete",
            AppsAction::RegenerateSecret(_) => "apps.regenerate-secret",
            AppsAction::QuickSetup(_) => "apps.quick-setup",
        },
        Command::Users(users) => match &users.action {
            UsersAction::List(_) => "users.list",
            UsersAction::Get(_) => "users.get",
            UsersAction::Create(_) => "users.create",
            UsersAction::CreateAdmin(_) => "users.create-admin",
            UsersAction::GrantIamOwner(_) => "users.grant-iam-owner",
            UsersAction::QuickSetup => "users.quick-setup",
        },
        Command::Idps(idps) => match &idps.action {
            IdpsAction::List(_) => "idps.list",
            IdpsAction::Get(_) => "idps.get",
            IdpsAction::ConfigureGoogle(_) => "idps.configure-google",
        },
        Command::Projects(projects) => match &projects.action {
            ProjectsAction::List(_) => "projects.list",
        },
        Command::Auth(auth) => match &auth.action {
            AuthAction::Login(_) => "auth.login",
            AuthAction::Logout => "auth.logout",
            AuthAction::Status => "auth.status",
        },
        Command::Config(config) => match &config.action {
            ConfigAction::Show => "config.show",
        },
        Command::Agent => "agent",
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
                action: AppsAction::List(ListArgs { limit: 100 })
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
        let cases: [(Command, &str); 22] = [
            (
                Command::Apps(AppsCommand {
                    action: AppsAction::List(ListArgs { limit: 100 }),
                }),
                "apps.list",
            ),
            (
                Command::Apps(AppsCommand {
                    action: AppsAction::Get(AppSelectorArgs {
                        app_id: None,
                        name: Some("x".to_string()),
                    }),
                }),
                "apps.get",
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
                    action: AppsAction::Delete(AppSelectorArgs {
                        app_id: Some("a".to_string()),
                        name: None,
                    }),
                }),
                "apps.delete",
            ),
            (
                Command::Apps(AppsCommand {
                    action: AppsAction::RegenerateSecret(RegenerateSecretArgs {
                        selector: AppSelectorArgs {
                            app_id: Some("a".to_string()),
                            name: None,
                        },
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
                    action: UsersAction::List(ListArgs { limit: 100 }),
                }),
                "users.list",
            ),
            (
                Command::Users(UsersCommand {
                    action: UsersAction::Get(UserSelectorArgs {
                        user_id: Some("u".to_string()),
                        username: None,
                        email: None,
                    }),
                }),
                "users.get",
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
                    action: UsersAction::GrantIamOwner(UserSelectorArgs {
                        user_id: None,
                        username: None,
                        email: Some("u@e.c".to_string()),
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
                    action: IdpsAction::List(ListArgs { limit: 100 }),
                }),
                "idps.list",
            ),
            (
                Command::Idps(IdpsCommand {
                    action: IdpsAction::Get(IdpSelectorArgs {
                        idp_id: Some("i".to_string()),
                        name: None,
                    }),
                }),
                "idps.get",
            ),
            (
                Command::Projects(ProjectsCommand {
                    action: ProjectsAction::List(ListArgs { limit: 100 }),
                }),
                "projects.list",
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
            (Command::Agent, "agent"),
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

    #[test]
    fn parses_agent_command() {
        let cli = Cli::parse_from(["zitadel-tui", "agent"]);
        assert!(matches!(cli.command, Some(Command::Agent)));
    }

    #[test]
    fn parses_projects_list() {
        let cli = Cli::parse_from(["zitadel-tui", "projects", "list"]);
        assert!(matches!(
            cli.command,
            Some(Command::Projects(ProjectsCommand {
                action: ProjectsAction::List(ListArgs { limit: 100 })
            }))
        ));
    }

    #[test]
    fn parses_limit_on_each_list_command() {
        for args in [
            ["zitadel-tui", "apps", "list", "--limit", "5"].as_slice(),
            ["zitadel-tui", "users", "list", "--limit", "5"].as_slice(),
            ["zitadel-tui", "idps", "list", "--limit", "5"].as_slice(),
            ["zitadel-tui", "projects", "list", "--limit", "5"].as_slice(),
        ] {
            let cli = Cli::parse_from(args);
            let limit = match cli.command.expect("list command parses") {
                Command::Apps(AppsCommand {
                    action: AppsAction::List(list),
                })
                | Command::Users(UsersCommand {
                    action: UsersAction::List(list),
                })
                | Command::Idps(IdpsCommand {
                    action: IdpsAction::List(list),
                })
                | Command::Projects(ProjectsCommand {
                    action: ProjectsAction::List(list),
                }) => list.limit,
                other => panic!("expected list command, got {other:?}"),
            };
            assert_eq!(limit, 5);
        }
    }

    #[test]
    fn limit_rejects_out_of_range_values() {
        for value in ["0", "1001"] {
            let result = Cli::try_parse_from(["zitadel-tui", "apps", "list", "--limit", value]);
            assert!(result.is_err(), "--limit {value} should be rejected");
        }
    }

    #[test]
    fn apps_get_requires_exactly_one_selector() {
        assert!(Cli::try_parse_from(["zitadel-tui", "apps", "get"]).is_err());
        assert!(Cli::try_parse_from([
            "zitadel-tui",
            "apps",
            "get",
            "--app-id",
            "a",
            "--name",
            "b"
        ])
        .is_err());
        for args in [
            ["zitadel-tui", "apps", "get", "--app-id", "a"].as_slice(),
            ["zitadel-tui", "apps", "get", "--name", "b"].as_slice(),
        ] {
            assert!(Cli::try_parse_from(args).is_ok());
        }
    }

    #[test]
    fn apps_delete_accepts_either_selector() {
        let cli = Cli::parse_from(["zitadel-tui", "apps", "delete", "--name", "grafana"]);
        match cli.command {
            Some(Command::Apps(AppsCommand {
                action: AppsAction::Delete(args),
            })) => assert_eq!(args.selector(), AppSelector::Name("grafana".to_string())),
            _ => panic!("expected apps delete command"),
        }
    }

    #[test]
    fn regenerate_secret_keeps_client_id_outside_selector_group() {
        // --client-id alone does not satisfy the required selector group.
        assert!(Cli::try_parse_from([
            "zitadel-tui",
            "apps",
            "regenerate-secret",
            "--client-id",
            "c"
        ])
        .is_err());

        let cli = Cli::parse_from([
            "zitadel-tui",
            "apps",
            "regenerate-secret",
            "--name",
            "grafana",
            "--client-id",
            "cid-1",
        ]);
        match cli.command {
            Some(Command::Apps(AppsCommand {
                action: AppsAction::RegenerateSecret(args),
            })) => {
                assert_eq!(
                    args.selector.selector(),
                    AppSelector::Name("grafana".to_string())
                );
                assert_eq!(args.client_id.as_deref(), Some("cid-1"));
            }
            _ => panic!("expected apps regenerate-secret command"),
        }
    }

    #[test]
    fn users_get_accepts_each_of_three_selectors() {
        assert!(Cli::try_parse_from(["zitadel-tui", "users", "get"]).is_err());
        assert!(Cli::try_parse_from([
            "zitadel-tui",
            "users",
            "get",
            "--username",
            "u",
            "--email",
            "e"
        ])
        .is_err());

        let cases: [(&[&str], UserSelector); 3] = [
            (
                &["zitadel-tui", "users", "get", "--user-id", "u-1"],
                UserSelector::Id("u-1".to_string()),
            ),
            (
                &["zitadel-tui", "users", "get", "--username", "admin"],
                UserSelector::Username("admin".to_string()),
            ),
            (
                &["zitadel-tui", "users", "get", "--email", "a@b.c"],
                UserSelector::Email("a@b.c".to_string()),
            ),
        ];
        for (args, expected) in cases {
            let cli = Cli::parse_from(args);
            match cli.command {
                Some(Command::Users(UsersCommand {
                    action: UsersAction::Get(parsed),
                })) => assert_eq!(parsed.selector(), expected),
                _ => panic!("expected users get command"),
            }
        }
    }

    #[test]
    fn users_grant_iam_owner_uses_same_selector_group() {
        let cli = Cli::parse_from([
            "zitadel-tui",
            "users",
            "grant-iam-owner",
            "--username",
            "admin",
        ]);
        match cli.command {
            Some(Command::Users(UsersCommand {
                action: UsersAction::GrantIamOwner(args),
            })) => assert_eq!(args.selector(), UserSelector::Username("admin".to_string())),
            _ => panic!("expected users grant-iam-owner command"),
        }
    }

    #[test]
    fn idps_get_requires_exactly_one_selector() {
        assert!(Cli::try_parse_from(["zitadel-tui", "idps", "get"]).is_err());
        assert!(Cli::try_parse_from([
            "zitadel-tui",
            "idps",
            "get",
            "--idp-id",
            "i",
            "--name",
            "n"
        ])
        .is_err());

        let cli = Cli::parse_from(["zitadel-tui", "idps", "get", "--name", "Google"]);
        match cli.command {
            Some(Command::Idps(IdpsCommand {
                action: IdpsAction::Get(args),
            })) => assert_eq!(args.selector(), IdpSelector::Name("Google".to_string())),
            _ => panic!("expected idps get command"),
        }
    }

    #[test]
    fn create_admin_password_reads_from_env() {
        let _guard = crate::test_support::env_lock();
        let original = env::var("ZITADEL_USER_PASSWORD").ok();
        env::set_var("ZITADEL_USER_PASSWORD", "env-secret");

        let cli = Cli::parse_from([
            "zitadel-tui",
            "users",
            "create-admin",
            "--username",
            "admin",
            "--first-name",
            "A",
            "--last-name",
            "B",
            "--email",
            "a@b.c",
        ]);

        match &original {
            Some(value) => env::set_var("ZITADEL_USER_PASSWORD", value),
            None => env::remove_var("ZITADEL_USER_PASSWORD"),
        }
        match cli.command {
            Some(Command::Users(UsersCommand {
                action: UsersAction::CreateAdmin(args),
            })) => assert_eq!(args.password.as_deref(), Some("env-secret")),
            _ => panic!("expected users create-admin command"),
        }
    }

    #[test]
    fn create_admin_password_flag_beats_env() {
        let _guard = crate::test_support::env_lock();
        let original = env::var("ZITADEL_USER_PASSWORD").ok();
        env::set_var("ZITADEL_USER_PASSWORD", "env-secret");

        let cli = Cli::parse_from([
            "zitadel-tui",
            "users",
            "create-admin",
            "--username",
            "admin",
            "--first-name",
            "A",
            "--last-name",
            "B",
            "--email",
            "a@b.c",
            "--password",
            "flag-secret",
        ]);

        match &original {
            Some(value) => env::set_var("ZITADEL_USER_PASSWORD", value),
            None => env::remove_var("ZITADEL_USER_PASSWORD"),
        }
        match cli.command {
            Some(Command::Users(UsersCommand {
                action: UsersAction::CreateAdmin(args),
            })) => assert_eq!(args.password.as_deref(), Some("flag-secret")),
            _ => panic!("expected users create-admin command"),
        }
    }

    #[test]
    fn google_client_secret_reads_from_env() {
        let _guard = crate::test_support::env_lock();
        let original = env::var("ZITADEL_GOOGLE_CLIENT_SECRET").ok();
        env::set_var("ZITADEL_GOOGLE_CLIENT_SECRET", "env-google-secret");

        let cli = Cli::parse_from([
            "zitadel-tui",
            "idps",
            "configure-google",
            "--client-id",
            "google-cid",
        ]);

        match &original {
            Some(value) => env::set_var("ZITADEL_GOOGLE_CLIENT_SECRET", value),
            None => env::remove_var("ZITADEL_GOOGLE_CLIENT_SECRET"),
        }
        match cli.command {
            Some(Command::Idps(IdpsCommand {
                action: IdpsAction::ConfigureGoogle(args),
            })) => assert_eq!(args.client_secret.as_deref(), Some("env-google-secret")),
            _ => panic!("expected idps configure-google command"),
        }
    }
}
