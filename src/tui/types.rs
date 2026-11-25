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
    pub provider_owner: Option<&'static str>,
    pub changed_at: String,
    pub redirect_uris: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct TuiBootstrap {
    pub organization: Option<String>,
    pub organizations: Vec<(String, String)>,
    pub projects: Vec<(String, String)>,
    pub org_error: Option<String>,
    pub project_error: Option<String>,
    pub app_error: Option<String>,
    pub user_error: Option<String>,
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
    EditRedirectUris {
        original: Vec<String>,
        app_id: String,
        name: String,
    },
    ApplyRedirectUris {
        app_id: String,
        original: Vec<String>,
        desired: Vec<String>,
    },
    RenameApplication {
        app_id: String,
    },
    CreateNative,
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
    UserRole {
        user: Box<Record>,
        project: String,
        grant: Option<crate::api::user_access::Grant>,
        add: bool,
        role: Option<String>,
    },
    UserLifecycle {
        user: Box<Record>,
        unlock: bool,
    },
    QuickSetupUsers,
    ConfigureGoogleIdp,
    EditGoogleIdp {
        id: String,
        scope: String,
        sequence: String,
    },
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
    Refresh,
    ChangeScope(super::Scope, String),
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
    pub overlay: Option<super::Overlay>,
    pub initial_values: Vec<String>,
    pub cursor: usize,
    pub expanded: Option<usize>,
    pub filter: String,
    pub filtering: bool,
    pub scroll: usize,
    pub focus: Focus,
    pub organization: Option<String>,
    pub organizations: Vec<(String, String)>,
    pub projects: Vec<(String, String)>,
    pub org_error: Option<String>,
    pub project_error: Option<String>,
    pub app_error: Option<String>,
    pub user_error: Option<String>,
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

pub const APPLICATION_ACTIONS: [Action; 7] = [
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
        label: "Edit redirect URIs",
    },
    Action {
        label: "Create native application",
    },
    Action {
        label: "Rename application",
    },
];

pub const USER_ACTIONS: [Action; 9] = [
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
    Action {
        label: "View project roles",
    },
    Action {
        label: "Grant project role",
    },
    Action {
        label: "Revoke project role",
    },
    Action {
        label: "Unlock user",
    },
    Action {
        label: "Deactivate user",
    },
];

pub const IDP_ACTIONS: [Action; 2] = [
    Action {
        label: "Add Google provider",
    },
    Action {
        label: "Edit Google provider",
    },
];

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
                kind: FieldKind::Choice(vec!["PAT".to_string(), "Service account".to_string()]),
                help: "Use a personal access token or service-account key.".to_string(),
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
