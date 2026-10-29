use serde_json::Value;

use crate::{
    cli::{ConfigAction, ConfigCommand},
    config::AppConfig,
    error::CommandError,
};

pub fn execute_config_command(
    command: &ConfigCommand,
    config: &AppConfig,
) -> Result<Value, CommandError> {
    match command.action {
        ConfigAction::Show => serde_json::to_value(config).map_err(CommandError::from),
    }
}
