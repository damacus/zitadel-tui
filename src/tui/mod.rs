use ratatui::text::Line;

mod render;
#[cfg(test)]
mod tests;

pub use render::draw;
use render::{
    browse_lines, browse_meta, browse_review_lines, browse_title, confirm_review_lines,
    cycle_choice, default_setup_form, form_review_lines, message_review_lines, render_form_line,
    toggle_field,
};

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

impl App {
    pub fn from_bootstrap(bootstrap: TuiBootstrap) -> Self {
        let setup_required = bootstrap.setup_required;
        let templates_path = bootstrap.templates_path.clone();
        let canvas_bootstrap = bootstrap.clone();
        let app_records = bootstrap.app_records;
        let user_records = bootstrap.user_records;
        let idp_records = bootstrap.idp_records;

        let canvas_mode = if setup_required {
            CanvasMode::Setup(default_setup_form(&canvas_bootstrap))
        } else {
            CanvasMode::Browse
        };

        Self {
            focus: Focus::Resources,
            show_inspector: false,
            host: bootstrap.host,
            project: bootstrap.project,
            auth_label: bootstrap.auth_label,
            templates_path: templates_path.clone(),
            setup_required,
            resources: vec![
                Resource {
                    kind: ResourceKind::Applications,
                    name: "Applications",
                    count: app_records.len().to_string(),
                },
                Resource {
                    kind: ResourceKind::Users,
                    name: "Users",
                    count: user_records.len().to_string(),
                },
                Resource {
                    kind: ResourceKind::Idps,
                    name: "IDPs",
                    count: idp_records.len().to_string(),
                },
                Resource {
                    kind: ResourceKind::Auth,
                    name: "Auth",
                    count: if setup_required {
                        "setup".to_string()
                    } else {
                        "ready".to_string()
                    },
                },
                Resource {
                    kind: ResourceKind::Config,
                    name: "Config",
                    count: if templates_path.is_some() {
                        "templated".to_string()
                    } else {
                        "local".to_string()
                    },
                },
            ],
            selected_resource: 0,
            selected_action: 0,
            app_records,
            user_records,
            idp_records,
            selected_record: 0,
            canvas_mode,
        }
    }

    pub fn sync_runtime(&mut self, bootstrap: TuiBootstrap) {
        self.host = bootstrap.host;
        self.project = bootstrap.project;
        self.auth_label = bootstrap.auth_label;
        self.templates_path = bootstrap.templates_path;
        self.setup_required = bootstrap.setup_required;
        self.app_records = bootstrap.app_records;
        self.user_records = bootstrap.user_records;
        self.idp_records = bootstrap.idp_records;
        self.selected_record = 0;
        self.selected_action = 0;
        self.resources[0].count = self.app_records.len().to_string();
        self.resources[1].count = self.user_records.len().to_string();
        self.resources[2].count = self.idp_records.len().to_string();
        self.resources[3].count = if self.setup_required {
            "setup".to_string()
        } else {
            "ready".to_string()
        };
        self.resources[4].count = if self.templates_path.is_some() {
            "templated".to_string()
        } else {
            "local".to_string()
        };
    }

    pub fn active_resource(&self) -> ResourceKind {
        self.resources[self.selected_resource].kind
    }

    pub fn actions(&self) -> &'static [Action] {
        match self.active_resource() {
            ResourceKind::Applications => &APPLICATION_ACTIONS,
            ResourceKind::Users => &USER_ACTIONS,
            ResourceKind::Idps => &IDP_ACTIONS,
            ResourceKind::Auth => &AUTH_ACTIONS,
            ResourceKind::Config => &CONFIG_ACTIONS,
        }
    }

    pub fn active_records(&self) -> &[Record] {
        match self.active_resource() {
            ResourceKind::Applications => &self.app_records,
            ResourceKind::Users => &self.user_records,
            ResourceKind::Idps => &self.idp_records,
            ResourceKind::Auth | ResourceKind::Config => &[],
        }
    }

    pub fn selected_record(&self) -> Option<&Record> {
        self.active_records().get(self.selected_record)
    }

    pub fn next_record(&mut self) {
        let len = self.active_records().len();
        if len == 0 {
            self.focus = Focus::Records;
            return;
        }

        self.selected_record = (self.selected_record + 1) % len;
        self.focus = Focus::Records;
    }

    pub fn previous_record(&mut self) {
        let len = self.active_records().len();
        if len == 0 {
            self.focus = Focus::Records;
            return;
        }

        self.selected_record = if self.selected_record == 0 {
            len - 1
        } else {
            self.selected_record - 1
        };
        self.focus = Focus::Records;
    }

    pub fn next_resource(&mut self) {
        self.selected_resource = (self.selected_resource + 1) % self.resources.len();
        self.selected_action = 0;
        self.selected_record = 0;
        self.focus = Focus::Resources;
    }

    pub fn previous_resource(&mut self) {
        self.selected_resource = if self.selected_resource == 0 {
            self.resources.len() - 1
        } else {
            self.selected_resource - 1
        };
        self.selected_action = 0;
        self.selected_record = 0;
        self.focus = Focus::Resources;
    }

    pub fn next_action(&mut self) {
        let len = self.actions().len();
        if len == 0 {
            self.focus = Focus::Actions;
            return;
        }

        self.selected_action = (self.selected_action + 1) % len;
        self.focus = Focus::Actions;
    }

    pub fn previous_action(&mut self) {
        let len = self.actions().len();
        if len == 0 {
            self.focus = Focus::Actions;
            return;
        }

        self.selected_action = if self.selected_action == 0 {
            len - 1
        } else {
            self.selected_action - 1
        };
        self.focus = Focus::Actions;
    }

    pub fn toggle_inspector(&mut self) {
        self.show_inspector = !self.show_inspector;
    }

    pub fn advance_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Resources => Focus::Actions,
            Focus::Actions => Focus::Form,
            Focus::Form => Focus::Records,
            Focus::Records => Focus::Resources,
        };
    }

    pub fn reverse_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Resources => Focus::Records,
            Focus::Actions => Focus::Resources,
            Focus::Form => Focus::Actions,
            Focus::Records => Focus::Form,
        };
    }

    pub fn set_canvas_mode(&mut self, canvas_mode: CanvasMode) {
        self.canvas_mode = canvas_mode;
        self.focus = match self.canvas_mode {
            CanvasMode::Browse => Focus::Resources,
            CanvasMode::EditForm(_) | CanvasMode::Setup(_) => Focus::Form,
            CanvasMode::Confirm(_) | CanvasMode::Success(_) | CanvasMode::Error(_) => Focus::Form,
        };
    }

    pub fn reset_to_browse(&mut self) {
        self.canvas_mode = if self.setup_required {
            CanvasMode::Setup(default_setup_form(&TuiBootstrap {
                host: self.host.clone(),
                project: self.project.clone(),
                auth_label: self.auth_label.clone(),
                templates_path: self.templates_path.clone(),
                setup_required: self.setup_required,
                app_records: self.app_records.clone(),
                user_records: self.user_records.clone(),
                idp_records: self.idp_records.clone(),
            }))
        } else {
            CanvasMode::Browse
        };
        self.focus = Focus::Resources;
    }

    pub fn form_next_field(&mut self) {
        if let Some(form) = self.form_state_mut() {
            form.selected_field = (form.selected_field + 1) % form.fields.len();
        }
    }

    pub fn form_previous_field(&mut self) {
        if let Some(form) = self.form_state_mut() {
            form.selected_field = if form.selected_field == 0 {
                form.fields.len() - 1
            } else {
                form.selected_field - 1
            };
        }
    }

    pub fn form_insert_char(&mut self, ch: char) {
        if let Some(field) = self.active_form_field_mut() {
            let kind = field.kind.clone();
            match kind {
                FieldKind::Text | FieldKind::Secret => field.value.push(ch),
                FieldKind::Toggle | FieldKind::Checkbox => {
                    if ch == ' ' {
                        toggle_field(field);
                    }
                }
                FieldKind::Choice(options) => {
                    if ch == ' ' {
                        cycle_choice(field, &options, true);
                    }
                }
            }
        }
    }

    pub fn form_backspace(&mut self) {
        if let Some(field) = self.active_form_field_mut() {
            match field.kind {
                FieldKind::Text | FieldKind::Secret => {
                    field.value.pop();
                }
                FieldKind::Toggle | FieldKind::Checkbox | FieldKind::Choice(_) => {}
            }
        }
    }

    pub fn form_toggle_or_cycle(&mut self, forward: bool) {
        if let Some(field) = self.active_form_field_mut() {
            let kind = field.kind.clone();
            match kind {
                FieldKind::Toggle | FieldKind::Checkbox => toggle_field(field),
                FieldKind::Choice(options) => cycle_choice(field, &options, forward),
                FieldKind::Text | FieldKind::Secret => {}
            }
        }
    }

    pub fn message_lines(&self) -> Vec<String> {
        match &self.canvas_mode {
            CanvasMode::Browse => browse_lines(self),
            CanvasMode::EditForm(form) | CanvasMode::Setup(form) => form
                .fields
                .iter()
                .enumerate()
                .map(|(index, field)| render_form_line(field, index == form.selected_field))
                .collect(),
            CanvasMode::Confirm(confirm) => confirm.lines.clone(),
            CanvasMode::Success(message) | CanvasMode::Error(message) => message.lines.clone(),
        }
    }

    pub fn canvas_title(&self) -> String {
        match &self.canvas_mode {
            CanvasMode::Browse => browse_title(self).to_string(),
            CanvasMode::EditForm(form) | CanvasMode::Setup(form) => form.title.clone(),
            CanvasMode::Confirm(confirm) => confirm.title.clone(),
            CanvasMode::Success(message) | CanvasMode::Error(message) => message.title.clone(),
        }
    }

    pub fn canvas_meta(&self) -> String {
        match &self.canvas_mode {
            CanvasMode::Browse => browse_meta(self).to_string(),
            CanvasMode::EditForm(form) | CanvasMode::Setup(form) => form.submit_label.clone(),
            CanvasMode::Confirm(confirm) => confirm.submit_label.clone(),
            CanvasMode::Success(_) => "[Enter] continue".to_string(),
            CanvasMode::Error(_) => "[Esc] back".to_string(),
        }
    }

    pub fn review_lines(&self) -> Vec<Line<'static>> {
        match &self.canvas_mode {
            CanvasMode::Browse => browse_review_lines(self),
            CanvasMode::EditForm(form) | CanvasMode::Setup(form) => form_review_lines(form),
            CanvasMode::Confirm(confirm) => confirm_review_lines(confirm),
            CanvasMode::Success(message) | CanvasMode::Error(message) => {
                message_review_lines(message)
            }
        }
    }

    fn form_state_mut(&mut self) -> Option<&mut FormState> {
        match &mut self.canvas_mode {
            CanvasMode::EditForm(form) | CanvasMode::Setup(form) => Some(form),
            CanvasMode::Browse
            | CanvasMode::Confirm(_)
            | CanvasMode::Success(_)
            | CanvasMode::Error(_) => None,
        }
    }

    fn active_form_field_mut(&mut self) -> Option<&mut FormField> {
        self.form_state_mut()
            .and_then(|form| form.fields.get_mut(form.selected_field))
    }
}

const APPLICATION_ACTIONS: [Action; 4] = [
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

const USER_ACTIONS: [Action; 4] = [
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

const IDP_ACTIONS: [Action; 1] = [Action {
    label: "Configure Google",
    hotkey: "[enter]",
}];

const AUTH_ACTIONS: [Action; 1] = [Action {
    label: "Run setup",
    hotkey: "[enter]",
}];

const CONFIG_ACTIONS: [Action; 1] = [Action {
    label: "Edit config",
    hotkey: "[enter]",
}];
