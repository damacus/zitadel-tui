use serde_json::Value;

use crate::tui::Record;

pub(crate) fn map_app_record(app: Value) -> Record {
    let (kind, summary, detail) = if let Some(oidc_config) = app.get("oidcConfig") {
        let kind = classify_oidc_kind(Some(oidc_config));
        let client_id = oidc_config
            .get("clientId")
            .and_then(|value| value.as_str())
            .unwrap_or("missing-client-id")
            .to_string();
        let redirect_count = oidc_config
            .get("redirectUris")
            .and_then(|value| value.as_array())
            .map(|uris| uris.len())
            .unwrap_or(0);

        (kind, format!("{redirect_count} redirects"), client_id)
    } else if let Some(api_config) = app.get("apiConfig") {
        let client_id = api_config
            .get("clientId")
            .and_then(|value| value.as_str())
            .unwrap_or("missing-client-id")
            .to_string();
        let auth_method = api_config
            .get("authMethodType")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown-auth-method")
            .to_string();

        ("api".to_string(), auth_method, client_id)
    } else {
        (
            "unknown".to_string(),
            "0 redirects".to_string(),
            "missing-client-id".to_string(),
        )
    };
    Record {
        id: string_field(&app, "id", "missing-id"),
        name: string_field(&app, "name", "unnamed"),
        kind,
        summary,
        detail,
        changed_at: string_field(&app, "state", "unknown"),
    }
}

fn classify_oidc_kind(oidc_config: Option<&Value>) -> String {
    let Some(oidc) = oidc_config else {
        return "unknown".to_string();
    };

    if has_grant_type(oidc, "OIDC_GRANT_TYPE_DEVICE_CODE") {
        return "device-code".to_string();
    }

    if string_field_value(oidc, "appType") == Some("OIDC_APP_TYPE_NATIVE") {
        return "native".to_string();
    }

    if let Some(auth_method_type) = string_field_value(oidc, "authMethodType") {
        if auth_method_type == "OIDC_AUTH_METHOD_TYPE_NONE" {
            return "public".to_string();
        }

        return "confidential".to_string();
    }

    match string_field_value(oidc, "appType") {
        Some("OIDC_APP_TYPE_USER_AGENT") => "public".to_string(),
        Some("OIDC_APP_TYPE_WEB") => "confidential".to_string(),
        Some(_) => "oidc".to_string(),
        None if has_useful_oidc_fields(oidc) => "oidc".to_string(),
        None => "unknown".to_string(),
    }
}

fn has_grant_type(oidc: &Value, grant_type: &str) -> bool {
    oidc.get("grantTypes")
        .and_then(|value| value.as_array())
        .is_some_and(|grant_types| {
            grant_types
                .iter()
                .filter_map(|value| value.as_str())
                .any(|value| value == grant_type)
        })
}

fn has_useful_oidc_fields(oidc: &Value) -> bool {
    has_string_field(oidc, "clientId")
        || has_non_empty_array_field(oidc, "redirectUris")
        || has_non_empty_array_field(oidc, "grantTypes")
}

fn has_string_field(value: &Value, key: &str) -> bool {
    string_field_value(value, key).is_some_and(|value| !value.is_empty())
}

fn has_non_empty_array_field(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(|field| field.as_array())
        .is_some_and(|values| !values.is_empty())
}

fn string_field_value<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(|field| field.as_str())
}

pub(crate) fn map_user_record(user: Value) -> Record {
    let is_machine = user.get("machine").is_some();
    let user_type = if is_machine { "machine" } else { "human" };
    let state = string_field(&user, "state", "unknown");
    let name = if is_machine {
        user.get("machine")
            .and_then(|machine| machine.get("name"))
            .and_then(|name| name.as_str())
            .unwrap_or_else(|| {
                user.get("userName")
                    .and_then(|user_name| user_name.as_str())
                    .unwrap_or("unknown-user")
            })
            .to_string()
    } else {
        string_field(&user, "userName", "unknown-user")
    };
    let summary = if is_machine {
        user.get("preferredLoginName")
            .and_then(|login| login.as_str())
            .or_else(|| {
                user.get("loginNames")
                    .and_then(|login_names| login_names.as_array())
                    .and_then(|login_names| login_names.first())
                    .and_then(|login| login.as_str())
            })
            .or_else(|| {
                user.get("userName")
                    .and_then(|user_name| user_name.as_str())
            })
            .unwrap_or("no login")
            .to_string()
    } else {
        user.get("human")
            .and_then(|human| human.get("email"))
            .and_then(|email| email.get("email"))
            .and_then(|email| email.as_str())
            .unwrap_or("no email")
            .to_string()
    };
    let detail = if is_machine {
        "machine user".to_string()
    } else {
        user.get("human")
            .and_then(|human| human.get("profile"))
            .and_then(|profile| profile.get("displayName"))
            .and_then(|display_name| display_name.as_str())
            .unwrap_or("human user")
            .to_string()
    };

    Record {
        id: string_field(&user, "id", "missing-id"),
        name,
        kind: format!("{user_type} {state}"),
        summary,
        detail,
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
