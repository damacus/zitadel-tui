use anyhow::{Context, Result};
use serde_json::Value;

use crate::{
    cli::{Cli, UsersAction, UsersCommand},
    config::AppConfig,
};

use super::shared::authenticated_client;

pub async fn execute_users_command(
    command: &UsersCommand,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value> {
    let client = authenticated_client(args, config).await?;
    match &command.action {
        UsersAction::List => Ok(serde_json::to_value(client.list_users(100).await?)?),
        UsersAction::Create(create) => {
            client
                .create_human_user(
                    &create.email,
                    &create.first_name,
                    &create.last_name,
                    create.username.as_deref(),
                )
                .await
        }
        UsersAction::CreateAdmin(create) => {
            let password = create
                .password
                .clone()
                .context("--password is required for users create-admin in headless mode")?;
            client
                .import_human_user(
                    &create.username,
                    &create.first_name,
                    &create.last_name,
                    &create.email,
                    &password,
                    true,
                )
                .await
        }
        UsersAction::GrantIamOwner(grant) => client.grant_iam_owner(&grant.user_id).await,
        UsersAction::QuickSetup => {
            let templates = config.templates()?;
            let mut created = Vec::new();
            for user in templates.users {
                let result = client
                    .create_human_user(&user.email, &user.first_name, &user.last_name, None)
                    .await?;
                if user.admin {
                    if let Some(user_id) = result.get("userId").and_then(|value| value.as_str()) {
                        client.grant_iam_owner(user_id).await?;
                    }
                }
                created.push(result);
            }
            Ok(Value::Array(created))
        }
    }
}
