use ratatui::text::Line;

use super::{
    types::{App, ConfirmState, Focus, FormState, MessageState, PendingAction, ResourceKind},
    widgets::{bold_line, field_line, muted_line, status_heading, subtle_line},
};

pub(crate) fn footer_lines(app: &App) -> Vec<Line<'static>> {
    match app.active_resource() {
        ResourceKind::Applications => vec![
            status_heading("Applications"),
            muted_line("Canvas forms create, delete, rotate secrets, and batch-apply templates."),
        ],
        ResourceKind::Users => vec![
            status_heading("Users"),
            muted_line("User creation, admin bootstrap, IAM_OWNER grants, and quick setup all live in the canvas."),
        ],
        ResourceKind::Idps => vec![
            status_heading("Identity providers"),
            muted_line("Google IDP uses manual credentials only. Kubernetes secret loading stays removed."),
        ],
        ResourceKind::Auth => vec![
            status_heading("Authentication"),
            muted_line("Use setup to validate PAT or service-account auth and persist config safely."),
        ],
        ResourceKind::Config => vec![
            status_heading("Configuration"),
            muted_line("Runtime config is TOML only, while app and user templates stay in YAML."),
        ],
    }
}

pub(crate) fn browse_title(app: &App) -> &'static str {
    match app.active_resource() {
        ResourceKind::Applications => "OIDC application workspace",
        ResourceKind::Users => "User management workspace",
        ResourceKind::Idps => "Identity provider workspace",
        ResourceKind::Auth => "Authentication setup",
        ResourceKind::Config => "Configuration editor",
    }
}

pub(crate) fn browse_meta(app: &App) -> &'static str {
    match app.active_resource() {
        ResourceKind::Applications => "choose an action",
        ResourceKind::Users => "choose an action",
        ResourceKind::Idps => "choose an action",
        ResourceKind::Auth => "run setup",
        ResourceKind::Config => "edit or import",
    }
}

pub(crate) fn browse_lines(app: &App) -> Vec<String> {
    match app.active_resource() {
        ResourceKind::Applications => vec![
            "List is the default view for applications.".to_string(),
            "Use the action rail to create an application, rotate a confidential secret, delete an app, or batch-create predefined templates.".to_string(),
            "The tray below stays active while you work, and the inspector popup keeps detailed application metadata off-canvas.".to_string(),
        ],
        ResourceKind::Users => vec![
            "List is the default view for users.".to_string(),
            "Create normal users, bootstrap local admins with a temporary password, grant IAM_OWNER, or run predefined quick setup from templates.".to_string(),
            "Admin credentials are shown once in a success canvas after import succeeds.".to_string(),
        ],
        ResourceKind::Idps => vec![
            "Google is configured manually in Rust.".to_string(),
            "No Kubernetes-backed credential source appears anywhere in the TUI.".to_string(),
        ],
        ResourceKind::Auth => vec![
            "This area owns interactive setup and auth recovery.".to_string(),
            "Fill host, optional project, auth method, and template path, then validate before returning to the main shell.".to_string(),
            "OAuth device flow remains visible as a placeholder only.".to_string(),
        ],
        ResourceKind::Config => vec![
            "Edit saved host, project, and templates path here.".to_string(),
            "Runtime config is canonical TOML under the XDG config path.".to_string(),
        ],
    }
}

pub(crate) fn browse_review_lines(app: &App) -> Vec<Line<'static>> {
    if let Some(record) = app.selected_record() {
        vec![
            bold_line(&record.name),
            subtle_line(&record.kind),
            Line::from(""),
            field_line("Summary", &record.summary),
            field_line("Detail", &record.detail),
            field_line("Changed", &record.changed_at),
        ]
    } else {
        vec![
            bold_line("Ready"),
            subtle_line("Select an action to start a workflow."),
            Line::from(""),
            field_line("Mode", resource_label(app.active_resource())),
            field_line("Focus", focus_label(app.focus)),
            field_line("Inspector", "press i for popup"),
        ]
    }
}

pub(crate) fn form_review_lines(form: &FormState) -> Vec<Line<'static>> {
    vec![
        bold_line(&form.title),
        subtle_line(&form.description),
        Line::from(""),
        field_line("Submit", &form.submit_label),
        field_line("Fields", &form.fields.len().to_string()),
        field_line("Selected", &form.fields[form.selected_field].label),
        muted_line("Use j/k to move fields. Type to edit. Space toggles booleans and choices."),
    ]
}

pub(crate) fn confirm_review_lines(confirm: &ConfirmState) -> Vec<Line<'static>> {
    vec![
        bold_line(&confirm.title),
        subtle_line(&confirm.submit_label),
        Line::from(""),
        field_line("Pending", pending_label(&confirm.pending)),
        muted_line("Press Enter to confirm or Esc to cancel."),
    ]
}

pub(crate) fn message_review_lines(message: &MessageState) -> Vec<Line<'static>> {
    vec![
        bold_line(&message.title),
        subtle_line("workflow result"),
        Line::from(""),
        field_line("Lines", &message.lines.len().to_string()),
        muted_line("Press Enter or Esc to return to the workspace."),
    ]
}

pub fn pending_label(pending: &PendingAction) -> &'static str {
    match pending {
        PendingAction::CreateApplication => "create application",
        PendingAction::QuickSetupApplications => "quick setup apps",
        PendingAction::DeleteApplication { .. } => "delete application",
        PendingAction::RegenerateSecret { .. } => "regenerate secret",
        PendingAction::CreateUser => "create user",
        PendingAction::CreateAdminUser => "create admin user",
        PendingAction::GrantIamOwner { .. } => "grant IAM_OWNER",
        PendingAction::QuickSetupUsers => "quick setup users",
        PendingAction::ConfigureGoogleIdp => "configure Google IDP",
        PendingAction::ValidateAuthSetup => "validate auth setup",
        PendingAction::SaveConfig => "save config",
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn selection_title(app: &App) -> &'static str {
    match app.active_resource() {
        ResourceKind::Applications => "existing applications",
        ResourceKind::Users => "existing users",
        ResourceKind::Idps => "configured identity providers",
        ResourceKind::Auth => "setup state",
        ResourceKind::Config => "saved values",
    }
}

pub fn resource_label(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Applications => "Applications",
        ResourceKind::Users => "Users",
        ResourceKind::Idps => "IDPs",
        ResourceKind::Auth => "Auth",
        ResourceKind::Config => "Config",
    }
}

pub fn focus_label(focus: Focus) -> &'static str {
    match focus {
        Focus::Resources => "resources",
        Focus::Actions => "actions",
        Focus::Form => "form",
        Focus::Records => "records",
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn status_mark(app: &App) -> &'static str {
    if app.setup_required {
        "!"
    } else {
        "✓"
    }
}
