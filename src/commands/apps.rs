use anyhow::{Context, Result};
use serde_json::Value;

use crate::{
    cli::{AppsAction, AppsCommand, Cli},
    config::AppConfig,
};

use super::shared::{authenticated_client, resolved_project_id};

pub async fn execute_apps_command(
    command: &AppsCommand,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value> {
    let client = authenticated_client(args, config).await?;
    let project_id = resolved_project_id(&client, args, config).await?;

    match &command.action {
        AppsAction::List => Ok(serde_json::to_value(client.list_apps(&project_id).await?)?),
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
            client
                .create_oidc_app(&project_id, &name, redirect_uris, public)
                .await
        }
        AppsAction::CreateNative(native) => {
            let result = client
                .create_native_app(&project_id, &native.name, native.device_code)
                .await?;

            if native.device_code {
                remember_device_client_id(config, &result)?;
            }

            Ok(result)
        }
        AppsAction::Delete(delete) => client.delete_app(&project_id, &delete.app_id).await,
        AppsAction::RegenerateSecret(regen) => {
            let mut result = client.regenerate_secret(&project_id, &regen.app_id).await?;
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

            let mut created = Vec::new();
            for name in names {
                let template = templates
                    .apps
                    .get(&name)
                    .with_context(|| format!("template {name} not found"))?;
                created.push(
                    client
                        .create_oidc_app(
                            &project_id,
                            &name,
                            template.redirect_uris.clone(),
                            template.public,
                        )
                        .await?,
                );
            }
            Ok(Value::Array(created))
        }
    }
}

fn remember_device_client_id(config: &AppConfig, result: &Value) -> Result<()> {
    let updated = config_with_device_client_id(config, result)?;
    updated
        .save_to_canonical_path()
        .context("failed to save native app clientId as device_client_id")?;
    Ok(())
}

fn config_with_device_client_id(config: &AppConfig, result: &Value) -> Result<AppConfig> {
    let Some(client_id) = device_client_id_from_create_response(result) else {
        anyhow::bail!("device-code native app response did not include clientId");
    };

    let mut updated = config.clone();
    updated.device_client_id = Some(client_id.to_string());
    Ok(updated)
}

fn device_client_id_from_create_response(result: &Value) -> Option<&str> {
    result
        .get("clientId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|client_id| !client_id.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_device_client_id_from_create_native_response() {
        let result = serde_json::json!({
            "appId": "app-1",
            "clientId": " client-1 "
        });

        assert_eq!(
            device_client_id_from_create_response(&result),
            Some("client-1")
        );
    }

    #[test]
    fn rejects_missing_device_client_id_from_create_native_response() {
        let result = serde_json::json!({ "appId": "app-1" });

        assert_eq!(device_client_id_from_create_response(&result), None);
    }

    #[test]
    fn updates_config_with_created_device_client_id() {
        let config = AppConfig {
            zitadel_url: Some("https://zitadel.example.com".to_string()),
            device_client_id: Some("old-client".to_string()),
            ..AppConfig::default()
        };
        let result = serde_json::json!({ "clientId": "new-client" });

        let updated = config_with_device_client_id(&config, &result).unwrap();

        assert_eq!(
            updated.zitadel_url.as_deref(),
            Some("https://zitadel.example.com")
        );
        assert_eq!(updated.device_client_id.as_deref(), Some("new-client"));
    }
}
