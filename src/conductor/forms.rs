use crate::{
    config::UserTemplate,
    tui::{CanvasMode, ConfirmState, FormField, FormState, PendingAction, Record, ResourceKind},
};

use super::{support::error_mode, TuiConductor};

impl TuiConductor {
    pub fn begin_action(
        &self,
        resource: ResourceKind,
        action_index: usize,
        selected_record: Option<&Record>,
    ) -> CanvasMode {
        match resource {
            ResourceKind::Applications => match action_index {
                0 => CanvasMode::EditForm(FormState {
                    title: "Create application".to_string(),
                    description: "Create a new OIDC application from a template or manually."
                        .to_string(),
                    submit_label: "[Enter] create application".to_string(),
                    selected_field: 0,
                    pending: PendingAction::CreateApplication,
                    fields: vec![
                        choice_field(
                            "mode",
                            "Mode",
                            "manual",
                            vec!["manual".to_string(), "template".to_string()],
                            "Choose manual fields or template-driven defaults.",
                        ),
                        text_field("name", "Name", "", "Application name"),
                        text_field(
                            "redirect_uris",
                            "Redirect URIs",
                            "",
                            "Comma-separated callback URLs",
                        ),
                        toggle_field("public", "Public", false, "Public clients have no secret"),
                        text_field(
                            "template",
                            "Template",
                            "",
                            "Template name when mode is template",
                        ),
                    ],
                }),
                1 => {
                    if let Some(record) = selected_record {
                        CanvasMode::Confirm(ConfirmState {
                            title: "Regenerate client secret".to_string(),
                            lines: vec![
                                format!("Application: {}", record.name),
                                "This invalidates the current confidential client secret."
                                    .to_string(),
                            ],
                            submit_label: "[Enter] regenerate secret".to_string(),
                            pending: PendingAction::RegenerateSecret {
                                app_id: record.id.clone(),
                                name: record.name.clone(),
                                client_id: record.detail.clone(),
                            },
                        })
                    } else {
                        error_mode("No application selected", "Choose an application first.")
                    }
                }
                2 => {
                    if let Some(record) = selected_record {
                        CanvasMode::Confirm(ConfirmState {
                            title: "Delete application".to_string(),
                            lines: vec![
                                format!("Application: {}", record.name),
                                "This permanently deletes the selected application.".to_string(),
                            ],
                            submit_label: "[Enter] delete application".to_string(),
                            pending: PendingAction::DeleteApplication {
                                app_id: record.id.clone(),
                                name: record.name.clone(),
                            },
                        })
                    } else {
                        error_mode("No application selected", "Choose an application first.")
                    }
                }
                3 => CanvasMode::EditForm(FormState {
                    title: "Quick setup applications".to_string(),
                    description: "Select predefined application templates to create.".to_string(),
                    submit_label: "[Enter] create selected apps".to_string(),
                    selected_field: 0,
                    pending: PendingAction::QuickSetupApplications,
                    fields: self
                        .templates
                        .apps
                        .keys()
                        .map(|name| {
                            checkbox_field(
                                "template_app",
                                name,
                                false,
                                "Toggle application creation",
                            )
                        })
                        .collect(),
                }),
                _ => CanvasMode::Browse,
            },
            ResourceKind::Users => match action_index {
                0 => CanvasMode::EditForm(FormState {
                    title: "Create user".to_string(),
                    description: "Create a normal human user.".to_string(),
                    submit_label: "[Enter] create user".to_string(),
                    selected_field: 0,
                    pending: PendingAction::CreateUser,
                    fields: vec![
                        text_field("email", "Email", "", "Email address"),
                        text_field("first_name", "First name", "", "Given name"),
                        text_field("last_name", "Last name", "", "Family name"),
                        text_field("username", "Username", "", "Optional username override"),
                    ],
                }),
                1 => CanvasMode::EditForm(FormState {
                    title: "Create admin user".to_string(),
                    description: "Create a local admin import with temporary password.".to_string(),
                    submit_label: "[Enter] create admin".to_string(),
                    selected_field: 0,
                    pending: PendingAction::CreateAdminUser,
                    fields: vec![
                        text_field("username", "Username", "admin", "Login username"),
                        text_field("first_name", "First name", "Admin", "Given name"),
                        text_field("last_name", "Last name", "User", "Family name"),
                        text_field("email", "Email", "", "Email address"),
                        secret_field("password", "Password", "", "Temporary password"),
                    ],
                }),
                2 => {
                    if let Some(record) = selected_record {
                        CanvasMode::Confirm(ConfirmState {
                            title: "Grant IAM_OWNER".to_string(),
                            lines: vec![
                                format!("User: {}", record.name),
                                "This grants full instance administration rights.".to_string(),
                            ],
                            submit_label: "[Enter] grant IAM_OWNER".to_string(),
                            pending: PendingAction::GrantIamOwner {
                                user_id: record.id.clone(),
                                username: record.name.clone(),
                            },
                        })
                    } else {
                        error_mode("No user selected", "Choose a user first.")
                    }
                }
                3 => CanvasMode::EditForm(FormState {
                    title: "Quick setup users".to_string(),
                    description: "Select predefined users to create.".to_string(),
                    submit_label: "[Enter] create selected users".to_string(),
                    selected_field: 0,
                    pending: PendingAction::QuickSetupUsers,
                    fields: self
                        .templates
                        .users
                        .iter()
                        .map(|user| {
                            checkbox_field(
                                "template_user",
                                &format!(
                                    "{} ({}){}",
                                    user.email,
                                    user.first_name,
                                    if user.admin { " admin" } else { "" }
                                ),
                                false,
                                "Toggle user creation",
                            )
                        })
                        .collect(),
                }),
                _ => CanvasMode::Browse,
            },
            ResourceKind::Idps => CanvasMode::EditForm(FormState {
                title: "Configure Google IDP".to_string(),
                description: "Configure Google with manual credentials only.".to_string(),
                submit_label: "[Enter] configure Google".to_string(),
                selected_field: 0,
                pending: PendingAction::ConfigureGoogleIdp,
                fields: vec![
                    text_field("name", "Name", "Google", "Display name"),
                    text_field("client_id", "Client ID", "", "Google OAuth client ID"),
                    secret_field(
                        "client_secret",
                        "Client secret",
                        "",
                        "Google OAuth client secret",
                    ),
                ],
            }),
            ResourceKind::Auth => CanvasMode::Setup(FormState {
                title: "Run setup".to_string(),
                description: "Validate and save auth, host, project, and templates path."
                    .to_string(),
                submit_label: "[Enter] validate and save".to_string(),
                selected_field: 0,
                pending: PendingAction::ValidateAuthSetup,
                fields: vec![
                    text_field("host", "Host", &self.host, "Zitadel base URL"),
                    text_field(
                        "project",
                        "Project",
                        &self.project,
                        "Optional default project",
                    ),
                    choice_field(
                        "auth_method",
                        "Auth method",
                        if self.config.service_account_file.is_some() {
                            "Service account"
                        } else {
                            "PAT"
                        },
                        vec![
                            "PAT".to_string(),
                            "Service account".to_string(),
                            "OAuth device (placeholder)".to_string(),
                        ],
                        "PAT and service account are live in this slice.",
                    ),
                    secret_field(
                        "token",
                        "PAT",
                        self.config.pat.as_deref().unwrap_or(""),
                        "Used when auth method is PAT",
                    ),
                    text_field(
                        "service_account_file",
                        "Service account",
                        &self
                            .config
                            .service_account_file
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_default(),
                        "Used when auth method is service account",
                    ),
                    text_field(
                        "templates_path",
                        "Templates path",
                        &self
                            .config
                            .apps_config_file
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_default(),
                        "Optional apps/users YAML path",
                    ),
                ],
            }),
            ResourceKind::Config => match action_index {
                0 => CanvasMode::EditForm(FormState {
                    title: "Edit config".to_string(),
                    description: "Update saved host, project, and templates path.".to_string(),
                    submit_label: "[Enter] save config".to_string(),
                    selected_field: 0,
                    pending: PendingAction::SaveConfig,
                    fields: vec![
                        text_field("host", "Host", &self.host, "Saved Zitadel URL"),
                        text_field("project", "Project", &self.project, "Saved default project"),
                        text_field(
                            "templates_path",
                            "Templates path",
                            &self
                                .config
                                .apps_config_file
                                .as_ref()
                                .map(|path| path.display().to_string())
                                .unwrap_or_default(),
                            "Saved templates YAML path",
                        ),
                    ],
                }),
                _ => CanvasMode::Browse,
            },
        }
    }
}

pub(crate) fn text_field(key: &'static str, label: &str, value: &str, help: &str) -> FormField {
    FormField {
        key,
        label: label.to_string(),
        value: value.to_string(),
        kind: crate::tui::FieldKind::Text,
        help: help.to_string(),
    }
}

pub(crate) fn secret_field(key: &'static str, label: &str, value: &str, help: &str) -> FormField {
    FormField {
        key,
        label: label.to_string(),
        value: value.to_string(),
        kind: crate::tui::FieldKind::Secret,
        help: help.to_string(),
    }
}

pub(crate) fn toggle_field(key: &'static str, label: &str, value: bool, help: &str) -> FormField {
    FormField {
        key,
        label: label.to_string(),
        value: if value { "true" } else { "false" }.to_string(),
        kind: crate::tui::FieldKind::Toggle,
        help: help.to_string(),
    }
}

pub(crate) fn choice_field(
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
        kind: crate::tui::FieldKind::Choice(options),
        help: help.to_string(),
    }
}

pub(crate) fn checkbox_field(key: &'static str, label: &str, value: bool, help: &str) -> FormField {
    FormField {
        key,
        label: label.to_string(),
        value: if value { "true" } else { "false" }.to_string(),
        kind: crate::tui::FieldKind::Checkbox,
        help: help.to_string(),
    }
}

pub(crate) fn form_value(form: &FormState, key: &str) -> String {
    form.fields
        .iter()
        .find(|field| field.key == key)
        .map(|field| field.value.clone())
        .unwrap_or_default()
}

pub(crate) fn optional_value(form: &FormState, key: &str) -> Option<String> {
    let value = form_value(form, key);
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

pub(crate) fn bool_value(form: &FormState, key: &str) -> bool {
    matches!(form_value(form, key).as_str(), "true" | "1" | "yes" | "on")
}

pub(crate) fn split_csv(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn checkbox_enabled(value: &str) -> bool {
    matches!(value, "true" | "1" | "yes" | "on")
}

pub(crate) async fn create_quick_user(
    client: &crate::client::ZitadelClient,
    user: UserTemplate,
) -> anyhow::Result<Vec<String>> {
    let mut lines = Vec::new();
    let result = client
        .create_human_user(&user.email, &user.first_name, &user.last_name, None)
        .await?;
    lines.push(format!("Created {}", user.email));
    if user.admin {
        if let Some(user_id) = result.get("userId").and_then(|value| value.as_str()) {
            client.grant_iam_owner(user_id).await?;
            lines.push(format!("Granted IAM_OWNER to {}", user.email));
        }
    }
    Ok(lines)
}
