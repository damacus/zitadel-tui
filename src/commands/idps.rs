use serde_json::Value;

use crate::{
    cli::{Cli, IdpsAction, IdpsCommand},
    config::AppConfig,
    error::CommandError,
};

use super::shared::authenticated_client;

pub async fn execute_idps_command(
    command: &IdpsCommand,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value, CommandError> {
    let client = authenticated_client(args, config).await?;
    match &command.action {
        IdpsAction::List => Ok(serde_json::to_value(
            crate::ops::idps::list(&client)
                .await
                .map_err(CommandError::from)?,
        )?),
        IdpsAction::ConfigureGoogle(google) => {
            let secret = google.client_secret.clone().ok_or_else(|| {
                CommandError::usage(
                    "--client-secret is required for idps configure-google in headless mode",
                )
            })?;
            Ok(serde_json::to_value(
                crate::ops::idps::configure_google(
                    &client,
                    &google.client_id,
                    &secret,
                    &google.name,
                )
                .await
                .map_err(CommandError::from)?,
            )?)
        }
    }
}
