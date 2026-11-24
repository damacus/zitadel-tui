#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Resources,
    Form,
    Records,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    Applications,
    Users,
    Idps,
    Auth,
    Config,
}

#[derive(Clone, Debug)]
pub struct Action {
    pub label: &'static str,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Record {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub summary: String,
    pub detail: String,
    pub changed_at: String,
    pub redirect_uris: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct TuiBootstrap {
    pub host: String,
    pub project: String,
    pub auth_label: String,
    pub templates_path: Option<String>,
    pub setup_required: bool,
    pub app_records: Vec<Record>,
    pub user_records: Vec<Record>,
    pub idp_records: Vec<Record>,
    pub idp_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanvasMode {
    Browse,
    EditForm(FormState),
    Confirm(ConfirmState),
    Success(MessageState),
    Error(MessageState),
    Setup(FormState),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PendingAction {
    CreateApplication,
    AddRedirectUris {
        app_id: String,
        name: String,
    },
    QuickSetupApplications,
    DeleteApplication {
        app_id: String,
        name: String,
    },
    RegenerateSecret {
        app_id: String,
        name: String,
        client_id: String,
    },
    CreateUser,
    CreateAdminUser,
    GrantIamOwner {
        user_id: String,
        username: String,
    },
    QuickSetupUsers,
    ConfigureGoogleIdp,
    ValidateAuthSetup,
    SaveConfig,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmState {
    pub title: String,
    pub lines: Vec<String>,
    pub submit_label: String,
    pub pending: PendingAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageState {
    pub title: String,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormState {
    pub title: String,
    pub description: String,
    pub submit_label: String,
    pub fields: Vec<FormField>,
    pub selected_field: usize,
    pub pending: PendingAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormField {
    pub key: &'static str,
    pub label: String,
    pub value: String,
    pub kind: FieldKind,
    pub help: String,
    pub cursor: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Secret,
    Toggle,
    Choice(Vec<String>),
    Checkbox,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppCommand {
    Noop,
    Quit,
    BeginAction {
        resource: ResourceKind,
        action_index: usize,
        selected_record: Option<Record>,
    },
    SubmitForm(FormState),
    Confirm(PendingAction),
}

#[derive(Debug)]
pub struct App {
    pub cursor: usize,
    pub expanded: Option<usize>,
    pub filter: String,
    pub filtering: bool,
    pub scroll: usize,
    pub focus: Focus,
    pub host: String,
    pub project: String,
    pub auth_label: String,
    pub templates_path: Option<String>,
    pub setup_required: bool,
    pub selected_resource: usize,
    pub app_records: Vec<Record>,
    pub user_records: Vec<Record>,
    pub idp_records: Vec<Record>,
    pub idp_error: Option<String>,
    pub canvas_mode: CanvasMode,
}

pub const APPLICATION_ACTIONS: [Action; 5] = [
    Action {
        label: "Create application",
    },
    Action {
        label: "Regenerate secret",
    },
    Action {
        label: "Delete selected",
    },
    Action {
        label: "Quick setup",
    },
    Action {
        label: "Add redirect URIs",
    },
];

pub const USER_ACTIONS: [Action; 4] = [
    Action {
        label: "Create user",
    },
    Action {
        label: "Create admin",
    },
    Action {
        label: "Grant IAM_OWNER",
    },
    Action {
        label: "Quick setup",
    },
];

pub const IDP_ACTIONS: [Action; 1] = [Action {
    label: "Configure Google",
}];

pub const AUTH_ACTIONS: [Action; 1] = [Action { label: "Run setup" }];

pub const CONFIG_ACTIONS: [Action; 1] = [Action {
    label: "Edit config",
}];

pub fn default_setup_form(bootstrap: &TuiBootstrap) -> FormState {
    FormState {
        title: "Initial setup".to_string(),
        description:
            "Configure Zitadel connectivity and validate credentials before entering the workspace."
                .to_string(),
        submit_label: "[Enter] validate and save".to_string(),
        selected_field: 0,
        pending: PendingAction::ValidateAuthSetup,
        fields: vec![
            FormField {
                key: "host",
                label: "Host".to_string(),
                value: bootstrap.host.clone(),
                kind: FieldKind::Text,
                help: "Zitadel base URL".to_string(),
                cursor: bootstrap.host.len(),
            },
            FormField {
                key: "project",
                label: "Project".to_string(),
                value: bootstrap.project.clone(),
                kind: FieldKind::Text,
                help: "Optional default project ID".to_string(),
                cursor: bootstrap.project.len(),
            },
            FormField {
                key: "auth_method",
                label: "Auth method".to_string(),
                value: "PAT".to_string(),
                kind: FieldKind::Choice(vec![
                    "PAT".to_string(),
                    "Service account".to_string(),
                    "OAuth device (placeholder)".to_string(),
                ]),
                help: "Choose PAT or service account for this slice".to_string(),
                cursor: 3,
            },
            FormField {
                key: "token",
                label: "PAT".to_string(),
                value: String::new(),
                kind: FieldKind::Secret,
                help: "Used when auth method is PAT".to_string(),
                cursor: 0,
            },
            FormField {
                key: "service_account_file",
                label: "Service account".to_string(),
                value: String::new(),
                kind: FieldKind::Text,
                help: "Used when auth method is service account".to_string(),
                cursor: 0,
            },
            FormField {
                key: "templates_path",
                label: "Templates path".to_string(),
                value: bootstrap.templates_path.clone().unwrap_or_default(),
                kind: FieldKind::Text,
                help: "Optional apps/users YAML file".to_string(),
                cursor: bootstrap.templates_path.clone().unwrap_or_default().len(),
            },
        ],
    }
}
