mod apps;
mod auth;
mod config;
mod idps;
pub mod shared;
mod users;

use anyhow::Result;
use serde_json::Value;

use crate::{
    cli::{Cli, Command},
    config::AppConfig,
};

pub async fn execute_command(command: &Command, args: &Cli, config: &AppConfig) -> Result<Value> {
    match command {
        Command::Config(config_command) => config::execute_config_command(config_command, config),
        Command::Auth(auth_command) => auth::execute_auth_command(auth_command, args, config).await,
        Command::Apps(apps_command) => apps::execute_apps_command(apps_command, args, config).await,
        Command::Users(users_command) => {
            users::execute_users_command(users_command, args, config).await
        }
        Command::Idps(idps_command) => idps::execute_idps_command(idps_command, args, config).await,
    }
}
