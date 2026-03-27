use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::{
    App, ConfirmState, FieldKind, Focus, FormField, FormState, MessageState, PendingAction,
    ResourceKind, TuiBootstrap,
};

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

pub(super) fn browse_title(app: &App) -> &'static str {
    match app.active_resource() {
        ResourceKind::Applications => "OIDC application workspace",
        ResourceKind::Users => "User management workspace",
        ResourceKind::Idps => "Identity provider workspace",
        ResourceKind::Auth => "Authentication setup",
        ResourceKind::Config => "Configuration editor",
    }
}

pub(super) fn browse_meta(app: &App) -> &'static str {
    match app.active_resource() {
        ResourceKind::Applications => "choose an action",
        ResourceKind::Users => "choose an action",
        ResourceKind::Idps => "choose an action",
        ResourceKind::Auth => "run setup",
        ResourceKind::Config => "edit or import",
    }
}

pub(super) fn browse_lines(app: &App) -> Vec<String> {
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

pub(super) fn browse_review_lines(app: &App) -> Vec<Line<'static>> {
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

pub(super) fn form_review_lines(form: &FormState) -> Vec<Line<'static>> {
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

pub(super) fn confirm_review_lines(confirm: &ConfirmState) -> Vec<Line<'static>> {
    vec![
        bold_line(&confirm.title),
        subtle_line(&confirm.submit_label),
        Line::from(""),
        field_line("Pending", pending_label(&confirm.pending)),
        muted_line("Press Enter to confirm or Esc to cancel."),
    ]
}

pub(super) fn message_review_lines(message: &MessageState) -> Vec<Line<'static>> {
    vec![
        bold_line(&message.title),
        subtle_line("workflow result"),
        Line::from(""),
        field_line("Lines", &message.lines.len().to_string()),
        muted_line("Press Enter or Esc to return to the workspace."),
    ]
}

pub(super) fn render_form_line(field: &FormField, selected: bool) -> String {
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

pub(super) fn pending_label(pending: &PendingAction) -> &'static str {
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

pub(super) fn default_setup_form(bootstrap: &TuiBootstrap) -> FormState {
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

pub(super) fn selection_title(app: &App) -> &'static str {
    match app.active_resource() {
        ResourceKind::Applications => "existing applications",
        ResourceKind::Users => "existing users",
        ResourceKind::Idps => "configured identity providers",
        ResourceKind::Auth => "setup state",
        ResourceKind::Config => "saved values",
    }
}

pub(super) fn resource_label(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Applications => "Applications",
        ResourceKind::Users => "Users",
        ResourceKind::Idps => "IDPs",
        ResourceKind::Auth => "Auth",
        ResourceKind::Config => "Config",
    }
}

pub(super) fn focus_label(focus: Focus) -> &'static str {
    match focus {
        Focus::Resources => "resources",
        Focus::Actions => "actions",
        Focus::Form => "form",
        Focus::Records => "records",
    }
}

pub(super) fn status_mark(app: &App) -> &'static str {
    if app.setup_required {
        "!"
    } else {
        "✓"
    }
}

pub(super) fn toggle_field(field: &mut FormField) {
    field.value = if is_enabled(&field.value) {
        "false".to_string()
    } else {
        "true".to_string()
    };
}

pub(super) fn cycle_choice(field: &mut FormField, options: &[String], forward: bool) {
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

pub(super) fn is_enabled(value: &str) -> bool {
    matches!(value, "true" | "yes" | "on" | "1")
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
