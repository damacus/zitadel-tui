use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
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
