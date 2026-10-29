use serde_json::{json, Value};

use crate::{
    api::client::ZitadelClient,
    cli::{Cli, UserSelectorArgs, UsersAction, UsersCommand},
    config::AppConfig,
    error::{CommandError, ErrorKind},
};

use super::shared::{authenticated_client, get_result, list_result};

pub async fn execute_users_command(
    command: &UsersCommand,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value, CommandError> {
    // Cheap input validation before credential resolution so
    // `error.kind` is deterministic regardless of credentials or API
    // reachability.
    if let UsersAction::CreateAdmin(create) = &command.action {
        if create.password.is_none() {
            return Err(CommandError::usage(
                "--password is required for users create-admin in headless mode",
            ));
        }
    }
    let templates = if matches!(command.action, UsersAction::QuickSetup) {
        Some(
            config
                .templates()
                .map_err(|error| CommandError::config(error.to_string()))?,
        )
    } else {
        None
    };

    let client = authenticated_client(args, config).await?;
    match &command.action {
        UsersAction::List(list) => {
            let users = crate::ops::users::list(&client, list.limit)
                .await
                .map_err(CommandError::from)?;
            list_result(&users, args.full)
        }
        UsersAction::Get(get) => {
            let selector = get.selector();
            let user = crate::ops::resolve_user(&client, &selector)
                .await
                .map_err(CommandError::from)?
                .into_command_result(&selector.describe(), selector.lookup())?;
            get_result(&user, args.full)
        }
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
            // Validated above; re-check defensively to stay panic-free.
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
        UsersAction::GrantIamOwner(grant) => {
            let user_id = resolved_user_id(&client, grant).await?;
            Ok(serde_json::to_value(
                crate::ops::users::grant_iam_owner(&client, &user_id)
                    .await
                    .map_err(CommandError::from)?,
            )?)
        }
        UsersAction::QuickSetup => {
            let templates = templates.expect("templates loaded for users quick-setup");
            let created = crate::ops::users::quick_setup(&client, templates.users)
                .await
                .map_err(CommandError::from)?;
            let results: Vec<_> = created.into_iter().map(|outcome| outcome.result).collect();
            Ok(serde_json::to_value(&results)?)
        }
    }
}

/// Resolves a selector group to a user id: zero matches → `not_found`,
/// multiple → `ambiguous`, a match without an id → `api` (malformed API
/// payload). Used by `grant-iam-owner` which mutates by id.
async fn resolved_user_id(
    client: &ZitadelClient,
    selector_args: &UserSelectorArgs,
) -> Result<String, CommandError> {
    let selector = selector_args.selector();
    let user = crate::ops::resolve_user(client, &selector)
        .await
        .map_err(CommandError::from)?
        .into_command_result(&selector.describe(), selector.lookup())?;
    user.id.clone().ok_or_else(|| {
        CommandError::with_details(
            ErrorKind::Api,
            format!("{} resolved without an id", selector.describe()),
            json!({ "lookup": selector.lookup() }),
        )
    })
}
