use serde_json::Value;

use crate::tui::Record;

pub(crate) fn map_app_record(app: Value) -> Record {
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

pub(crate) fn map_user_record(user: Value) -> Record {
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

pub(crate) fn map_idp_record(idp: Value) -> Record {
    Record {
        id: string_field(&idp, "id", "missing-id"),
        name: string_field(&idp, "name", "unnamed-idp"),
        kind: string_field(&idp, "state", "unknown"),
        summary: string_field(&idp, "type", "provider"),
        detail: "manual credentials".to_string(),
        changed_at: "configured".to_string(),
    }
}

pub(crate) fn app_creation_summary(name: &str, value: &Value) -> String {
    format!(
        "{} -> {}",
        name,
        value
            .get("clientId")
            .and_then(|v| v.as_str())
            .unwrap_or("created")
    )
}

pub(crate) fn string_field(value: &Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(|field| field.as_str())
        .unwrap_or(fallback)
        .to_string()
}
