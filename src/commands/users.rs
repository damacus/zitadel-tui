use serde_json::Value;

use crate::{
    cli::{Cli, UsersAction, UsersCommand},
    config::AppConfig,
    error::CommandError,
};

use super::shared::authenticated_client;

pub async fn execute_users_command(
    command: &UsersCommand,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value, CommandError> {
    let client = authenticated_client(args, config).await?;
    match &command.action {
        UsersAction::List => Ok(serde_json::to_value(
            crate::ops::users::list(&client, 100)
                .await
                .map_err(CommandError::from)?,
        )?),
        UsersAction::Create(create) => Ok(serde_json::to_value(
            crate::ops::users::create_human(
                &client,
                &create.email,
                &create.first_name,
                &create.last_name,
                create.username.as_deref(),
            )
            .await
            .map_err(CommandError::from)?,
        )?),
        UsersAction::CreateAdmin(create) => {
            let password = create.password.clone().ok_or_else(|| {
                CommandError::usage(
                    "--password is required for users create-admin in headless mode",
                )
            })?;
            Ok(serde_json::to_value(
                crate::ops::users::import_admin(
                    &client,
                    &create.username,
                    &create.first_name,
                    &create.last_name,
                    &create.email,
                    &password,
                    true,
                )
                .await
                .map_err(CommandError::from)?,
            )?)
        }
        UsersAction::GrantIamOwner(grant) => Ok(serde_json::to_value(
            crate::ops::users::grant_iam_owner(&client, &grant.user_id)
                .await
                .map_err(CommandError::from)?,
        )?),
        UsersAction::QuickSetup => {
            let templates = config
                .templates()
                .map_err(|error| CommandError::config(error.to_string()))?;
            let created = crate::ops::users::quick_setup(&client, templates.users)
                .await
                .map_err(CommandError::from)?;
            let results: Vec<_> = created.into_iter().map(|outcome| outcome.result).collect();
            Ok(serde_json::to_value(&results)?)
        }
    }
}
