use crossterm::event::KeyCode;

use super::types::{
    default_setup_form, App, AppCommand, CanvasMode, Focus, FormField, FormState, ResourceKind,
    TuiBootstrap, APPLICATION_ACTIONS, AUTH_ACTIONS, CONFIG_ACTIONS, IDP_ACTIONS, USER_ACTIONS,
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
            initial_values: Self::field_values(&canvas_mode),
            overlay: None,
            cursor: 0,
            expanded: None,
            filter: String::new(),
            filtering: false,
            scroll: 0,
            focus: if setup_required {
                Focus::Form
            } else {
                Focus::Resources
            },
            organization: bootstrap.organization,
            organizations: bootstrap.organizations,
            projects: bootstrap.projects,
            org_error: bootstrap.org_error,
            project_error: bootstrap.project_error,
            app_error: bootstrap.app_error,
            user_error: bootstrap.user_error,
            host: bootstrap.host,
            project: bootstrap.project,
            auth_label: bootstrap.auth_label,
            templates_path: templates_path.clone(),
            setup_required,
            selected_resource: 0,
            app_records,
            user_records,
            idp_records,
            idp_error: bootstrap.idp_error,
            canvas_mode,
        }
    }

    pub fn sync_runtime(&mut self, bootstrap: TuiBootstrap) {
        self.organization = bootstrap.organization;
        self.organizations = bootstrap.organizations;
        self.projects = bootstrap.projects;
        self.org_error = bootstrap.org_error;
        self.project_error = bootstrap.project_error;
        self.app_error = bootstrap.app_error;
        self.user_error = bootstrap.user_error;
        self.host = bootstrap.host;
        self.project = bootstrap.project;
        self.auth_label = bootstrap.auth_label;
        self.templates_path = bootstrap.templates_path;
        self.setup_required = bootstrap.setup_required;
        let target = self
            .expanded
            .and_then(|i| self.active_records().get(i))
            .map(|r| r.id.clone());
        let cursor_id = match self.entries().get(self.cursor) {
            Some(super::directory::Entry::Record(i)) => Some(self.active_records()[*i].id.clone()),
            _ => None,
        };
        let old_cursor = self.cursor;
        self.app_records = bootstrap.app_records;
        self.user_records = bootstrap.user_records;
        self.idp_records = bootstrap.idp_records;
        self.idp_error = bootstrap.idp_error;
        self.expanded = target
            .as_ref()
            .and_then(|id| self.active_records().iter().position(|r| &r.id == id));
        let entries = self.entries();
        self.cursor=cursor_id.and_then(|id|entries.iter().position(|entry| matches!(entry,super::directory::Entry::Record(i) if self.active_records()[*i].id==id))).unwrap_or_else(||old_cursor.min(entries.len().saturating_sub(1)));
    }

    pub fn active_resource(&self) -> ResourceKind {
        [
            ResourceKind::Applications,
            ResourceKind::Users,
            ResourceKind::Idps,
            ResourceKind::Auth,
        ][self.selected_resource]
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

    pub fn handle_key(&mut self, key: KeyCode) -> AppCommand {
        if self.overlay.is_some() {
            return self.overlay_key(key);
        }
        if !self.setup_required {
            match key {
                KeyCode::F(n @ 1..=4) => {
                    return self.navigate(super::Destination::Section(usize::from(n - 1)))
                }
                KeyCode::F(5) if matches!(self.canvas_mode, CanvasMode::Browse) => {
                    return AppCommand::Refresh
                }
                KeyCode::F(6) => return self.open_picker(super::Scope::Organisation),
                KeyCode::F(7) => return self.open_picker(super::Scope::Project),
                _ => {}
            }
        }
        if self.is_form_editing() {
            match key {
                KeyCode::Esc => self.navigate(super::Destination::Back),
                KeyCode::Enter => self.handle_enter(),
                KeyCode::Backspace => {
                    self.form_backspace();
                    AppCommand::Noop
                }
                KeyCode::Char(' ') => {
                    self.form_insert_char(' ');
                    AppCommand::Noop
                }
                KeyCode::Char(ch) => {
                    self.form_insert_char(ch);
                    AppCommand::Noop
                }
                KeyCode::Tab => {
                    self.form_next_field();
                    AppCommand::Noop
                }
                KeyCode::BackTab => {
                    self.form_previous_field();
                    AppCommand::Noop
                }
                KeyCode::Left => {
                    self.form_cursor_left();
                    AppCommand::Noop
                }
                KeyCode::Right => {
                    self.form_cursor_right();
                    AppCommand::Noop
                }
                KeyCode::Up => {
                    self.form_previous_field();
                    AppCommand::Noop
                }
                KeyCode::Down => {
                    self.form_next_field();
                    AppCommand::Noop
                }
                _ => AppCommand::Noop,
            }
        } else if matches!(self.canvas_mode, CanvasMode::Browse) {
            self.directory_key(key)
        } else {
            match key {
                KeyCode::Enter => self.handle_enter(),
                KeyCode::Esc => {
                    self.handle_escape();
                    AppCommand::Noop
                }
                KeyCode::Down => {
                    self.scroll = self.scroll.saturating_add(1);
                    AppCommand::Noop
                }
                KeyCode::Up => {
                    self.scroll = self.scroll.saturating_sub(1);
                    AppCommand::Noop
                }
                KeyCode::Char('q') => AppCommand::Quit,
                _ => AppCommand::Noop,
            }
        }
    }

    pub fn finish_submission(&mut self, next: CanvasMode) {
        match next {
            CanvasMode::Error(error) => {
                self.overlay = Some(super::Overlay::SubmissionError(error, 0))
            }
            mode => self.set_canvas_mode(mode),
        }
    }

    pub fn set_canvas_mode(&mut self, canvas_mode: CanvasMode) {
        self.scroll = 0;
        self.initial_values = Self::field_values(&canvas_mode);
        self.canvas_mode = canvas_mode;
        self.focus = if matches!(self.canvas_mode, CanvasMode::Browse) {
            Focus::Resources
        } else {
            Focus::Form
        };
    }

    pub fn reset_to_browse(&mut self) {
        self.canvas_mode = if self.setup_required {
            CanvasMode::Setup(default_setup_form(&TuiBootstrap {
                organization: self.organization.clone(),
                organizations: self.organizations.clone(),
                projects: self.projects.clone(),
                org_error: self.org_error.clone(),
                project_error: self.project_error.clone(),
                app_error: self.app_error.clone(),
                user_error: self.user_error.clone(),
                host: self.host.clone(),
                project: self.project.clone(),
                auth_label: self.auth_label.clone(),
                templates_path: self.templates_path.clone(),
                setup_required: self.setup_required,
                app_records: self.app_records.clone(),
                user_records: self.user_records.clone(),
                idp_records: self.idp_records.clone(),
                idp_error: self.idp_error.clone(),
            }))
        } else {
            CanvasMode::Browse
        };
        self.focus = if self.setup_required {
            Focus::Form
        } else {
            Focus::Records
        };
    }

    pub fn form_next_field(&mut self) {
        if let Some(form) = self.form_state_mut() {
            if form.fields.is_empty() {
                return;
            }
            form.selected_field = (form.selected_field + 1) % form.fields.len();
        }
    }

    pub fn form_previous_field(&mut self) {
        if let Some(form) = self.form_state_mut() {
            if form.fields.is_empty() {
                return;
            }
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
                    let cursor = field.cursor.min(field.value.len());
                    field.value.insert(cursor, ch);
                    field.cursor = cursor + ch.len_utf8();
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
                    let cursor = field.cursor.min(field.value.len());
                    if cursor > 0 {
                        let prev = field.value[..cursor]
                            .char_indices()
                            .next_back()
                            .map_or(0, |(i, _)| i);
                        field.value.remove(prev);
                        field.cursor = prev;
                    }
                }
                super::types::FieldKind::Toggle
                | super::types::FieldKind::Checkbox
                | super::types::FieldKind::Choice(_) => {}
            }
        }
    }

    pub fn form_cursor_left(&mut self) {
        if let Some(field) = self.active_form_field_mut() {
            match field.kind {
                super::types::FieldKind::Text | super::types::FieldKind::Secret => {
                    field.cursor = field.value[..field.cursor.min(field.value.len())]
                        .char_indices()
                        .next_back()
                        .map_or(0, |(i, _)| i);
                }
                _ => {}
            }
        }
    }

    pub fn form_cursor_right(&mut self) {
        if let Some(field) = self.active_form_field_mut() {
            match field.kind {
                super::types::FieldKind::Text | super::types::FieldKind::Secret => {
                    let len = field.value.len();
                    field.cursor += field.value[field.cursor.min(len)..]
                        .chars()
                        .next()
                        .map_or(0, char::len_utf8);
                }
                _ => {}
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

    fn handle_enter(&mut self) -> AppCommand {
        match &self.canvas_mode {
            CanvasMode::Browse => AppCommand::Noop,
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
