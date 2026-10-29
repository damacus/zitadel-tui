use serde_json::Value;

use crate::{
    cli::{Cli, IdpsAction, IdpsCommand},
    config::AppConfig,
    error::CommandError,
};

use super::shared::{authenticated_client, get_result, list_result};

pub async fn execute_idps_command(
    command: &IdpsCommand,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value, CommandError> {
    // Cheap input validation before credential resolution so
    // `error.kind` is deterministic regardless of credentials or API
    // reachability.
    if let IdpsAction::ConfigureGoogle(google) = &command.action {
        if google.client_secret.is_none() {
            return Err(CommandError::usage(
                "--client-secret is required for idps configure-google in headless mode",
            ));
        }
    }

    let client = authenticated_client(args, config).await?;
    match &command.action {
        IdpsAction::List(list) => {
            let idps = crate::ops::idps::list(&client, list.limit)
                .await
                .map_err(CommandError::from)?;
            list_result(&idps, args.full)
        }
        IdpsAction::Get(get) => {
            let selector = get.selector();
            let idp = crate::ops::resolve_idp(&client, &selector)
                .await
                .map_err(CommandError::from)?
                .into_command_result(&selector.describe(), selector.lookup())?;
            get_result(&idp, args.full)
        }
        IdpsAction::ConfigureGoogle(google) => {
            // Validated above; re-check defensively to stay panic-free.
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
