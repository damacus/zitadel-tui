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

#[derive(Clone, Debug)]
pub struct Record {
    pub name: String,
    pub kind: String,
    pub redirects: String,
    pub changed_at: String,
}

#[derive(Debug)]
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
}

impl App {
    pub fn from_bootstrap(bootstrap: TuiBootstrap) -> Self {
        let app_records = if bootstrap.app_records.is_empty() {
            sample_app_records()
        } else {
            bootstrap.app_records
        };
        let templates_path = bootstrap.templates_path.clone();
        let setup_required = bootstrap.setup_required;

        Self {
            focus: Focus::Resources,
            show_inspector: false,
            host: bootstrap.host,
            project: bootstrap.project,
            auth_label: bootstrap.auth_label,
            templates_path,
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
                    count: bootstrap.user_records.len().to_string(),
                },
                Resource {
                    kind: ResourceKind::Idps,
                    name: "IDPs",
                    count: bootstrap.idp_records.len().to_string(),
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
                    count: if bootstrap.templates_path.is_some() {
                        "templated".to_string()
                    } else {
                        "local".to_string()
                    },
                },
            ],
            selected_resource: 0,
            selected_action: 0,
            app_records,
            user_records: bootstrap.user_records,
            idp_records: bootstrap.idp_records,
            selected_record: 0,
        }
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
}

const APPLICATION_ACTIONS: [Action; 4] = [
    Action {
        label: "Create application",
        hotkey: "[n]",
    },
    Action {
        label: "Regenerate secret",
        hotkey: "[r]",
    },
    Action {
        label: "Delete selected",
        hotkey: "[d]",
    },
    Action {
        label: "Quick setup",
        hotkey: "[s]",
    },
];

const USER_ACTIONS: [Action; 4] = [
    Action {
        label: "Create user",
        hotkey: "[n]",
    },
    Action {
        label: "Create admin",
        hotkey: "[a]",
    },
    Action {
        label: "Grant IAM_OWNER",
        hotkey: "[g]",
    },
    Action {
        label: "Quick setup",
        hotkey: "[s]",
    },
];

const IDP_ACTIONS: [Action; 2] = [
    Action {
        label: "Configure Google",
        hotkey: "[n]",
    },
    Action {
        label: "Refresh providers",
        hotkey: "[r]",
    },
];

const AUTH_ACTIONS: [Action; 2] = [
    Action {
        label: "Validate token",
        hotkey: "[v]",
    },
    Action {
        label: "Run setup",
        hotkey: "[n]",
    },
];

const CONFIG_ACTIONS: [Action; 3] = [
    Action {
        label: "Edit host",
        hotkey: "[h]",
    },
    Action {
        label: "Edit project",
        hotkey: "[p]",
    },
    Action {
        label: "Templates file",
        hotkey: "[t]",
    },
];

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
        List::new(actions).block(section_block("Actions", "[/] palette")),
        sections[1],
    );

    let footer = Paragraph::new(footer_lines(app))
        .wrap(Wrap { trim: true })
        .block(section_block("Status", ""));
    frame.render_widget(footer, sections[2]);
}

fn footer_lines(app: &App) -> Vec<Line<'static>> {
    match app.active_resource() {
        ResourceKind::Applications => vec![
            status_heading("Applications"),
            muted_line("Create OIDC apps from templates or by hand. Confidential apps expose secret rotation."),
        ],
        ResourceKind::Users => vec![
            status_heading("Users"),
            muted_line("Human users, admin bootstrap, and IAM_OWNER grants now share one shell."),
        ],
        ResourceKind::Idps => vec![
            status_heading("Identity providers"),
            muted_line("Google IDP is manual-entry only in Rust. Kubernetes secret loading stays removed."),
        ],
        ResourceKind::Auth => vec![
            status_heading("Authentication"),
            muted_line("Precedence is CLI, env, config, then setup. Headless commands fail fast instead of prompting."),
        ],
        ResourceKind::Config => vec![
            status_heading("Configuration"),
            muted_line("Canonical config lives in TOML. Legacy Ruby YAML is only for first-run import."),
        ],
    }
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

    let form_block = Block::default()
        .borders(Borders::ALL)
        .border_style(focus_border(app.focus == Focus::Form))
        .style(Style::default().bg(Color::Rgb(18, 26, 39)))
        .title(title_span("Canvas", canvas_title(app)));
    frame.render_widget(form_block, chunks[0]);

    let form = Paragraph::new(canvas_lines(app))
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
        .title(title_span("Review", review_title(app)));
    frame.render_widget(review_block, chunks[1]);

    let review = Paragraph::new(review_lines(app)).wrap(Wrap { trim: true });
    frame.render_widget(
        review,
        chunks[1].inner(Margin {
            horizontal: 2,
            vertical: 1,
        }),
    );
}

fn canvas_title(app: &App) -> &'static str {
    match app.active_resource() {
        ResourceKind::Applications => "Create OIDC application",
        ResourceKind::Users => "Create or elevate user",
        ResourceKind::Idps => "Configure Google provider",
        ResourceKind::Auth => "Validate runtime credentials",
        ResourceKind::Config => "First-run and saved settings",
    }
}

fn canvas_lines(app: &App) -> Vec<Line<'static>> {
    match app.active_resource() {
        ResourceKind::Applications => vec![
            field_line("Mode", "template or manual"),
            field_line(
                "Client type",
                &selected_field(app, "confidential", "public"),
            ),
            field_line(
                "Redirect URIs",
                &selected_meta(app, "configure before create"),
            ),
            field_line("Template file", app.templates_path.as_deref().unwrap_or("not configured")),
            muted_line("Use quick setup to batch-create predefined apps from the templates YAML."),
        ],
        ResourceKind::Users => vec![
            field_line("Mode", "human or imported admin"),
            field_line("Email", &selected_field(app, "pending", "choose a user")),
            field_line("Privilege", &selected_meta(app, "IAM_OWNER optional")),
            field_line("Bootstrap", "temporary password shown once"),
            muted_line("Admin creation keeps the Ruby flow: import local user, then optionally grant IAM_OWNER."),
        ],
        ResourceKind::Idps => vec![
            field_line("Provider", "Google"),
            field_line("Credential source", "manual token entry"),
            field_line("Creation", "allowed"),
            field_line("Linking", "allowed"),
            muted_line("The Rust port intentionally drops Kubernetes secret reads for Google OAuth credentials."),
        ],
        ResourceKind::Auth => vec![
            field_line("Current source", &app.auth_label),
            field_line(
                "Status",
                if app.setup_required {
                    "setup required"
                } else {
                    "validated by startup bootstrap"
                },
            ),
            field_line("Priority", "CLI > env > config > setup"),
            field_line("OAuth device", "planned next"),
            muted_line("Interactive setup should recover stale PATs and invalid service-account files here."),
        ],
        ResourceKind::Config => vec![
            field_line("Host", &app.host),
            field_line("Project", &app.project),
            field_line(
                "Templates",
                app.templates_path.as_deref().unwrap_or("not configured"),
            ),
            field_line(
                "Legacy import",
                if app.setup_required {
                    "available"
                } else {
                    "completed or skipped"
                },
            ),
            muted_line("Config remains TOML in XDG space, with Ruby YAML import only as a migration bridge."),
        ],
    }
}

fn review_title(app: &App) -> &'static str {
    match app.active_resource() {
        ResourceKind::Applications => "selected application",
        ResourceKind::Users => "selected user",
        ResourceKind::Idps => "selected provider",
        ResourceKind::Auth => "resolution detail",
        ResourceKind::Config => "stored values",
    }
}

fn review_lines(app: &App) -> Vec<Line<'static>> {
    match app.active_resource() {
        ResourceKind::Applications | ResourceKind::Users | ResourceKind::Idps => {
            if let Some(record) = app.selected_record() {
                vec![
                    bold_line(&record.name),
                    subtle_line(&record.kind),
                    Line::from(""),
                    field_line("Summary", &record.redirects),
                    field_line("Changed", &record.changed_at),
                    field_line("Inspector", "press i for full detail"),
                ]
            } else {
                vec![
                    bold_line("Nothing selected"),
                    muted_line("No records are loaded for this resource yet."),
                ]
            }
        }
        ResourceKind::Auth => vec![
            bold_line("Resolved at startup"),
            subtle_line(&app.auth_label),
            Line::from(""),
            field_line("Ready", if app.setup_required { "no" } else { "yes" }),
            field_line(
                "Fallback",
                if app.setup_required {
                    "launch setup flow"
                } else {
                    "revalidate token"
                },
            ),
        ],
        ResourceKind::Config => vec![
            bold_line("Configuration scope"),
            subtle_line("runtime + templates"),
            Line::from(""),
            field_line("Host", &app.host),
            field_line("Project", &app.project),
            field_line(
                "Templates",
                app.templates_path.as_deref().unwrap_or("not configured"),
            ),
        ],
    }
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

fn selection_title(app: &App) -> &'static str {
    match app.active_resource() {
        ResourceKind::Applications => "existing applications",
        ResourceKind::Users => "existing users",
        ResourceKind::Idps => "configured identity providers",
        ResourceKind::Auth => "auth overview",
        ResourceKind::Config => "saved settings",
    }
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
        "{}. [Tab] focus  [h/l] resource  [j/k] move  [i] inspector  [q] quit  Selected: {}",
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

    let lines = match app.active_resource() {
        ResourceKind::Applications | ResourceKind::Users | ResourceKind::Idps => {
            if let Some(record) = app.selected_record() {
                vec![
                    field_line("Name", &record.name),
                    field_line("State", &record.kind),
                    field_line("Summary", &record.redirects),
                    field_line("Changed", &record.changed_at),
                    Line::from(""),
                    muted_line("Inspector is off-canvas so the workspace can stay calmer while still exposing detail on demand."),
                ]
            } else {
                vec![
                    bold_line("Nothing selected"),
                    muted_line("Choose a record first, then open the inspector again."),
                ]
            }
        }
        ResourceKind::Auth => vec![
            field_line("Source", &app.auth_label),
            field_line(
                "Requires setup",
                if app.setup_required { "yes" } else { "no" },
            ),
            field_line("Priority", "CLI > env > config > setup"),
            Line::from(""),
            muted_line("This popup is where validation errors and re-auth actions should land next."),
        ],
        ResourceKind::Config => vec![
            field_line("Host", &app.host),
            field_line("Project", &app.project),
            field_line(
                "Templates",
                app.templates_path.as_deref().unwrap_or("not configured"),
            ),
            Line::from(""),
            muted_line("Keep runtime config and templates separate so secrets do not drift into domain templates."),
        ],
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

fn resource_label(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Applications => "Applications",
        ResourceKind::Users => "Users",
        ResourceKind::Idps => "IDPs",
        ResourceKind::Auth => "Auth",
        ResourceKind::Config => "Config",
    }
}

fn status_mark(app: &App) -> &'static str {
    if app.setup_required {
        "!"
    } else {
        "✓"
    }
}

fn selected_field(app: &App, default: &str, empty: &str) -> String {
    app.selected_record()
        .map(|record| record.name.clone())
        .unwrap_or_else(|| {
            if app.active_records().is_empty() {
                empty.to_string()
            } else {
                default.to_string()
            }
        })
}

fn selected_meta(app: &App, empty: &str) -> String {
    app.selected_record()
        .map(|record| record.redirects.clone())
        .unwrap_or_else(|| empty.to_string())
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

pub fn sample_app_records() -> Vec<Record> {
    vec![
        Record {
            name: "grafana-main".to_string(),
            kind: "confidential".to_string(),
            redirects: "2 configured".to_string(),
            changed_at: "09:41 UTC".to_string(),
        },
        Record {
            name: "paperless".to_string(),
            kind: "confidential".to_string(),
            redirects: "3 configured".to_string(),
            changed_at: "09:12 UTC".to_string(),
        },
        Record {
            name: "mealie-web".to_string(),
            kind: "public".to_string(),
            redirects: "2 configured".to_string(),
            changed_at: "08:54 UTC".to_string(),
        },
        Record {
            name: "nextcloud".to_string(),
            kind: "inactive".to_string(),
            redirects: "4 configured".to_string(),
            changed_at: "yesterday".to_string(),
        },
    ]
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
            user_records: vec![Record {
                name: "alice@example.com".to_string(),
                kind: "active".to_string(),
                redirects: "IAM_OWNER".to_string(),
                changed_at: "today".to_string(),
            }],
            idp_records: vec![Record {
                name: "Google".to_string(),
                kind: "active".to_string(),
                redirects: "openid profile email".to_string(),
                changed_at: "today".to_string(),
            }],
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
    fn focus_cycles_backward() {
        let mut app = test_app();
        app.reverse_focus();
        assert_eq!(app.focus, Focus::Records);
        app.reverse_focus();
        assert_eq!(app.focus, Focus::Form);
    }

    #[test]
    fn toggles_inspector_popup() {
        let mut app = test_app();
        assert!(!app.show_inspector);
        app.toggle_inspector();
        assert!(app.show_inspector);
        app.toggle_inspector();
        assert!(!app.show_inspector);
    }

    #[test]
    fn selection_navigation_wraps() {
        let mut app = test_app();
        app.previous_record();
        assert_eq!(app.selected_record().unwrap().name, "nextcloud");
        app.next_record();
        assert_eq!(app.selected_record().unwrap().name, "grafana-main");
    }

    #[test]
    fn resource_navigation_changes_workspace() {
        let mut app = test_app();
        assert_eq!(app.active_resource(), ResourceKind::Applications);
        app.next_resource();
        assert_eq!(app.active_resource(), ResourceKind::Users);
        assert_eq!(app.selected_record().unwrap().name, "alice@example.com");
        app.previous_resource();
        assert_eq!(app.active_resource(), ResourceKind::Applications);
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
        assert_eq!(app.selected_action, 0);
        assert_eq!(app.actions()[app.selected_action].label, "Create user");
    }
}
