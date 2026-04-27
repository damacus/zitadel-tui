use crossterm::event::KeyCode;

use super::types::{
    default_setup_form, App, AppCommand, CanvasMode, Focus, FormField, FormState, Resource,
    ResourceKind, TuiBootstrap, APPLICATION_ACTIONS, AUTH_ACTIONS, CONFIG_ACTIONS, IDP_ACTIONS,
    USER_ACTIONS,
};

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

    pub fn actions(&self) -> &'static [super::types::Action] {
        match self.active_resource() {
            ResourceKind::Applications => &APPLICATION_ACTIONS,
            ResourceKind::Users => &USER_ACTIONS,
            ResourceKind::Idps => &IDP_ACTIONS,
            ResourceKind::Auth => &AUTH_ACTIONS,
            ResourceKind::Config => &CONFIG_ACTIONS,
        }
    }

    pub fn active_records(&self) -> &[super::types::Record] {
        match self.active_resource() {
            ResourceKind::Applications => &self.app_records,
            ResourceKind::Users => &self.user_records,
            ResourceKind::Idps => &self.idp_records,
            ResourceKind::Auth | ResourceKind::Config => &[],
        }
    }

    pub fn selected_record(&self) -> Option<&super::types::Record> {
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

    pub fn handle_key(&mut self, key: KeyCode) -> AppCommand {
        match key {
            KeyCode::Char('q') => AppCommand::Quit,
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_forward();
                AppCommand::Noop
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_backward();
                AppCommand::Noop
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.move_left();
                AppCommand::Noop
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.move_right();
                AppCommand::Noop
            }
            KeyCode::Char('i') => {
                self.toggle_inspector();
                AppCommand::Noop
            }
            KeyCode::Char('n') => {
                self.set_focus(Focus::Actions);
                AppCommand::Noop
            }
            KeyCode::Char('g') => {
                self.set_focus(Focus::Resources);
                AppCommand::Noop
            }
            KeyCode::Enter => self.handle_enter(),
            KeyCode::Esc => {
                self.handle_escape();
                AppCommand::Noop
            }
            KeyCode::Backspace => {
                if self.is_form_editing() {
                    self.form_backspace();
                }
                AppCommand::Noop
            }
            KeyCode::Char(' ') => {
                if self.is_form_editing() {
                    self.form_toggle_or_cycle(true);
                }
                AppCommand::Noop
            }
            KeyCode::Char(ch) => {
                if self.is_form_editing() {
                    self.form_insert_char(ch);
                }
                AppCommand::Noop
            }
            KeyCode::Tab => {
                self.advance_focus();
                AppCommand::Noop
            }
            KeyCode::BackTab => {
                self.reverse_focus();
                AppCommand::Noop
            }
            _ => AppCommand::Noop,
        }
    }

    pub fn advance_focus(&mut self) {
        self.cycle_focus(true);
    }

    pub fn reverse_focus(&mut self) {
        self.cycle_focus(false);
    }

    pub fn set_canvas_mode(&mut self, canvas_mode: CanvasMode) {
        self.canvas_mode = canvas_mode;
        self.focus = self.focus_order()[0];
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
                super::types::FieldKind::Text | super::types::FieldKind::Secret => {
                    field.value.push(ch)
                }
                super::types::FieldKind::Toggle | super::types::FieldKind::Checkbox => {
                    if ch == ' ' {
                        toggle_field(field);
                    }
                }
                super::types::FieldKind::Choice(options) => {
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
                super::types::FieldKind::Text | super::types::FieldKind::Secret => {
                    field.value.pop();
                }
                super::types::FieldKind::Toggle
                | super::types::FieldKind::Checkbox
                | super::types::FieldKind::Choice(_) => {}
            }
        }
    }

    pub fn form_toggle_or_cycle(&mut self, forward: bool) {
        if let Some(field) = self.active_form_field_mut() {
            let kind = field.kind.clone();
            match kind {
                super::types::FieldKind::Toggle | super::types::FieldKind::Checkbox => {
                    toggle_field(field)
                }
                super::types::FieldKind::Choice(options) => cycle_choice(field, &options, forward),
                super::types::FieldKind::Text | super::types::FieldKind::Secret => {}
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

    fn move_forward(&mut self) {
        match self.focus {
            Focus::Resources => self.next_resource(),
            Focus::Actions => self.next_action(),
            Focus::Form => self.form_next_field(),
            Focus::Records => self.next_record(),
        }
    }

    fn move_backward(&mut self) {
        match self.focus {
            Focus::Resources => self.previous_resource(),
            Focus::Actions => self.previous_action(),
            Focus::Form => self.form_previous_field(),
            Focus::Records => self.previous_record(),
        }
    }

    fn move_left(&mut self) {
        if self.is_form_editing() {
            self.form_toggle_or_cycle(false);
            return;
        }

        if self.focus != Focus::Form {
            self.previous_resource();
        }
    }

    fn move_right(&mut self) {
        if self.is_form_editing() {
            self.form_toggle_or_cycle(true);
            return;
        }

        if self.focus != Focus::Form {
            self.next_resource();
        }
    }

    fn handle_enter(&mut self) -> AppCommand {
        match &self.canvas_mode {
            CanvasMode::Browse => AppCommand::BeginAction {
                resource: self.active_resource(),
                action_index: self.selected_action,
                selected_record: self.selected_record().cloned(),
            },
            CanvasMode::EditForm(form) | CanvasMode::Setup(form) => {
                AppCommand::SubmitForm(form.clone())
            }
            CanvasMode::Confirm(confirm) => AppCommand::Confirm(confirm.pending.clone()),
            CanvasMode::Success(_) | CanvasMode::Error(_) => {
                self.reset_to_browse();
                AppCommand::Noop
            }
        }
    }

    fn handle_escape(&mut self) {
        if !matches!(self.canvas_mode, CanvasMode::Browse) {
            self.reset_to_browse();
        }
    }

    fn is_form_editing(&self) -> bool {
        self.focus == Focus::Form
            && matches!(
                self.canvas_mode,
                CanvasMode::EditForm(_) | CanvasMode::Setup(_)
            )
    }

    fn cycle_focus(&mut self, forward: bool) {
        let ring = self.focus_order();
        let current_index = ring
            .iter()
            .position(|focus| *focus == self.focus)
            .unwrap_or(0);
        let next_index = if forward {
            (current_index + 1) % ring.len()
        } else if current_index == 0 {
            ring.len() - 1
        } else {
            current_index - 1
        };
        self.focus = ring[next_index];
    }

    fn set_focus(&mut self, focus: Focus) {
        if self.focus_order().contains(&focus) {
            self.focus = focus;
        }
    }

    fn focus_order(&self) -> Vec<Focus> {
        let mut order = if matches!(self.canvas_mode, CanvasMode::Browse) {
            vec![Focus::Resources, Focus::Actions]
        } else {
            vec![Focus::Form, Focus::Resources, Focus::Actions]
        };

        if !self.active_records().is_empty() {
            order.push(Focus::Records);
        }

        order
    }
}

pub(crate) fn toggle_field(field: &mut FormField) {
    field.value = if is_enabled(&field.value) {
        "false".to_string()
    } else {
        "true".to_string()
    };
}

pub(crate) fn cycle_choice(field: &mut FormField, options: &[String], forward: bool) {
    let Some(current) = options.iter().position(|value| value == &field.value) else {
        if let Some(first) = options.first() {
            field.value = first.clone();
        }
        return;
    };

    let next = if forward {
        (current + 1) % options.len()
    } else if current == 0 {
        options.len() - 1
    } else {
        current - 1
    };
    field.value = options[next].clone();
}

pub(crate) fn is_enabled(value: &str) -> bool {
    matches!(value, "true" | "yes" | "on" | "1")
}
