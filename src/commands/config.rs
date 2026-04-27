use anyhow::Result;
use serde_json::Value;

use crate::{
    cli::{ConfigAction, ConfigCommand},
    config::AppConfig,
};

pub fn execute_config_command(command: &ConfigCommand, config: &AppConfig) -> Result<Value> {
    match command.action {
        ConfigAction::Show => Ok(serde_json::to_value(config)?),
    }
}
