use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, ListItem},
};

use super::{
    state::is_enabled,
    types::{FieldKind, FormField},
};

pub(crate) fn render_form_line(field: &FormField, selected: bool) -> String {
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

pub(crate) fn list_item(
    label: &str,
    meta: &str,
    selected: bool,
    accent: Color,
) -> ListItem<'static> {
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

pub(crate) fn section_block<'a>(title: &'a str, meta: &'a str) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(47, 58, 79)))
        .style(Style::default().bg(Color::Rgb(18, 26, 39)))
        .title(title_span(title, meta))
}

pub(crate) fn title_span<'a>(title: &'a str, meta: &'a str) -> Line<'a> {
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

pub(crate) fn field_line(label: &str, value: &str) -> Line<'static> {
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

pub(crate) fn bold_line(value: &str) -> Line<'static> {
    Line::from(Span::styled(
        value.to_string(),
        Style::default()
            .fg(Color::Rgb(237, 243, 255))
            .add_modifier(Modifier::BOLD),
    ))
}

pub(crate) fn subtle_line(value: &str) -> Line<'static> {
    Line::from(Span::styled(
        value.to_string(),
        Style::default().fg(Color::Rgb(176, 188, 213)),
    ))
}

pub(crate) fn status_heading(value: &str) -> Line<'static> {
    Line::from(Span::styled(
        value.to_string(),
        Style::default()
            .fg(Color::Rgb(176, 188, 213))
            .add_modifier(Modifier::BOLD),
    ))
}

pub(crate) fn muted_line(value: &str) -> Line<'static> {
    Line::from(Span::styled(
        value.to_string(),
        Style::default().fg(Color::Rgb(121, 131, 151)),
    ))
}

pub(crate) fn focus_border(selected: bool) -> Style {
    if selected {
        Style::default().fg(Color::Rgb(145, 194, 255))
    } else {
        Style::default().fg(Color::Rgb(47, 58, 79))
    }
}

pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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
