use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::{
    copy::{footer_lines, selection_title},
    types::{App, Focus},
    widgets::{
        bold_line, centered_rect, field_line, focus_border, list_item, muted_line, section_block,
        title_span,
    },
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
            format!("{} {}", app.auth_label, super::copy::status_mark(app)),
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
        super::copy::resource_label(app.active_resource()),
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
