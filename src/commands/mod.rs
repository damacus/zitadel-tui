mod apps;
mod auth;
mod config;
mod idps;
mod projects;
pub mod shared;
mod users;

use serde_json::Value;

use crate::{
    cli::{Cli, Command},
    config::AppConfig,
    error::CommandError,
};

/// Prints docs/AGENT.md verbatim. `agent` is a doc surface, not an API
/// command, so it intentionally bypasses the JSON envelope — main returns
/// early before envelope construction.
pub fn print_agent_guide() {
    print!("{}", include_str!("../../docs/AGENT.md"));
}

pub async fn execute_command(
    command: &Command,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value, CommandError> {
    match command {
        Command::Agent => unreachable!("`agent` is handled before the envelope"),
        Command::Config(config_command) => config::execute_config_command(config_command, config),
        Command::Auth(auth_command) => auth::execute_auth_command(auth_command, args, config).await,
        Command::Apps(apps_command) => apps::execute_apps_command(apps_command, args, config).await,
        Command::Users(users_command) => {
            users::execute_users_command(users_command, args, config).await
        }
        Command::Idps(idps_command) => idps::execute_idps_command(idps_command, args, config).await,
        Command::Projects(projects_command) => {
            projects::execute_projects_command(projects_command, args, config).await
        }
    }
}
