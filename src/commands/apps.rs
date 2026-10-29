use anyhow::{Context, Result};
use serde_json::Value;

use crate::{
    api::models::CreateAppResult,
    cli::{AppsAction, AppsCommand, Cli},
    config::AppConfig,
};

use super::shared::authenticated_client;

pub async fn execute_apps_command(
    command: &AppsCommand,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value> {
    let client = authenticated_client(args, config).await?;
    let project_id =
        crate::ops::resolve_project_id(&client, args.project_id.clone(), config.project_id.clone())
            .await?;

    match &command.action {
        AppsAction::List => Ok(serde_json::to_value(
            crate::ops::apps::list(&client, &project_id).await?,
        )?),
        AppsAction::Create(create) => {
            let templates = config.templates()?;
            let (name, redirect_uris, public) = if let Some(template_name) = &create.template {
                let template = templates
                    .apps
                    .get(template_name)
                    .with_context(|| format!("template {template_name} not found"))?;
                (
                    template_name.clone(),
                    template.redirect_uris.clone(),
                    template.public,
                )
            } else {
                if create.redirect_uris.is_empty() {
                    anyhow::bail!(
                        "--redirect-uris is required for apps create when not using --template"
                    );
                }
                (
                    create
                        .name
                        .clone()
                        .context("app name is required when not using --template")?,
                    create.redirect_uris.clone(),
                    create.public,
                )
            };
            Ok(serde_json::to_value(
                crate::ops::apps::create_oidc(&client, &project_id, &name, redirect_uris, public)
                    .await?,
            )?)
        }
        AppsAction::CreateNative(native) => {
            let result = crate::ops::apps::create_native(
                &client,
                &project_id,
                &native.name,
                native.device_code,
            )
            .await?;

            if native.device_code {
                remember_device_client_id(config, &result)?;
            }

            Ok(serde_json::to_value(&result)?)
        }
        AppsAction::Delete(delete) => {
            crate::ops::apps::delete(&client, &project_id, &delete.app_id).await?;
            Ok(serde_json::json!({}))
        }
        AppsAction::RegenerateSecret(regen) => {
            let secret =
                crate::ops::apps::regenerate_secret(&client, &project_id, &regen.app_id).await?;
            let mut result = serde_json::to_value(&secret)?;
            if let Some(client_id) = &regen.client_id {
                result["clientId"] = Value::String(client_id.clone());
            }
            Ok(result)
        }
        AppsAction::QuickSetup(quick) => {
            let templates = config.templates()?;
            let names = if quick.names.is_empty() {
                templates.apps.keys().cloned().collect::<Vec<_>>()
            } else {
                quick.names.clone()
            };

            let created =
                crate::ops::apps::quick_setup(&client, &project_id, &templates.apps, &names)
                    .await?;
            Ok(serde_json::to_value(&created)?)
        }
    }
}

fn remember_device_client_id(config: &AppConfig, result: &CreateAppResult) -> Result<()> {
    let updated = config_with_device_client_id(config, result)?;
    updated
        .save_to_canonical_path()
        .context("failed to save native app clientId as device_client_id")?;
    Ok(())
}

fn config_with_device_client_id(config: &AppConfig, result: &CreateAppResult) -> Result<AppConfig> {
    let Some(client_id) = device_client_id_from_create_response(result) else {
        anyhow::bail!("device-code native app response did not include clientId");
    };

    let mut updated = config.clone();
    updated.device_client_id = Some(client_id.to_string());
    Ok(updated)
}

fn device_client_id_from_create_response(result: &CreateAppResult) -> Option<&str> {
    result
        .client_id
        .as_deref()
        .map(str::trim)
        .filter(|client_id| !client_id.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_app_result(value: serde_json::Value) -> CreateAppResult {
        serde_json::from_value(value).expect("fixture decodes into CreateAppResult")
    }

    #[test]
    fn extracts_device_client_id_from_create_native_response() {
        let result = create_app_result(serde_json::json!({
            "appId": "app-1",
            "clientId": " client-1 "
        }));

        assert_eq!(
            device_client_id_from_create_response(&result),
            Some("client-1")
        );
    }

    #[test]
    fn rejects_missing_device_client_id_from_create_native_response() {
        let result = create_app_result(serde_json::json!({ "appId": "app-1" }));

        assert_eq!(device_client_id_from_create_response(&result), None);
    }

    #[test]
    fn updates_config_with_created_device_client_id() {
        let config = AppConfig {
            zitadel_url: Some("https://zitadel.example.com".to_string()),
            device_client_id: Some("old-client".to_string()),
            ..AppConfig::default()
        };
        let result = create_app_result(serde_json::json!({ "clientId": "new-client" }));

        let updated = config_with_device_client_id(&config, &result).unwrap();

        assert_eq!(
            updated.zitadel_url.as_deref(),
            Some("https://zitadel.example.com")
        );
        assert_eq!(updated.device_client_id.as_deref(), Some("new-client"));
    }
}
