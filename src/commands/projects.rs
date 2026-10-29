use serde_json::Value;

use crate::{
    cli::{Cli, ProjectsAction, ProjectsCommand},
    config::AppConfig,
    error::CommandError,
};

use super::shared::{authenticated_client, list_result};

pub async fn execute_projects_command(
    command: &ProjectsCommand,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value, CommandError> {
    let client = authenticated_client(args, config).await?;
    match &command.action {
        ProjectsAction::List(list) => {
            let projects = crate::ops::projects::list(&client, list.limit)
                .await
                .map_err(CommandError::from)?;
            list_result(&projects, args.full)
        }
    }
}
