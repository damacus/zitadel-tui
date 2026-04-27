use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::{
    copy::{
        browse_lines, browse_meta, browse_review_lines, browse_title, confirm_review_lines,
        focus_label, footer_lines, form_review_lines, message_review_lines, resource_label,
    },
    widgets::{bold_line, field_line, muted_line, render_form_line, status_heading},
    App, CanvasMode, Focus,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShellLayout {
    Narrow,
    Medium,
    Wide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CanvasTone {
    Browse,
    Form,
    Setup,
    Confirm,
    Success,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PanelTone {
    Info,
    Warm,
    Success,
    Warning,
    Error,
    Muted,
}

#[derive(Debug)]
pub(super) struct CanvasContent {
    title: String,
    meta: String,
    lines: Vec<String>,
    review_lines: Vec<Line<'static>>,
    tone: CanvasTone,
}

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let layout_mode = shell_layout(area);
    let outer = Block::default().style(Style::default().bg(shell_bg()).fg(foreground_color()));
    frame.render_widget(outer, area);

    let header_height = shell_header_height(area, layout_mode);
    let status_height = shell_status_height(area, layout_mode);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Min(8),
            Constraint::Length(status_height),
        ])
        .split(area);

    draw_header(frame, layout[0], app, layout_mode);
    draw_body(frame, layout[1], app, layout_mode);
    draw_status(frame, layout[2], app, layout_mode);

    if app.show_inspector {
        draw_inspector_popup(frame, centered_rect(layout_mode, area), app);
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App, layout_mode: ShellLayout) {
    let header = Block::default().style(Style::default().bg(header_bg()));
    frame.render_widget(header, area);

    let inner = inner_with_margin(area, 2, 1);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "ZITADEL TUI",
            Style::default()
                .fg(info_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("Command Atelier", Style::default().fg(warm_color())),
    ]));

    let auth = Paragraph::new(auth_summary_line(app)).alignment(Alignment::Right);
    let project = Paragraph::new(project_summary_line(app));
    let host = Paragraph::new(host_summary_line(app))
        .style(Style::default().fg(muted_color()))
        .alignment(Alignment::Left);
    let inspector = Paragraph::new(shortcut_line("[i]", "inspector")).alignment(Alignment::Right);

    match layout_mode {
        ShellLayout::Wide => {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Length(1)])
                .split(inner);
            let top = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(24), Constraint::Length(30)])
                .split(rows[0]);
            let bottom = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(45),
                    Constraint::Percentage(40),
                    Constraint::Length(16),
                ])
                .split(rows[1]);

            frame.render_widget(title, top[0]);
            frame.render_widget(auth, top[1]);
            frame.render_widget(project, bottom[0]);
            frame.render_widget(host, bottom[1]);
            frame.render_widget(inspector, bottom[2]);
        }
        ShellLayout::Medium => {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Length(1)])
                .split(inner);
            let top = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(18), Constraint::Length(26)])
                .split(rows[0]);
            let bottom = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Percentage(45),
                    Constraint::Length(16),
                ])
                .split(rows[1]);

            frame.render_widget(title, top[0]);
            frame.render_widget(auth, top[1]);
            frame.render_widget(project, bottom[0]);
            frame.render_widget(host, bottom[1]);
            frame.render_widget(inspector, bottom[2]);
        }
        ShellLayout::Narrow => {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(1),
                ])
                .split(inner);
            let middle = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(16), Constraint::Length(18)])
                .split(rows[1]);
            let bottom = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(16), Constraint::Length(14)])
                .split(rows[2]);

            frame.render_widget(title, rows[0]);
            frame.render_widget(project, middle[0]);
            frame.render_widget(auth, middle[1]);
            frame.render_widget(host, bottom[0]);
            frame.render_widget(inspector, bottom[1]);
        }
    }
}

fn draw_body(frame: &mut Frame, area: Rect, app: &App, layout_mode: ShellLayout) {
    let chunks = match layout_mode {
        ShellLayout::Wide => Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(10)])
            .split(area),
        ShellLayout::Medium => Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(26), Constraint::Min(10)])
            .split(area),
        ShellLayout::Narrow => Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(19), Constraint::Min(10)])
            .split(area),
    };

    draw_command_rail(frame, chunks[0], app, layout_mode);
    draw_workspace(frame, chunks[1], app, layout_mode);
}

fn draw_command_rail(frame: &mut Frame, area: Rect, app: &App, layout_mode: ShellLayout) {
    let rail = Block::default()
        .borders(match layout_mode {
            ShellLayout::Narrow => Borders::BOTTOM,
            ShellLayout::Medium | ShellLayout::Wide => Borders::RIGHT,
        })
        .border_style(Style::default().fg(border_color(false, PanelTone::Muted)))
        .style(Style::default().bg(rail_bg()));
    frame.render_widget(rail, area);

    let inner = inner_with_margin(area, 2, 1);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints(match layout_mode {
            ShellLayout::Wide => [
                Constraint::Length(9),
                Constraint::Length(8),
                Constraint::Min(4),
            ],
            ShellLayout::Medium => [
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Min(4),
            ],
            ShellLayout::Narrow => [
                Constraint::Length(7),
                Constraint::Length(7),
                Constraint::Min(3),
            ],
        })
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
        List::new(resources).block(panel_block(
            "Resources",
            "[h/l] cycle",
            PanelTone::Info,
            app.focus == Focus::Resources,
        )),
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
                warm_color(),
            )
        })
        .collect();
    frame.render_widget(
        List::new(actions).block(panel_block(
            "Actions",
            "[j/k] choose  [Enter] run",
            PanelTone::Warm,
            app.focus == Focus::Actions,
        )),
        sections[1],
    );

    let footer = Paragraph::new(guide_lines(app, layout_mode))
        .wrap(Wrap { trim: true })
        .block(panel_block(
            "Guide",
            focused_hint(app),
            PanelTone::Muted,
            false,
        ));
    frame.render_widget(footer, sections[2]);
}

fn draw_workspace(frame: &mut Frame, area: Rect, app: &App, layout_mode: ShellLayout) {
    let inner = inner_with_margin(area, 2, 1);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints(match layout_mode {
            ShellLayout::Wide => [Constraint::Min(10), Constraint::Length(9)],
            ShellLayout::Medium => [Constraint::Min(11), Constraint::Length(8)],
            ShellLayout::Narrow => [Constraint::Min(10), Constraint::Length(8)],
        })
        .split(inner);
    draw_canvas(frame, sections[0], app, layout_mode);
    draw_selection_tray(frame, sections[1], app);
}

fn draw_canvas(frame: &mut Frame, area: Rect, app: &App, layout_mode: ShellLayout) {
    let chunks = match layout_mode {
        ShellLayout::Wide => Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area),
        ShellLayout::Medium => Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(area),
        ShellLayout::Narrow => Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(area),
    };
    let canvas = canvas_content(app);

    let form_block = panel_block(
        &canvas.title,
        &canvas.meta,
        panel_tone(canvas.tone),
        app.focus == Focus::Form,
    );
    frame.render_widget(form_block, chunks[0]);

    let form = Paragraph::new(canvas_body_lines(&canvas))
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);
    frame.render_widget(form, inner_with_margin(chunks[0], 2, 1));

    let review_block = panel_block("Review", review_meta(canvas.tone), PanelTone::Muted, false);
    frame.render_widget(review_block, chunks[1]);

    let review = Paragraph::new(canvas.review_lines).wrap(Wrap { trim: true });
    frame.render_widget(review, inner_with_margin(chunks[1], 2, 1));
}

fn draw_selection_tray(frame: &mut Frame, area: Rect, app: &App) {
    let tray = panel_block(
        "Selection tray",
        selection_meta(app),
        PanelTone::Info,
        app.focus == Focus::Records,
    );
    frame.render_widget(tray, area);

    if app.active_records().is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(Span::styled(
                "No records loaded yet.",
                Style::default()
                    .fg(subtle_color())
                    .add_modifier(Modifier::BOLD),
            )),
            muted_line("This tray updates after refreshes and completed workflows."),
            muted_line("Use the resource rail to switch context or run an action."),
        ]);
        frame.render_widget(empty, inner_with_margin(area, 2, 1));
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
    frame.render_widget(List::new(rows), inner_with_margin(area, 2, 1));
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App, layout_mode: ShellLayout) {
    let status = Block::default()
        .style(Style::default().bg(status_bg()))
        .borders(Borders::TOP)
        .border_style(Style::default().fg(border_color(false, PanelTone::Muted)));
    frame.render_widget(status, area);

    frame.render_widget(
        Paragraph::new(status_lines(app, layout_mode))
            .style(Style::default().fg(muted_color()))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left),
        inner_with_margin(area, 2, 0),
    );
}

fn draw_inspector_popup(frame: &mut Frame, area: Rect, app: &App) {
    frame.render_widget(Clear, area);
    let block = panel_block("Inspector", "[i] close", PanelTone::Info, true)
        .style(Style::default().bg(popup_bg()));
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
    frame.render_widget(text, inner_with_margin(area, 2, 1));
}

fn shell_layout(area: Rect) -> ShellLayout {
    if area.width < 96 {
        ShellLayout::Narrow
    } else if area.width < 140 {
        ShellLayout::Medium
    } else {
        ShellLayout::Wide
    }
}

fn shell_header_height(area: Rect, layout_mode: ShellLayout) -> u16 {
    if area.height < 12 {
        3
    } else {
        match layout_mode {
            ShellLayout::Narrow => 5,
            ShellLayout::Medium | ShellLayout::Wide => 4,
        }
    }
}

fn shell_status_height(area: Rect, layout_mode: ShellLayout) -> u16 {
    if area.height < 10 {
        1
    } else {
        match layout_mode {
            ShellLayout::Narrow => 3,
            ShellLayout::Medium | ShellLayout::Wide => 2,
        }
    }
}

pub(super) fn canvas_content(app: &App) -> CanvasContent {
    match &app.canvas_mode {
        CanvasMode::Browse => CanvasContent {
            title: browse_title(app).to_string(),
            meta: browse_meta(app).to_string(),
            lines: browse_lines(app),
            review_lines: browse_review_lines(app),
            tone: CanvasTone::Browse,
        },
        CanvasMode::EditForm(form) => CanvasContent {
            title: form.title.clone(),
            meta: form.submit_label.clone(),
            lines: form
                .fields
                .iter()
                .enumerate()
                .map(|(index, field)| render_form_line(field, index == form.selected_field))
                .collect(),
            review_lines: form_review_lines(form),
            tone: CanvasTone::Form,
        },
        CanvasMode::Setup(form) => CanvasContent {
            title: form.title.clone(),
            meta: form.submit_label.clone(),
            lines: form
                .fields
                .iter()
                .enumerate()
                .map(|(index, field)| render_form_line(field, index == form.selected_field))
                .collect(),
            review_lines: form_review_lines(form),
            tone: CanvasTone::Setup,
        },
        CanvasMode::Confirm(confirm) => CanvasContent {
            title: confirm.title.clone(),
            meta: confirm.submit_label.clone(),
            lines: confirm.lines.clone(),
            review_lines: confirm_review_lines(confirm),
            tone: CanvasTone::Confirm,
        },
        CanvasMode::Success(message) => CanvasContent {
            title: message.title.clone(),
            meta: "[Enter] continue".to_string(),
            lines: message.lines.clone(),
            review_lines: message_review_lines(message),
            tone: CanvasTone::Success,
        },
        CanvasMode::Error(message) => CanvasContent {
            title: message.title.clone(),
            meta: "[Esc] back".to_string(),
            lines: message.lines.clone(),
            review_lines: message_review_lines(message),
            tone: CanvasTone::Error,
        },
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn canvas_title(app: &App) -> String {
    canvas_content(app).title
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn canvas_lines(app: &App) -> Vec<String> {
    canvas_content(app).lines
}

fn canvas_body_lines(canvas: &CanvasContent) -> Vec<Line<'static>> {
    let mut lines = match canvas.tone {
        CanvasTone::Browse => Vec::new(),
        CanvasTone::Form => vec![highlight_line(
            "● Edit fields and submit when ready.",
            info_color(),
        )],
        CanvasTone::Setup => vec![highlight_line(
            "⚠ Setup is required before the main workspace can run actions.",
            warning_color(),
        )],
        CanvasTone::Confirm => vec![highlight_line(
            "● Confirm the pending action or cancel to return safely.",
            warm_color(),
        )],
        CanvasTone::Success => vec![highlight_line("✓ Workflow completed.", success_color())],
        CanvasTone::Error => vec![highlight_line(
            "⚠ Workflow needs attention before continuing.",
            error_color(),
        )],
    };

    if !lines.is_empty() {
        lines.push(Line::from(""));
    }

    lines.extend(canvas.lines.iter().enumerate().map(|(index, line)| {
        let style = if index == 0 && matches!(canvas.tone, CanvasTone::Browse) {
            Style::default()
                .fg(subtle_color())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(foreground_color())
        };
        Line::from(Span::styled(line.clone(), style))
    }));
    lines
}

fn review_meta(tone: CanvasTone) -> &'static str {
    match tone {
        CanvasTone::Browse => "selection context",
        CanvasTone::Form | CanvasTone::Setup => "field summary",
        CanvasTone::Confirm => "pending action",
        CanvasTone::Success | CanvasTone::Error => "workflow result",
    }
}

fn selection_meta(app: &App) -> &'static str {
    if app.active_records().is_empty() {
        "waiting for records"
    } else {
        "[j/k] move  [i] inspect"
    }
}

fn guide_lines(app: &App, layout_mode: ShellLayout) -> Vec<Line<'static>> {
    let mut lines = vec![status_heading(match app.focus {
        Focus::Resources => "Resource rail",
        Focus::Actions => "Action rail",
        Focus::Form => "Canvas",
        Focus::Records => "Selection tray",
    })];

    match app.focus {
        Focus::Resources => lines.push(shortcut_line("[h/l]", "change resource")),
        Focus::Actions => {
            lines.push(shortcut_line("[j/k]", "choose action"));
            lines.push(shortcut_line("[Enter]", "run selected action"));
        }
        Focus::Form => match &app.canvas_mode {
            CanvasMode::Browse => lines.push(shortcut_line("[Enter]", "begin selected action")),
            CanvasMode::EditForm(_) | CanvasMode::Setup(_) => {
                lines.push(shortcut_line("[j/k]", "move fields"));
                lines.push(shortcut_line("[Space]", "toggle or cycle choices"));
            }
            CanvasMode::Confirm(_) => lines.push(shortcut_line("[Enter]", "confirm action")),
            CanvasMode::Success(_) | CanvasMode::Error(_) => {
                lines.push(shortcut_line("[Enter]", "return to workspace"))
            }
        },
        Focus::Records => {
            lines.push(shortcut_line("[j/k]", "move record"));
            lines.push(shortcut_line("[i]", "inspect selected record"));
        }
    }

    if matches!(layout_mode, ShellLayout::Wide) {
        lines.push(Line::from(""));
        lines.extend(footer_lines(app));
    } else if let Some(summary) = footer_lines(app).into_iter().nth(1) {
        lines.push(Line::from(""));
        lines.push(summary);
    }

    lines
}

fn focused_hint(app: &App) -> &'static str {
    match app.focus {
        Focus::Resources => "resource focus",
        Focus::Actions => "action focus",
        Focus::Form => "canvas focus",
        Focus::Records => "record focus",
    }
}

fn status_lines(app: &App, layout_mode: ShellLayout) -> Vec<Line<'static>> {
    let selected = app
        .selected_record()
        .map(|record| record.name.as_str())
        .unwrap_or("none");

    let summary = Line::from(vec![
        Span::styled(
            resource_label(app.active_resource()).to_string(),
            Style::default()
                .fg(info_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  •  ", Style::default().fg(muted_color())),
        Span::styled(
            format!("focus {}", focus_label(app.focus)),
            Style::default().fg(subtle_color()),
        ),
        Span::styled("  •  ", Style::default().fg(muted_color())),
        Span::styled(
            format!("selected {selected}"),
            Style::default().fg(foreground_color()),
        ),
    ]);

    let primary_help = match layout_mode {
        ShellLayout::Wide | ShellLayout::Medium => Line::from(vec![
            shortcut_span("[Tab]"),
            muted_span(" focus  "),
            shortcut_span("[Esc]"),
            muted_span(" back  "),
            shortcut_span("[q]"),
            muted_span(" quit  "),
            shortcut_span("[i]"),
            muted_span(" inspector"),
        ]),
        ShellLayout::Narrow => Line::from(vec![
            shortcut_span("[Tab]"),
            muted_span(" focus  "),
            shortcut_span("[Enter]"),
            muted_span(" act  "),
            shortcut_span("[Esc]"),
            muted_span(" back"),
        ]),
    };

    if matches!(layout_mode, ShellLayout::Narrow) {
        vec![
            summary,
            primary_help,
            Line::from(vec![
                shortcut_span("[j/k]"),
                muted_span(" move  "),
                shortcut_span("[h/l]"),
                muted_span(" resource"),
            ]),
        ]
    } else {
        vec![summary, primary_help]
    }
}

fn auth_summary_line(app: &App) -> Line<'static> {
    let (status_text, color) = if app.setup_required {
        ("⚠ setup required", warning_color())
    } else {
        ("✓ ready", success_color())
    };

    Line::from(vec![
        Span::styled(
            app.auth_label.clone(),
            Style::default()
                .fg(subtle_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            status_text.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn project_summary_line(app: &App) -> Line<'static> {
    let project = if app.project.is_empty() {
        "auto".to_string()
    } else {
        app.project.clone()
    };
    Line::from(vec![
        Span::styled("project", Style::default().fg(muted_color())),
        Span::raw(" "),
        Span::styled(project, Style::default().fg(subtle_color())),
    ])
}

fn host_summary_line(app: &App) -> Line<'static> {
    Line::from(vec![
        Span::styled("host", Style::default().fg(muted_color())),
        Span::raw(" "),
        Span::styled(app.host.clone(), Style::default().fg(subtle_color())),
    ])
}

fn shortcut_line(key: &str, label: &str) -> Line<'static> {
    Line::from(vec![shortcut_span(key), muted_span(format!(" {label}"))])
}

fn shortcut_span(value: &str) -> Span<'static> {
    Span::styled(
        value.to_string(),
        Style::default()
            .fg(info_color())
            .add_modifier(Modifier::BOLD),
    )
}

fn muted_span(value: impl Into<String>) -> Span<'static> {
    Span::styled(value.into(), Style::default().fg(muted_color()))
}

fn highlight_line(value: &str, color: Color) -> Line<'static> {
    Line::from(Span::styled(
        value.to_string(),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ))
}

fn panel_tone(tone: CanvasTone) -> PanelTone {
    match tone {
        CanvasTone::Browse => PanelTone::Info,
        CanvasTone::Form => PanelTone::Warm,
        CanvasTone::Setup => PanelTone::Warning,
        CanvasTone::Confirm => PanelTone::Warm,
        CanvasTone::Success => PanelTone::Success,
        CanvasTone::Error => PanelTone::Error,
    }
}

fn list_item(label: &str, meta: &str, selected: bool, accent: Color) -> ListItem<'static> {
    let style = if selected {
        Style::default()
            .bg(accent_surface(accent))
            .fg(foreground_color())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(210, 219, 239))
    };
    let mut spans = vec![Span::styled(label.to_string(), style)];
    if !meta.is_empty() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(meta.to_string(), Style::default().fg(accent)));
    }
    ListItem::new(Line::from(spans))
}

fn panel_block<'a>(title: &'a str, meta: &'a str, tone: PanelTone, focused: bool) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color(focused, tone)))
        .style(Style::default().bg(panel_bg(tone)))
        .title(title_span(title, meta, tone, focused))
}

fn title_span<'a>(title: &'a str, meta: &'a str, tone: PanelTone, focused: bool) -> Line<'a> {
    let mut spans = vec![Span::styled(
        title,
        Style::default()
            .fg(title_color(tone, focused))
            .add_modifier(Modifier::BOLD),
    )];
    if !meta.is_empty() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(meta, Style::default().fg(muted_color())));
    }
    Line::from(spans)
}

fn centered_rect(layout_mode: ShellLayout, area: Rect) -> Rect {
    let preferred_width = match layout_mode {
        ShellLayout::Wide => 76,
        ShellLayout::Medium => 64,
        ShellLayout::Narrow => 48,
    };
    let preferred_height = match layout_mode {
        ShellLayout::Wide => 18,
        ShellLayout::Medium => 16,
        ShellLayout::Narrow => 12,
    };

    let horizontal_padding = if area.width > 20 { 2 } else { 1 };
    let vertical_padding = if area.height > 10 { 1 } else { 0 };
    let width = clamp_popup_dimension(area.width, preferred_width, 28, horizontal_padding);
    let height = clamp_popup_dimension(area.height, preferred_height, 8, vertical_padding);

    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

fn clamp_popup_dimension(total: u16, preferred: u16, minimum: u16, padding: u16) -> u16 {
    let max_available = total.saturating_sub(padding.saturating_mul(2)).max(1);
    preferred
        .min(max_available)
        .max(total.min(minimum).max(1))
        .min(total)
}

fn inner_with_margin(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    area.inner(Margin {
        horizontal: horizontal.min(area.width / 2),
        vertical: vertical.min(area.height / 2),
    })
}

fn shell_bg() -> Color {
    Color::Rgb(14, 19, 27)
}

fn header_bg() -> Color {
    Color::Rgb(20, 28, 41)
}

fn rail_bg() -> Color {
    Color::Rgb(16, 22, 33)
}

fn status_bg() -> Color {
    Color::Rgb(11, 16, 25)
}

fn popup_bg() -> Color {
    Color::Rgb(9, 13, 20)
}

fn panel_bg(tone: PanelTone) -> Color {
    match tone {
        PanelTone::Info => Color::Rgb(18, 27, 40),
        PanelTone::Warm => Color::Rgb(23, 26, 37),
        PanelTone::Success => Color::Rgb(18, 31, 28),
        PanelTone::Warning => Color::Rgb(35, 30, 18),
        PanelTone::Error => Color::Rgb(39, 21, 24),
        PanelTone::Muted => Color::Rgb(17, 24, 35),
    }
}

fn border_color(focused: bool, tone: PanelTone) -> Color {
    if focused {
        match tone {
            PanelTone::Info => info_color(),
            PanelTone::Warm => warm_color(),
            PanelTone::Success => success_color(),
            PanelTone::Warning => warning_color(),
            PanelTone::Error => error_color(),
            PanelTone::Muted => subtle_color(),
        }
    } else {
        match tone {
            PanelTone::Info => Color::Rgb(53, 72, 97),
            PanelTone::Warm => Color::Rgb(88, 67, 58),
            PanelTone::Success => Color::Rgb(44, 90, 71),
            PanelTone::Warning => Color::Rgb(107, 89, 44),
            PanelTone::Error => Color::Rgb(113, 64, 69),
            PanelTone::Muted => Color::Rgb(45, 56, 73),
        }
    }
}

fn title_color(tone: PanelTone, focused: bool) -> Color {
    if focused {
        border_color(true, tone)
    } else {
        match tone {
            PanelTone::Info => subtle_color(),
            PanelTone::Warm => Color::Rgb(224, 178, 147),
            PanelTone::Success => success_color(),
            PanelTone::Warning => warning_color(),
            PanelTone::Error => error_color(),
            PanelTone::Muted => subtle_color(),
        }
    }
}

fn accent_surface(accent: Color) -> Color {
    match accent {
        Color::Rgb(236, 156, 114) => Color::Rgb(68, 43, 33),
        Color::Rgb(112, 224, 144) => Color::Rgb(26, 58, 44),
        _ => Color::Rgb(31, 48, 76),
    }
}

fn foreground_color() -> Color {
    Color::Rgb(237, 243, 255)
}

fn subtle_color() -> Color {
    Color::Rgb(176, 188, 213)
}

fn muted_color() -> Color {
    Color::Rgb(121, 131, 151)
}

fn info_color() -> Color {
    Color::Rgb(145, 194, 255)
}

fn warm_color() -> Color {
    Color::Rgb(236, 156, 114)
}

fn success_color() -> Color {
    Color::Rgb(112, 224, 144)
}

fn warning_color() -> Color {
    Color::Rgb(245, 194, 66)
}

fn error_color() -> Color {
    Color::Rgb(242, 117, 121)
}
