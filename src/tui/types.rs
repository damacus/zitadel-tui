#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Resources,
    Actions,
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
pub struct Resource {
    pub kind: ResourceKind,
    pub name: &'static str,
    pub count: String,
}

#[derive(Clone, Debug)]
pub struct Action {
    pub label: &'static str,
    pub hotkey: &'static str,
}

#[derive(Clone, Debug, Default)]
pub struct Record {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub summary: String,
    pub detail: String,
    pub changed_at: String,
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Secret,
    Toggle,
    Choice(Vec<String>),
    Checkbox,
}

#[derive(Debug)]
pub struct App {
    pub focus: Focus,
    pub show_inspector: bool,
    pub host: String,
    pub project: String,
    pub auth_label: String,
    pub templates_path: Option<String>,
    pub setup_required: bool,
    pub resources: Vec<Resource>,
    pub selected_resource: usize,
    pub selected_action: usize,
    pub app_records: Vec<Record>,
    pub user_records: Vec<Record>,
    pub idp_records: Vec<Record>,
    pub selected_record: usize,
    pub canvas_mode: CanvasMode,
}

pub const APPLICATION_ACTIONS: [Action; 4] = [
    Action {
        label: "Create application",
        hotkey: "[enter]",
    },
    Action {
        label: "Regenerate secret",
        hotkey: "[enter]",
    },
    Action {
        label: "Delete selected",
        hotkey: "[enter]",
    },
    Action {
        label: "Quick setup",
        hotkey: "[enter]",
    },
];

pub const USER_ACTIONS: [Action; 4] = [
    Action {
        label: "Create user",
        hotkey: "[enter]",
    },
    Action {
        label: "Create admin",
        hotkey: "[enter]",
    },
    Action {
        label: "Grant IAM_OWNER",
        hotkey: "[enter]",
    },
    Action {
        label: "Quick setup",
        hotkey: "[enter]",
    },
];

pub const IDP_ACTIONS: [Action; 1] = [Action {
    label: "Configure Google",
    hotkey: "[enter]",
}];

pub const AUTH_ACTIONS: [Action; 1] = [Action {
    label: "Run setup",
    hotkey: "[enter]",
}];

pub const CONFIG_ACTIONS: [Action; 1] = [Action {
    label: "Edit config",
    hotkey: "[enter]",
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
            },
            FormField {
                key: "project",
                label: "Project".to_string(),
                value: bootstrap.project.clone(),
                kind: FieldKind::Text,
                help: "Optional default project ID".to_string(),
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
            },
            FormField {
                key: "token",
                label: "PAT".to_string(),
                value: String::new(),
                kind: FieldKind::Secret,
                help: "Used when auth method is PAT".to_string(),
            },
            FormField {
                key: "service_account_file",
                label: "Service account".to_string(),
                value: String::new(),
                kind: FieldKind::Text,
                help: "Used when auth method is service account".to_string(),
            },
            FormField {
                key: "templates_path",
                label: "Templates path".to_string(),
                value: bootstrap.templates_path.clone().unwrap_or_default(),
                kind: FieldKind::Text,
                help: "Optional apps/users YAML file".to_string(),
            },
        ],
    }
}
