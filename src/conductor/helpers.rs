use anyhow::{Context, Result};
use serde_json::Value;

use crate::{
    client::ZitadelClient,
    config::UserTemplate,
    tui::{CanvasMode, FieldKind, FormField, FormState, MessageState, Record},
};

pub(super) fn map_app_record(app: Value) -> Record {
    let kind = app
        .get("oidcConfig")
        .and_then(|oidc| oidc.get("authMethodType"))
        .and_then(|value| value.as_str())
        .map(|value| {
            if value == "OIDC_AUTH_METHOD_TYPE_NONE" {
                "public".to_string()
            } else {
                "confidential".to_string()
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    let client_id = app
        .get("oidcConfig")
        .and_then(|oidc| oidc.get("clientId"))
        .and_then(|value| value.as_str())
        .unwrap_or("missing-client-id")
        .to_string();
    let redirect_count = app
        .get("oidcConfig")
        .and_then(|oidc| oidc.get("redirectUris"))
        .and_then(|value| value.as_array())
        .map(|uris| uris.len())
        .unwrap_or(0);
    Record {
        id: string_field(&app, "id", "missing-id"),
        name: string_field(&app, "name", "unnamed"),
        kind,
        summary: format!("{redirect_count} redirects"),
        detail: client_id,
        changed_at: string_field(&app, "state", "unknown"),
    }
}

pub(super) fn map_user_record(user: Value) -> Record {
    Record {
        id: string_field(&user, "id", "missing-id"),
        name: string_field(&user, "userName", "unknown-user"),
        kind: string_field(&user, "state", "unknown"),
        summary: user
            .get("human")
            .and_then(|human| human.get("email"))
            .and_then(|email| email.get("email"))
            .and_then(|email| email.as_str())
            .unwrap_or("no email")
            .to_string(),
        detail: user
            .get("human")
            .and_then(|human| human.get("profile"))
            .and_then(|profile| profile.get("displayName"))
            .and_then(|display_name| display_name.as_str())
            .unwrap_or("human user")
            .to_string(),
        changed_at: "loaded".to_string(),
    }
}

pub(super) fn map_idp_record(idp: Value) -> Record {
    Record {
        id: string_field(&idp, "id", "missing-id"),
        name: string_field(&idp, "name", "unnamed-idp"),
        kind: string_field(&idp, "state", "unknown"),
        summary: string_field(&idp, "type", "provider"),
        detail: "manual credentials".to_string(),
        changed_at: "configured".to_string(),
    }
}

pub(super) async fn resolve_project_id(
    client: &ZitadelClient,
    cli_project: Option<String>,
    config_project: Option<String>,
) -> Result<String> {
    if let Some(project) = cli_project.or(config_project) {
        return Ok(project);
    }

    client
        .get_default_project()
        .await?
        .get("id")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .context("failed to determine default project id")
}

pub(super) fn error_mode(title: &str, message: &str) -> CanvasMode {
    CanvasMode::Error(MessageState {
        title: title.to_string(),
        lines: vec![message.to_string()],
    })
}

pub(super) fn text_field(key: &'static str, label: &str, value: &str, help: &str) -> FormField {
    FormField {
        key,
        label: label.to_string(),
        value: value.to_string(),
        kind: FieldKind::Text,
        help: help.to_string(),
    }
}

pub(super) fn secret_field(
    key: &'static str,
    label: &str,
    value: &str,
    help: &str,
) -> FormField {
    FormField {
        key,
        label: label.to_string(),
        value: value.to_string(),
        kind: FieldKind::Secret,
        help: help.to_string(),
    }
}

pub(super) fn toggle_field(
    key: &'static str,
    label: &str,
    value: bool,
    help: &str,
) -> FormField {
    FormField {
        key,
        label: label.to_string(),
        value: if value { "true" } else { "false" }.to_string(),
        kind: FieldKind::Toggle,
        help: help.to_string(),
    }
}

pub(super) fn choice_field(
    key: &'static str,
    label: &str,
    value: &str,
    options: Vec<String>,
    help: &str,
) -> FormField {
    FormField {
        key,
        label: label.to_string(),
        value: value.to_string(),
        kind: FieldKind::Choice(options),
        help: help.to_string(),
    }
}

pub(super) fn checkbox_field(
    key: &'static str,
    label: &str,
    value: bool,
    help: &str,
) -> FormField {
    FormField {
        key,
        label: label.to_string(),
        value: if value { "true" } else { "false" }.to_string(),
        kind: FieldKind::Checkbox,
        help: help.to_string(),
    }
}

pub(super) fn form_value(form: &FormState, key: &str) -> String {
    form.fields
        .iter()
        .find(|field| field.key == key)
        .map(|field| field.value.clone())
        .unwrap_or_default()
}

pub(super) fn optional_value(form: &FormState, key: &str) -> Option<String> {
    let value = form_value(form, key);
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

pub(super) fn bool_value(form: &FormState, key: &str) -> bool {
    matches!(form_value(form, key).as_str(), "true" | "1" | "yes" | "on")
}

pub(super) fn split_csv(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn app_creation_summary(name: &str, value: &Value) -> String {
    format!(
        "{} -> {}",
        name,
        value
            .get("clientId")
            .and_then(|v| v.as_str())
            .unwrap_or("created")
    )
}

pub(super) async fn create_quick_user(
    client: &ZitadelClient,
    user: UserTemplate,
) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    let result = client
        .create_human_user(&user.email, &user.first_name, &user.last_name, None)
        .await?;
    lines.push(format!("Created {}", user.email));
    if user.admin {
        if let Some(user_id) = result.get("userId").and_then(|v| v.as_str()) {
            client.grant_iam_owner(user_id).await?;
            lines.push(format!("Granted IAM_OWNER to {}", user.email));
        }
    }
    Ok(lines)
}

pub(super) fn checkbox_enabled(value: &str) -> bool {
    matches!(value, "true" | "1" | "yes" | "on")
}

pub(super) fn string_field(value: &Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(|field| field.as_str())
        .unwrap_or(fallback)
        .to_string()
}
