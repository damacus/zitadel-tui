use serde_json::{json, Value};

use crate::{
    api::models::CreateAppResult,
    cli::{AppsAction, AppsCommand, Cli, CreateAppArgs},
    config::{AppConfig, TemplatesFile},
    error::{CommandError, ErrorKind},
};

use super::shared::authenticated_client;

/// `apps create`/`quick-setup` template lookup failure — `NotFound` with
/// the lookup echoed in `details`.
fn template_not_found(name: &str) -> CommandError {
    CommandError::with_details(
        ErrorKind::NotFound,
        format!("template {name} not found"),
        json!({ "lookup": { "field": "template", "value": name } }),
    )
}

/// Cheap, local input validation for `apps create` — runs before any
/// credential resolution or network call so `error.kind` is
/// deterministic.
fn validate_create(create: &CreateAppArgs, templates: &TemplatesFile) -> Result<(), CommandError> {
    if let Some(template_name) = &create.template {
        if !templates.apps.contains_key(template_name) {
            return Err(template_not_found(template_name));
        }
        return Ok(());
    }
    if create.redirect_uris.is_empty() {
        return Err(CommandError::usage(
            "--redirect-uris is required for apps create when not using --template",
        ));
    }
    if create.name.is_none() {
        return Err(CommandError::usage(
            "app name is required when not using --template",
        ));
    }
    Ok(())
}

/// Cheap, local template-name validation for `apps quick-setup`.
fn validate_template_names(
    names: &[String],
    templates: &TemplatesFile,
) -> Result<(), CommandError> {
    for name in names {
        if !templates.apps.contains_key(name) {
            return Err(template_not_found(name));
        }
    }
    Ok(())
}

pub async fn execute_apps_command(
    command: &AppsCommand,
    args: &Cli,
    config: &AppConfig,
) -> Result<Value, CommandError> {
    // Templates + input validation first: cheap and local, so
    // `error.kind` is deterministic regardless of credentials or API
    // reachability.
    let templates = if matches!(
        command.action,
        AppsAction::Create(_) | AppsAction::QuickSetup(_)
    ) {
        Some(
            config
                .templates()
                .map_err(|error| CommandError::config(error.to_string()))?,
        )
    } else {
        None
    };
    match &command.action {
        AppsAction::Create(create) => {
            validate_create(create, templates.as_ref().expect("templates loaded"))?
        }
        AppsAction::QuickSetup(quick) => {
            validate_template_names(&quick.names, templates.as_ref().expect("templates loaded"))?
        }
        _ => {}
    }

    let client = authenticated_client(args, config).await?;
    let project_id =
        crate::ops::resolve_project_id(&client, args.project_id.clone(), config.project_id.clone())
            .await
            .map_err(CommandError::from)?;

    match &command.action {
        AppsAction::List => Ok(serde_json::to_value(
            crate::ops::apps::list(&client, &project_id)
                .await
                .map_err(CommandError::from)?,
        )?),
        AppsAction::Create(create) => {
            let templates = templates.expect("templates loaded for apps create");
            let (name, redirect_uris, public) = if let Some(template_name) = &create.template {
                // Validated above; re-check defensively to stay panic-free.
                let template = templates
                    .apps
                    .get(template_name)
                    .ok_or_else(|| template_not_found(template_name))?;
                (
                    template_name.clone(),
                    template.redirect_uris.clone(),
                    template.public,
                )
            } else {
                if create.redirect_uris.is_empty() {
                    return Err(CommandError::usage(
                        "--redirect-uris is required for apps create when not using --template",
                    ));
                }
                let name = create.name.clone().ok_or_else(|| {
                    CommandError::usage("app name is required when not using --template")
                })?;
                (name, create.redirect_uris.clone(), create.public)
            };
            Ok(serde_json::to_value(
                crate::ops::apps::create_oidc(&client, &project_id, &name, redirect_uris, public)
                    .await
                    .map_err(CommandError::from)?,
            )?)
        }
        AppsAction::CreateNative(native) => {
            let result = crate::ops::apps::create_native(
                &client,
                &project_id,
                &native.name,
                native.device_code,
            )
            .await
            .map_err(CommandError::from)?;

            if native.device_code {
                remember_device_client_id(config, &result)?;
            }

            Ok(serde_json::to_value(&result)?)
        }
        AppsAction::Delete(delete) => {
            crate::ops::apps::delete(&client, &project_id, &delete.app_id)
                .await
                .map_err(CommandError::from)?;
            Ok(serde_json::json!({}))
        }
        AppsAction::RegenerateSecret(regen) => {
            let secret = crate::ops::apps::regenerate_secret(&client, &project_id, &regen.app_id)
                .await
                .map_err(CommandError::from)?;
            let mut result = serde_json::to_value(&secret)?;
            if let Some(client_id) = &regen.client_id {
                result["clientId"] = Value::String(client_id.clone());
            }
            Ok(result)
        }
        AppsAction::QuickSetup(quick) => {
            let templates = templates.expect("templates loaded for apps quick-setup");
            let names = if quick.names.is_empty() {
                templates.apps.keys().cloned().collect::<Vec<_>>()
            } else {
                quick.names.clone()
            };

            let created =
                crate::ops::apps::quick_setup(&client, &project_id, &templates.apps, &names)
                    .await
                    .map_err(CommandError::from)?;
            Ok(serde_json::to_value(&created)?)
        }
    }
}

fn remember_device_client_id(
    config: &AppConfig,
    result: &CreateAppResult,
) -> Result<(), CommandError> {
    let updated = config_with_device_client_id(config, result).map_err(CommandError::from)?;
    updated
        .save_to_canonical_path()
        .map_err(|error| CommandError::config(error.to_string()))?;
    Ok(())
}

fn config_with_device_client_id(
    config: &AppConfig,
    result: &CreateAppResult,
) -> anyhow::Result<AppConfig> {
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

    #[test]
    fn template_not_found_is_typed_not_found_with_lookup() {
        let error = template_not_found("grafana");

        assert_eq!(error.kind, ErrorKind::NotFound);
        assert_eq!(error.message, "template grafana not found");
        assert_eq!(
            error.details,
            Some(json!({ "lookup": { "field": "template", "value": "grafana" } }))
        );
    }
}
