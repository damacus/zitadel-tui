use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
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

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let outer = Block::default().style(
        Style::default()
            .bg(Color::Rgb(16, 20, 29))
            .fg(Color::Rgb(237, 243, 255)),
    );
    frame.render_widget(outer, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(area);

    draw_header(frame, layout[0], app);
    draw_body(frame, layout[1], app);
    draw_status(frame, layout[2], app);

    if app.show_inspector {
        draw_inspector_popup(frame, centered_rect(38, 44, area), app);
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let header = Block::default().style(Style::default().bg(Color::Rgb(22, 30, 44)));
    frame.render_widget(header, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(52), Constraint::Min(10)])
        .split(area.inner(Margin {
            horizontal: 2,
            vertical: 1,
        }));

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "ZITADEL TUI",
            Style::default()
                .fg(Color::Rgb(145, 194, 255))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            "Command Atelier",
            Style::default().fg(Color::Rgb(236, 156, 114)),
        ),
    ]));
    frame.render_widget(title, chunks[0]);

    let auth_style = if app.setup_required {
        Style::default()
            .fg(Color::Rgb(245, 194, 66))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Rgb(112, 224, 144))
            .add_modifier(Modifier::BOLD)
    };

    let right = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{} {}", app.auth_label, status_mark(app)),
            auth_style,
        ),
        Span::raw("   "),
        Span::styled(
            format!("project {}", app.project),
            Style::default().fg(Color::Rgb(176, 188, 213)),
        ),
        Span::raw("   "),
        Span::styled(
            "[i] inspector",
            Style::default().fg(Color::Rgb(145, 194, 255)),
        ),
    ]))
    .alignment(Alignment::Right);
    frame.render_widget(right, chunks[1]);

    let host = Paragraph::new(app.host.clone())
        .style(Style::default().fg(Color::Rgb(130, 140, 163)))
        .alignment(Alignment::Right);
    frame.render_widget(
        host,
        Rect::new(area.x + area.width.saturating_sub(34), area.y, 32, 1),
    );
}

fn draw_body(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Min(10)])
        .split(area);
    draw_command_rail(frame, chunks[0], app);
    draw_workspace(frame, chunks[1], app);
}

fn draw_command_rail(frame: &mut Frame, area: Rect, app: &App) {
    let rail = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::Rgb(45, 56, 73)))
        .style(Style::default().bg(Color::Rgb(15, 22, 33)));
    frame.render_widget(rail, area);

    let inner = area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Min(0),
        ])
        .split(inner);

    let resources: Vec<ListItem> = app
        .resources
        .iter()
        .enumerate()
        .map(|(index, item)| {
            list_item(
                item.name,
                &item.count,
                index == app.selected_resource && app.focus == Focus::Resources,
                Color::Rgb(145, 194, 255),
            )
        })
        .collect();
    frame.render_widget(
        List::new(resources).block(section_block("Command rail", "[h/l] resource")),
        sections[0],
    );

    let actions: Vec<ListItem> = app
        .actions()
        .iter()
        .enumerate()
        .map(|(index, item)| {
            list_item(
                item.label,
                item.hotkey,
                index == app.selected_action && app.focus == Focus::Actions,
                Color::Rgb(236, 156, 114),
            )
        })
        .collect();
    frame.render_widget(
        List::new(actions).block(section_block("Actions", "[Enter] run")),
        sections[1],
    );

    let footer = Paragraph::new(footer_lines(app))
        .wrap(Wrap { trim: true })
        .block(section_block("Status", ""));
    frame.render_widget(footer, sections[2]);
}

fn draw_workspace(frame: &mut Frame, area: Rect, app: &App) {
    let inner = area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(9)])
        .split(inner);
    draw_canvas(frame, sections[0], app);
    draw_selection_tray(frame, sections[1], app);
}

fn draw_canvas(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(63), Constraint::Percentage(37)])
        .split(area);
    let canvas_title = app.canvas_title();
    let canvas_meta = app.canvas_meta();

    let form_block = Block::default()
        .borders(Borders::ALL)
        .border_style(focus_border(app.focus == Focus::Form))
        .style(Style::default().bg(Color::Rgb(18, 26, 39)))
        .title(title_span("Canvas", &canvas_title));
    frame.render_widget(form_block, chunks[0]);

    let form_lines = app.message_lines().join("\n");
    let form = Paragraph::new(form_lines)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);
    frame.render_widget(
        form,
        chunks[0].inner(Margin {
            horizontal: 2,
            vertical: 1,
        }),
    );

    let review_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(47, 58, 79)))
        .style(Style::default().bg(Color::Rgb(19, 27, 38)))
        .title(title_span("Review", &canvas_meta));
    frame.render_widget(review_block, chunks[1]);

    let review = Paragraph::new(app.review_lines()).wrap(Wrap { trim: true });
    frame.render_widget(
        review,
        chunks[1].inner(Margin {
            horizontal: 2,
            vertical: 1,
        }),
    );
}

fn draw_selection_tray(frame: &mut Frame, area: Rect, app: &App) {
    let tray = Block::default()
        .borders(Borders::ALL)
        .border_style(focus_border(app.focus == Focus::Records))
        .style(Style::default().bg(Color::Rgb(18, 26, 39)))
        .title(title_span("Selection tray", selection_title(app)));
    frame.render_widget(tray, area);

    if app.active_records().is_empty() {
        let empty = Paragraph::new("No records loaded for this resource.")
            .style(Style::default().fg(Color::Rgb(121, 131, 151)));
        frame.render_widget(
            empty,
            area.inner(Margin {
                horizontal: 2,
                vertical: 1,
            }),
        );
        return;
    }

    let rows: Vec<ListItem> = app
        .active_records()
        .iter()
        .enumerate()
        .map(|(index, record)| {
            list_item(
                &record.name,
                &record.kind,
                index == app.selected_record,
                Color::Rgb(145, 194, 255),
            )
        })
        .collect();
    frame.render_widget(
        List::new(rows),
        area.inner(Margin {
            horizontal: 2,
            vertical: 1,
        }),
    );
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let status = Block::default()
        .style(Style::default().bg(Color::Rgb(12, 18, 29)))
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::Rgb(45, 56, 73)));
    frame.render_widget(status, area);

    let selected = app
        .selected_record()
        .map(|record| record.name.as_str())
        .unwrap_or("none");
    let text = format!(
        "{}. [Tab] focus  [Enter] act  [Esc] back  [j/k] move  [i] inspector  Selected: {}",
        resource_label(app.active_resource()),
        selected
    );
    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(Color::Rgb(130, 140, 163)))
            .alignment(Alignment::Left),
        area.inner(Margin {
            horizontal: 2,
            vertical: 0,
        }),
    );
}

fn draw_inspector_popup(frame: &mut Frame, area: Rect, app: &App) {
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(145, 194, 255)))
        .style(Style::default().bg(Color::Rgb(10, 14, 22)))
        .title(title_span("Inspector popup", "[i] close"));
    frame.render_widget(block, area);

    let lines = if let Some(record) = app.selected_record() {
        vec![
            field_line("Name", &record.name),
            field_line("ID", &record.id),
            field_line("Kind", &record.kind),
            field_line("Summary", &record.summary),
            field_line("Detail", &record.detail),
            field_line("Changed", &record.changed_at),
        ]
    } else {
        vec![
            bold_line("No selected record"),
            muted_line("Choose a record in the tray to inspect more detail."),
        ]
    };

    let text = Paragraph::new(lines).wrap(Wrap { trim: true });
    frame.render_widget(
        text,
        area.inner(Margin {
            horizontal: 2,
            vertical: 1,
        }),
    );
}

fn footer_lines(app: &App) -> Vec<Line<'static>> {
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

fn browse_title(app: &App) -> &'static str {
    match app.active_resource() {
        ResourceKind::Applications => "OIDC application workspace",
        ResourceKind::Users => "User management workspace",
        ResourceKind::Idps => "Identity provider workspace",
        ResourceKind::Auth => "Authentication setup",
        ResourceKind::Config => "Configuration editor",
    }
}

fn browse_meta(app: &App) -> &'static str {
    match app.active_resource() {
        ResourceKind::Applications => "choose an action",
        ResourceKind::Users => "choose an action",
        ResourceKind::Idps => "choose an action",
        ResourceKind::Auth => "run setup",
        ResourceKind::Config => "edit or import",
    }
}

fn browse_lines(app: &App) -> Vec<String> {
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

fn browse_review_lines(app: &App) -> Vec<Line<'static>> {
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

fn form_review_lines(form: &FormState) -> Vec<Line<'static>> {
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

fn confirm_review_lines(confirm: &ConfirmState) -> Vec<Line<'static>> {
    vec![
        bold_line(&confirm.title),
        subtle_line(&confirm.submit_label),
        Line::from(""),
        field_line("Pending", pending_label(&confirm.pending)),
        muted_line("Press Enter to confirm or Esc to cancel."),
    ]
}

fn message_review_lines(message: &MessageState) -> Vec<Line<'static>> {
    vec![
        bold_line(&message.title),
        subtle_line("workflow result"),
        Line::from(""),
        field_line("Lines", &message.lines.len().to_string()),
        muted_line("Press Enter or Esc to return to the workspace."),
    ]
}

fn render_form_line(field: &FormField, selected: bool) -> String {
    let marker = if selected { "›" } else { " " };
    let value = match field.kind {
        FieldKind::Secret => "•".repeat(field.value.len().max(1)),
        FieldKind::Toggle | FieldKind::Checkbox => {
            if is_enabled(&field.value) {
                "[x]".to_string()
            } else {
                "[ ]".to_string()
            }
        }
        FieldKind::Choice(_) | FieldKind::Text => field.value.clone(),
    };
    format!("{marker} {:<18} {}", field.label, value)
}

fn pending_label(pending: &PendingAction) -> &'static str {
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

fn default_setup_form(bootstrap: &TuiBootstrap) -> FormState {
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

fn selection_title(app: &App) -> &'static str {
    match app.active_resource() {
        ResourceKind::Applications => "existing applications",
        ResourceKind::Users => "existing users",
        ResourceKind::Idps => "configured identity providers",
        ResourceKind::Auth => "setup state",
        ResourceKind::Config => "saved values",
    }
}

fn resource_label(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Applications => "Applications",
        ResourceKind::Users => "Users",
        ResourceKind::Idps => "IDPs",
        ResourceKind::Auth => "Auth",
        ResourceKind::Config => "Config",
    }
}

fn focus_label(focus: Focus) -> &'static str {
    match focus {
        Focus::Resources => "resources",
        Focus::Actions => "actions",
        Focus::Form => "form",
        Focus::Records => "records",
    }
}

fn status_mark(app: &App) -> &'static str {
    if app.setup_required {
        "!"
    } else {
        "✓"
    }
}

fn list_item(label: &str, meta: &str, selected: bool, accent: Color) -> ListItem<'static> {
    let style = if selected {
        Style::default()
            .bg(Color::Rgb(31, 48, 76))
            .fg(Color::Rgb(237, 243, 255))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(210, 219, 239))
    };
    ListItem::new(Line::from(vec![
        Span::styled(label.to_string(), style),
        Span::raw(" "),
        Span::styled(meta.to_string(), Style::default().fg(accent)),
    ]))
}

fn section_block<'a>(title: &'a str, meta: &'a str) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(47, 58, 79)))
        .style(Style::default().bg(Color::Rgb(18, 26, 39)))
        .title(title_span(title, meta))
}

fn title_span<'a>(title: &'a str, meta: &'a str) -> Line<'a> {
    let mut spans = vec![Span::styled(
        title,
        Style::default()
            .fg(Color::Rgb(176, 188, 213))
            .add_modifier(Modifier::BOLD),
    )];
    if !meta.is_empty() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            meta,
            Style::default().fg(Color::Rgb(121, 131, 151)),
        ));
    }
    Line::from(spans)
}

fn field_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:<15}"),
            Style::default().fg(Color::Rgb(121, 131, 151)),
        ),
        Span::styled(
            value.to_string(),
            Style::default().fg(Color::Rgb(237, 243, 255)),
        ),
    ])
}

fn bold_line(value: &str) -> Line<'static> {
    Line::from(Span::styled(
        value.to_string(),
        Style::default()
            .fg(Color::Rgb(237, 243, 255))
            .add_modifier(Modifier::BOLD),
    ))
}

fn subtle_line(value: &str) -> Line<'static> {
    Line::from(Span::styled(
        value.to_string(),
        Style::default().fg(Color::Rgb(176, 188, 213)),
    ))
}

fn status_heading(value: &str) -> Line<'static> {
    Line::from(Span::styled(
        value.to_string(),
        Style::default()
            .fg(Color::Rgb(176, 188, 213))
            .add_modifier(Modifier::BOLD),
    ))
}

fn muted_line(value: &str) -> Line<'static> {
    Line::from(Span::styled(
        value.to_string(),
        Style::default().fg(Color::Rgb(121, 131, 151)),
    ))
}

fn toggle_field(field: &mut FormField) {
    field.value = if is_enabled(&field.value) {
        "false".to_string()
    } else {
        "true".to_string()
    };
}

fn cycle_choice(field: &mut FormField, options: &[String], forward: bool) {
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

fn is_enabled(value: &str) -> bool {
    matches!(value, "true" | "yes" | "on" | "1")
}

fn focus_border(selected: bool) -> Style {
    if selected {
        Style::default().fg(Color::Rgb(145, 194, 255))
    } else {
        Style::default().fg(Color::Rgb(47, 58, 79))
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        App::from_bootstrap(TuiBootstrap {
            host: "https://zitadel.example.com".to_string(),
            project: "core".to_string(),
            auth_label: "PAT".to_string(),
            templates_path: Some("/tmp/apps.yml".to_string()),
            setup_required: false,
            app_records: vec![],
            user_records: vec![],
            idp_records: vec![],
        })
    }

    #[test]
    fn focus_cycles_forward() {
        let mut app = test_app();
        app.advance_focus();
        assert_eq!(app.focus, Focus::Actions);
        app.advance_focus();
        assert_eq!(app.focus, Focus::Form);
        app.advance_focus();
        assert_eq!(app.focus, Focus::Records);
        app.advance_focus();
        assert_eq!(app.focus, Focus::Resources);
    }

    #[test]
    fn toggles_inspector_popup() {
        let mut app = test_app();
        assert!(!app.show_inspector);
        app.toggle_inspector();
        assert!(app.show_inspector);
    }

    #[test]
    fn action_navigation_tracks_current_resource() {
        let mut app = test_app();
        app.next_action();
        assert_eq!(
            app.actions()[app.selected_action].label,
            "Regenerate secret"
        );
        app.next_resource();
        assert_eq!(app.actions()[app.selected_action].label, "Create user");
    }

    #[test]
    fn focus_cycles_backward() {
        let mut app = test_app();
        app.reverse_focus();
        assert_eq!(app.focus, Focus::Records);
        app.reverse_focus();
        assert_eq!(app.focus, Focus::Form);
        app.reverse_focus();
        assert_eq!(app.focus, Focus::Actions);
    }

    #[test]
    fn resource_navigation_wraps_and_resets_selection() {
        let mut app = test_app();
        app.selected_action = 2;
        app.selected_record = 1;

        app.next_resource();
        assert_eq!(app.active_resource(), ResourceKind::Users);
        assert_eq!(app.selected_action, 0);
        assert_eq!(app.selected_record, 0);

        app.previous_resource();
        assert_eq!(app.active_resource(), ResourceKind::Applications);
        assert_eq!(app.selected_action, 0);
        assert_eq!(app.selected_record, 0);
    }

    #[test]
    fn empty_bootstrap_keeps_empty_records() {
        let app = App::from_bootstrap(TuiBootstrap {
            host: "https://zitadel.example.com".to_string(),
            project: "core".to_string(),
            auth_label: "PAT".to_string(),
            templates_path: None,
            setup_required: false,
            app_records: vec![],
            user_records: vec![],
            idp_records: vec![],
        });

        assert!(app.app_records.is_empty());
        assert!(app.user_records.is_empty());
        assert!(app.idp_records.is_empty());
        assert_eq!(app.resources[0].count, "0");
        assert_eq!(app.resources[1].count, "0");
        assert_eq!(app.resources[2].count, "0");
        assert!(matches!(app.canvas_mode, CanvasMode::Browse));
        assert!(app.selected_record().is_none());
    }

    #[test]
    fn setup_mode_uses_setup_form() {
        let app = App::from_bootstrap(TuiBootstrap {
            host: "https://zitadel.example.com".to_string(),
            project: "core".to_string(),
            auth_label: "Setup required".to_string(),
            templates_path: None,
            setup_required: true,
            app_records: vec![],
            user_records: vec![],
            idp_records: vec![],
        });

        assert!(matches!(app.canvas_mode, CanvasMode::Setup(_)));
    }

    #[test]
    fn form_editing_changes_selected_field() {
        let mut app = test_app();
        let mut form = default_setup_form(&TuiBootstrap {
            host: "https://zitadel.example.com".to_string(),
            project: "core".to_string(),
            auth_label: "PAT".to_string(),
            templates_path: None,
            setup_required: true,
            app_records: vec![],
            user_records: vec![],
            idp_records: vec![],
        });
        form.selected_field = 3;
        app.set_canvas_mode(CanvasMode::Setup(form));
        app.form_insert_char('x');
        let CanvasMode::Setup(form) = &app.canvas_mode else {
            panic!("expected setup mode");
        };
        assert_eq!(form.fields[3].value, "x");
    }

    #[test]
    fn form_toggle_and_choice_cycle() {
        let mut app = test_app();
        let mut form = default_setup_form(&TuiBootstrap {
            host: "https://zitadel.example.com".to_string(),
            project: "core".to_string(),
            auth_label: "PAT".to_string(),
            templates_path: None,
            setup_required: true,
            app_records: vec![],
            user_records: vec![],
            idp_records: vec![],
        });

        form.selected_field = 2;
        app.set_canvas_mode(CanvasMode::Setup(form));
        app.form_toggle_or_cycle(true);
        let CanvasMode::Setup(form) = &app.canvas_mode else {
            panic!("expected setup mode");
        };
        assert_eq!(form.fields[2].value, "Service account");

        app.form_toggle_or_cycle(true);
        let CanvasMode::Setup(form) = &app.canvas_mode else {
            panic!("expected setup mode");
        };
        assert_eq!(form.fields[2].value, "OAuth device (placeholder)");

        app.form_toggle_or_cycle(false);
        let CanvasMode::Setup(form) = &app.canvas_mode else {
            panic!("expected setup mode");
        };
        assert_eq!(form.fields[2].value, "Service account");
    }

    #[test]
    fn reset_to_browse_returns_to_setup_when_required() {
        let mut app = App::from_bootstrap(TuiBootstrap {
            host: "https://zitadel.example.com".to_string(),
            project: "core".to_string(),
            auth_label: "Setup required".to_string(),
            templates_path: Some("/tmp/apps.yml".to_string()),
            setup_required: true,
            app_records: vec![],
            user_records: vec![],
            idp_records: vec![],
        });

        app.reset_to_browse();
        assert!(matches!(app.canvas_mode, CanvasMode::Setup(_)));
        assert_eq!(app.focus, Focus::Resources);
    }

    #[test]
    fn reset_to_browse_returns_to_browser_when_ready() {
        let mut app = test_app();
        app.reset_to_browse();
        assert!(matches!(app.canvas_mode, CanvasMode::Browse));
        assert_eq!(app.focus, Focus::Resources);
    }

    #[test]
    fn sync_runtime_updates_counts_without_fallback_records() {
        let mut app = test_app();
        app.selected_record = 1;
        app.selected_action = 2;

        app.sync_runtime(TuiBootstrap {
            host: "https://zitadel.example.com".to_string(),
            project: "ops".to_string(),
            auth_label: "Service account".to_string(),
            templates_path: None,
            setup_required: true,
            app_records: vec![],
            user_records: vec![],
            idp_records: vec![],
        });

        assert_eq!(app.project, "ops");
        assert_eq!(app.auth_label, "Service account");
        assert_eq!(app.resources[0].count, "0");
        assert_eq!(app.resources[1].count, "0");
        assert_eq!(app.resources[2].count, "0");
        assert_eq!(app.resources[3].count, "setup");
        assert!(app.app_records.is_empty());
        assert!(app.user_records.is_empty());
        assert!(app.idp_records.is_empty());
        assert_eq!(app.selected_record, 0);
        assert_eq!(app.selected_action, 0);
    }

    #[test]
    fn record_navigation_wraps_even_when_empty() {
        let mut app = test_app();
        app.next_record();
        assert_eq!(app.focus, Focus::Records);
        assert_eq!(app.selected_record, 0);
        app.previous_record();
        assert_eq!(app.focus, Focus::Records);
        assert_eq!(app.selected_record, 0);
    }

    #[test]
    fn render_form_line_text_field_selected() {
        let field = FormField {
            key: "host",
            label: "Host".to_string(),
            value: "https://z.example.com".to_string(),
            kind: FieldKind::Text,
            help: String::new(),
        };
        let line = render_form_line(&field, true);
        assert!(line.starts_with("›"));
        assert!(line.contains("Host"));
        assert!(line.contains("https://z.example.com"));
    }

    #[test]
    fn render_form_line_text_field_unselected() {
        let field = FormField {
            key: "host",
            label: "Host".to_string(),
            value: "value".to_string(),
            kind: FieldKind::Text,
            help: String::new(),
        };
        let line = render_form_line(&field, false);
        assert!(line.starts_with(" "));
    }

    #[test]
    fn render_form_line_secret_masks_value() {
        let field = FormField {
            key: "token",
            label: "PAT".to_string(),
            value: "abc".to_string(),
            kind: FieldKind::Secret,
            help: String::new(),
        };
        let line = render_form_line(&field, false);
        assert!(line.contains("•••"));
        assert!(!line.contains("abc"));
    }

    #[test]
    fn render_form_line_secret_empty_shows_single_dot() {
        let field = FormField {
            key: "token",
            label: "PAT".to_string(),
            value: String::new(),
            kind: FieldKind::Secret,
            help: String::new(),
        };
        let line = render_form_line(&field, false);
        assert!(line.contains("•"));
    }

    #[test]
    fn render_form_line_toggle_enabled() {
        let field = FormField {
            key: "flag",
            label: "Admin".to_string(),
            value: "true".to_string(),
            kind: FieldKind::Toggle,
            help: String::new(),
        };
        let line = render_form_line(&field, false);
        assert!(line.contains("[x]"));
    }

    #[test]
    fn render_form_line_toggle_disabled() {
        let field = FormField {
            key: "flag",
            label: "Admin".to_string(),
            value: "false".to_string(),
            kind: FieldKind::Toggle,
            help: String::new(),
        };
        let line = render_form_line(&field, false);
        assert!(line.contains("[ ]"));
    }

    #[test]
    fn render_form_line_checkbox_enabled() {
        let field = FormField {
            key: "cb",
            label: "Enable".to_string(),
            value: "true".to_string(),
            kind: FieldKind::Checkbox,
            help: String::new(),
        };
        let line = render_form_line(&field, false);
        assert!(line.contains("[x]"));
    }

    #[test]
    fn render_form_line_choice_shows_value() {
        let field = FormField {
            key: "method",
            label: "Auth".to_string(),
            value: "PAT".to_string(),
            kind: FieldKind::Choice(vec!["PAT".to_string(), "SA".to_string()]),
            help: String::new(),
        };
        let line = render_form_line(&field, false);
        assert!(line.contains("PAT"));
    }

    #[test]
    fn pending_label_covers_all_variants() {
        assert_eq!(
            pending_label(&PendingAction::CreateApplication),
            "create application"
        );
        assert_eq!(
            pending_label(&PendingAction::QuickSetupApplications),
            "quick setup apps"
        );
        assert_eq!(
            pending_label(&PendingAction::DeleteApplication {
                app_id: "a".to_string(),
                name: "b".to_string()
            }),
            "delete application"
        );
        assert_eq!(
            pending_label(&PendingAction::RegenerateSecret {
                app_id: "a".to_string(),
                name: "b".to_string(),
                client_id: "c".to_string()
            }),
            "regenerate secret"
        );
        assert_eq!(pending_label(&PendingAction::CreateUser), "create user");
        assert_eq!(
            pending_label(&PendingAction::CreateAdminUser),
            "create admin user"
        );
        assert_eq!(
            pending_label(&PendingAction::GrantIamOwner {
                user_id: "u".to_string(),
                username: "n".to_string()
            }),
            "grant IAM_OWNER"
        );
        assert_eq!(
            pending_label(&PendingAction::QuickSetupUsers),
            "quick setup users"
        );
        assert_eq!(
            pending_label(&PendingAction::ConfigureGoogleIdp),
            "configure Google IDP"
        );
        assert_eq!(
            pending_label(&PendingAction::ValidateAuthSetup),
            "validate auth setup"
        );
        assert_eq!(pending_label(&PendingAction::SaveConfig), "save config");
    }

    #[test]
    fn is_enabled_recognizes_truthy_values() {
        assert!(is_enabled("true"));
        assert!(is_enabled("yes"));
        assert!(is_enabled("on"));
        assert!(is_enabled("1"));
        assert!(!is_enabled("false"));
        assert!(!is_enabled(""));
        assert!(!is_enabled("maybe"));
    }

    #[test]
    fn toggle_field_flips_value() {
        let mut field = FormField {
            key: "flag",
            label: "Flag".to_string(),
            value: "false".to_string(),
            kind: FieldKind::Toggle,
            help: String::new(),
        };
        toggle_field(&mut field);
        assert_eq!(field.value, "true");
        toggle_field(&mut field);
        assert_eq!(field.value, "false");
    }

    #[test]
    fn cycle_choice_forward() {
        let options = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let mut field = FormField {
            key: "opt",
            label: "Opt".to_string(),
            value: "a".to_string(),
            kind: FieldKind::Choice(options.clone()),
            help: String::new(),
        };
        cycle_choice(&mut field, &options, true);
        assert_eq!(field.value, "b");
        cycle_choice(&mut field, &options, true);
        assert_eq!(field.value, "c");
        cycle_choice(&mut field, &options, true);
        assert_eq!(field.value, "a");
    }

    #[test]
    fn cycle_choice_backward() {
        let options = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let mut field = FormField {
            key: "opt",
            label: "Opt".to_string(),
            value: "a".to_string(),
            kind: FieldKind::Choice(options.clone()),
            help: String::new(),
        };
        cycle_choice(&mut field, &options, false);
        assert_eq!(field.value, "c");
        cycle_choice(&mut field, &options, false);
        assert_eq!(field.value, "b");
    }

    #[test]
    fn cycle_choice_unknown_value_resets_to_first() {
        let options = vec!["a".to_string(), "b".to_string()];
        let mut field = FormField {
            key: "opt",
            label: "Opt".to_string(),
            value: "unknown".to_string(),
            kind: FieldKind::Choice(options.clone()),
            help: String::new(),
        };
        cycle_choice(&mut field, &options, true);
        assert_eq!(field.value, "a");
    }

    #[test]
    fn form_backspace_removes_last_char() {
        let mut app = test_app();
        let form = FormState {
            title: "Test".to_string(),
            description: String::new(),
            submit_label: String::new(),
            fields: vec![FormField {
                key: "name",
                label: "Name".to_string(),
                value: "hello".to_string(),
                kind: FieldKind::Text,
                help: String::new(),
            }],
            selected_field: 0,
            pending: PendingAction::SaveConfig,
        };
        app.set_canvas_mode(CanvasMode::EditForm(form));
        app.form_backspace();
        let CanvasMode::EditForm(form) = &app.canvas_mode else {
            panic!("expected EditForm");
        };
        assert_eq!(form.fields[0].value, "hell");
    }

    #[test]
    fn form_backspace_noop_on_toggle() {
        let mut app = test_app();
        let form = FormState {
            title: "Test".to_string(),
            description: String::new(),
            submit_label: String::new(),
            fields: vec![FormField {
                key: "flag",
                label: "Flag".to_string(),
                value: "true".to_string(),
                kind: FieldKind::Toggle,
                help: String::new(),
            }],
            selected_field: 0,
            pending: PendingAction::SaveConfig,
        };
        app.set_canvas_mode(CanvasMode::EditForm(form));
        app.form_backspace();
        let CanvasMode::EditForm(form) = &app.canvas_mode else {
            panic!("expected EditForm");
        };
        assert_eq!(form.fields[0].value, "true");
    }

    #[test]
    fn form_next_field_wraps() {
        let mut app = test_app();
        let form = FormState {
            title: "Test".to_string(),
            description: String::new(),
            submit_label: String::new(),
            fields: vec![
                FormField {
                    key: "a",
                    label: "A".to_string(),
                    value: String::new(),
                    kind: FieldKind::Text,
                    help: String::new(),
                },
                FormField {
                    key: "b",
                    label: "B".to_string(),
                    value: String::new(),
                    kind: FieldKind::Text,
                    help: String::new(),
                },
            ],
            selected_field: 0,
            pending: PendingAction::SaveConfig,
        };
        app.set_canvas_mode(CanvasMode::EditForm(form));
        app.form_next_field();
        let CanvasMode::EditForm(form) = &app.canvas_mode else {
            panic!("expected EditForm");
        };
        assert_eq!(form.selected_field, 1);
        app.form_next_field();
        let CanvasMode::EditForm(form) = &app.canvas_mode else {
            panic!("expected EditForm");
        };
        assert_eq!(form.selected_field, 0);
    }

    #[test]
    fn form_previous_field_wraps() {
        let mut app = test_app();
        let form = FormState {
            title: "Test".to_string(),
            description: String::new(),
            submit_label: String::new(),
            fields: vec![
                FormField {
                    key: "a",
                    label: "A".to_string(),
                    value: String::new(),
                    kind: FieldKind::Text,
                    help: String::new(),
                },
                FormField {
                    key: "b",
                    label: "B".to_string(),
                    value: String::new(),
                    kind: FieldKind::Text,
                    help: String::new(),
                },
            ],
            selected_field: 0,
            pending: PendingAction::SaveConfig,
        };
        app.set_canvas_mode(CanvasMode::EditForm(form));
        app.form_previous_field();
        let CanvasMode::EditForm(form) = &app.canvas_mode else {
            panic!("expected EditForm");
        };
        assert_eq!(form.selected_field, 1);
    }

    #[test]
    fn selected_record_returns_record_when_present() {
        let mut app = App::from_bootstrap(TuiBootstrap {
            host: "https://zitadel.example.com".to_string(),
            project: "core".to_string(),
            auth_label: "PAT".to_string(),
            templates_path: None,
            setup_required: false,
            app_records: vec![Record {
                id: "app-1".to_string(),
                name: "grafana".to_string(),
                kind: "public".to_string(),
                summary: "1 redirect".to_string(),
                detail: "cid-1".to_string(),
                changed_at: "ACTIVE".to_string(),
            }],
            user_records: vec![],
            idp_records: vec![],
        });
        app.selected_record = 0;
        let record = app.selected_record();
        assert!(record.is_some());
        assert_eq!(record.unwrap().name, "grafana");
    }

    #[test]
    fn resource_label_covers_all_kinds() {
        assert_eq!(resource_label(ResourceKind::Applications), "Applications");
        assert_eq!(resource_label(ResourceKind::Users), "Users");
        assert_eq!(resource_label(ResourceKind::Idps), "IDPs");
        assert_eq!(resource_label(ResourceKind::Auth), "Auth");
        assert_eq!(resource_label(ResourceKind::Config), "Config");
    }

    #[test]
    fn focus_label_covers_all_foci() {
        assert_eq!(focus_label(Focus::Resources), "resources");
        assert_eq!(focus_label(Focus::Actions), "actions");
        assert_eq!(focus_label(Focus::Form), "form");
        assert_eq!(focus_label(Focus::Records), "records");
    }

    #[test]
    fn status_mark_setup_required() {
        let mut app = test_app();
        app.setup_required = true;
        assert_eq!(status_mark(&app), "!");
    }

    #[test]
    fn status_mark_ready() {
        let app = test_app();
        assert_eq!(status_mark(&app), "✓");
    }

    #[test]
    fn selection_title_per_resource() {
        let mut app = test_app();
        assert_eq!(selection_title(&app), "existing applications");
        app.next_resource();
        assert_eq!(selection_title(&app), "existing users");
        app.next_resource();
        assert_eq!(selection_title(&app), "configured identity providers");
    }

    #[test]
    fn set_canvas_mode_sets_focus_to_form_for_edit() {
        let mut app = test_app();
        let form = FormState {
            title: "Test".to_string(),
            description: String::new(),
            submit_label: String::new(),
            fields: vec![FormField {
                key: "a",
                label: "A".to_string(),
                value: String::new(),
                kind: FieldKind::Text,
                help: String::new(),
            }],
            selected_field: 0,
            pending: PendingAction::SaveConfig,
        };
        app.set_canvas_mode(CanvasMode::EditForm(form));
        assert_eq!(app.focus, Focus::Form);
    }

    #[test]
    fn set_canvas_mode_browse_sets_focus_to_resources() {
        let mut app = test_app();
        app.focus = Focus::Form;
        app.set_canvas_mode(CanvasMode::Browse);
        assert_eq!(app.focus, Focus::Resources);
    }

    #[test]
    fn canvas_title_browse_mode() {
        let app = test_app();
        let title = app.canvas_title();
        assert!(!title.is_empty());
    }

    #[test]
    fn canvas_title_error_mode() {
        let mut app = test_app();
        app.set_canvas_mode(CanvasMode::Error(MessageState {
            title: "Something failed".to_string(),
            lines: vec!["detail".to_string()],
        }));
        assert_eq!(app.canvas_title(), "Something failed");
    }

    #[test]
    fn message_lines_success_mode() {
        let mut app = test_app();
        app.set_canvas_mode(CanvasMode::Success(MessageState {
            title: "Done".to_string(),
            lines: vec!["line1".to_string(), "line2".to_string()],
        }));
        let lines = app.message_lines();
        assert_eq!(lines, vec!["line1".to_string(), "line2".to_string()]);
    }

    #[test]
    fn message_lines_form_mode_renders_fields() {
        let mut app = test_app();
        let form = FormState {
            title: "Test".to_string(),
            description: String::new(),
            submit_label: String::new(),
            fields: vec![FormField {
                key: "host",
                label: "Host".to_string(),
                value: "https://z.example.com".to_string(),
                kind: FieldKind::Text,
                help: String::new(),
            }],
            selected_field: 0,
            pending: PendingAction::SaveConfig,
        };
        app.set_canvas_mode(CanvasMode::EditForm(form));
        let lines = app.message_lines();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("Host"));
    }

    #[test]
    fn record_navigation_wraps_with_records() {
        let mut app = App::from_bootstrap(TuiBootstrap {
            host: "https://zitadel.example.com".to_string(),
            project: "core".to_string(),
            auth_label: "PAT".to_string(),
            templates_path: None,
            setup_required: false,
            app_records: vec![
                Record {
                    id: "a1".to_string(),
                    name: "app1".to_string(),
                    kind: "public".to_string(),
                    summary: String::new(),
                    detail: String::new(),
                    changed_at: String::new(),
                },
                Record {
                    id: "a2".to_string(),
                    name: "app2".to_string(),
                    kind: "public".to_string(),
                    summary: String::new(),
                    detail: String::new(),
                    changed_at: String::new(),
                },
            ],
            user_records: vec![],
            idp_records: vec![],
        });
        app.next_record();
        assert_eq!(app.selected_record, 1);
        app.next_record();
        assert_eq!(app.selected_record, 0);
        app.previous_record();
        assert_eq!(app.selected_record, 1);
    }

    #[test]
    fn form_insert_char_space_toggles_toggle_field() {
        let mut app = test_app();
        let form = FormState {
            title: "Test".to_string(),
            description: String::new(),
            submit_label: String::new(),
            fields: vec![FormField {
                key: "flag",
                label: "Admin".to_string(),
                value: "false".to_string(),
                kind: FieldKind::Toggle,
                help: String::new(),
            }],
            selected_field: 0,
            pending: PendingAction::SaveConfig,
        };
        app.set_canvas_mode(CanvasMode::EditForm(form));
        app.form_insert_char(' ');
        let CanvasMode::EditForm(form) = &app.canvas_mode else {
            panic!("expected EditForm");
        };
        assert_eq!(form.fields[0].value, "true");
    }

    #[test]
    fn form_insert_char_space_cycles_choice() {
        let mut app = test_app();
        let form = FormState {
            title: "Test".to_string(),
            description: String::new(),
            submit_label: String::new(),
            fields: vec![FormField {
                key: "method",
                label: "Auth".to_string(),
                value: "PAT".to_string(),
                kind: FieldKind::Choice(vec!["PAT".to_string(), "SA".to_string()]),
                help: String::new(),
            }],
            selected_field: 0,
            pending: PendingAction::SaveConfig,
        };
        app.set_canvas_mode(CanvasMode::EditForm(form));
        app.form_insert_char(' ');
        let CanvasMode::EditForm(form) = &app.canvas_mode else {
            panic!("expected EditForm");
        };
        assert_eq!(form.fields[0].value, "SA");
    }

    #[test]
    fn action_navigation_wraps() {
        let mut app = test_app();
        let action_count = app.actions().len();
        for _ in 0..action_count {
            app.next_action();
        }
        assert_eq!(app.selected_action, 0);
    }

    #[test]
    fn previous_action_wraps_to_last() {
        let mut app = test_app();
        app.previous_action();
        let last = app.actions().len() - 1;
        assert_eq!(app.selected_action, last);
    }
}
