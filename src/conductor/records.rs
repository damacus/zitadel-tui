use crate::{
    api::models::{App, CreateAppResult, Idp, User},
    tui::Record,
};

impl From<&App> for Record {
    fn from(app: &App) -> Self {
        let (summary, detail) = if let Some(oidc_config) = &app.oidc_config {
            let client_id = oidc_config
                .client_id
                .clone()
                .unwrap_or_else(|| "missing-client-id".to_string());
            let redirect_count = oidc_config
                .redirect_uris
                .as_ref()
                .map(|uris| uris.len())
                .unwrap_or(0);

            (format!("{redirect_count} redirects"), client_id)
        } else if let Some(api_config) = &app.api_config {
            let client_id = api_config
                .client_id
                .clone()
                .unwrap_or_else(|| "missing-client-id".to_string());
            let auth_method = api_config
                .auth_method_type
                .clone()
                .unwrap_or_else(|| "unknown-auth-method".to_string());

            (auth_method, client_id)
        } else {
            ("0 redirects".to_string(), "missing-client-id".to_string())
        };
        Record {
            id: app.id.clone().unwrap_or_else(|| "missing-id".to_string()),
            name: app.name.clone().unwrap_or_else(|| "unnamed".to_string()),
            kind: app.kind().to_string(),
            summary,
            detail,
            changed_at: app.state.clone().unwrap_or_else(|| "unknown".to_string()),
        }
    }
}

impl From<&User> for Record {
    fn from(user: &User) -> Self {
        let is_machine = user.machine.is_some();
        let user_type = if is_machine { "machine" } else { "human" };
        let state = user.state.clone().unwrap_or_else(|| "unknown".to_string());
        let name = if is_machine {
            user.machine
                .as_ref()
                .and_then(|machine| machine.name.clone())
                .unwrap_or_else(|| {
                    user.user_name
                        .clone()
                        .unwrap_or_else(|| "unknown-user".to_string())
                })
        } else {
            user.user_name
                .clone()
                .unwrap_or_else(|| "unknown-user".to_string())
        };
        let summary = if is_machine {
            user.preferred_login_name
                .clone()
                .or_else(|| {
                    user.login_names
                        .as_ref()
                        .and_then(|login_names| login_names.first().cloned())
                })
                .or_else(|| user.user_name.clone())
                .unwrap_or_else(|| "no login".to_string())
        } else {
            user.human
                .as_ref()
                .and_then(|human| human.email.as_ref())
                .and_then(|email| email.email.clone())
                .unwrap_or_else(|| "no email".to_string())
        };
        let detail = if is_machine {
            "machine user".to_string()
        } else {
            user.human
                .as_ref()
                .and_then(|human| human.profile.as_ref())
                .and_then(|profile| profile.display_name.clone())
                .unwrap_or_else(|| "human user".to_string())
        };

        Record {
            id: user.id.clone().unwrap_or_else(|| "missing-id".to_string()),
            name,
            kind: format!("{user_type} {state}"),
            summary,
            detail,
            changed_at: "loaded".to_string(),
        }
    }
}

impl From<&Idp> for Record {
    fn from(idp: &Idp) -> Self {
        Record {
            id: idp.id.clone().unwrap_or_else(|| "missing-id".to_string()),
            name: idp
                .name
                .clone()
                .unwrap_or_else(|| "unnamed-idp".to_string()),
            kind: idp.state.clone().unwrap_or_else(|| "unknown".to_string()),
            summary: idp.r#type.clone().unwrap_or_else(|| "provider".to_string()),
            detail: "manual credentials".to_string(),
            changed_at: "configured".to_string(),
        }
    }
}

pub(crate) fn app_creation_summary(name: &str, result: &CreateAppResult) -> String {
    format!(
        "{} -> {}",
        name,
        result.client_id.as_deref().unwrap_or("created")
    )
}
