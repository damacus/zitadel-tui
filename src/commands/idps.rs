use anyhow::{Context, Result};
use serde_json::Value;

use crate::{
    cli::{Cli, IdpsAction, IdpsCommand},
    config::AppConfig,
};

use super::shared::authenticated_client;

pub async fn execute_idps_command(
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
