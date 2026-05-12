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
        FieldKind::Secret => {
            let masked = "•".repeat(field.value.len().max(1));
            if selected {
                let cursor = field.cursor.min(field.value.len());
                insert_cursor(&masked, cursor)
            } else {
                masked
            }
        }
        FieldKind::Toggle | FieldKind::Checkbox => {
            if is_enabled(&field.value) {
                "[x]".to_string()
            } else {
                "[ ]".to_string()
            }
        }
        FieldKind::Choice(_) | FieldKind::Text => {
            if selected && !field.value.is_empty() {
                let cursor = field.cursor.min(field.value.len());
                insert_cursor(&field.value, cursor)
            } else {
                field.value.clone()
            }
        }
    };
    format!("{marker} {:<18} {}", field.label, value)
}

fn insert_cursor(value: &str, cursor: usize) -> String {
    let char_count = value.chars().count();
    let cursor = cursor.min(char_count);
    let before: String = value.chars().take(cursor).collect();
    let after: String = value.chars().skip(cursor).collect();
    format!("{before}▏{after}")
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
