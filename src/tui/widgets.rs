use super::{
    state::is_enabled,
    types::{FieldKind, FormField},
};

pub(crate) fn render_form_line(field: &FormField, selected: bool) -> String {
    let marker = if selected { "›" } else { " " };
    let value = match field.kind {
        FieldKind::Secret => {
            let masked = "•".repeat(field.value.chars().count().max(1));
            if selected {
                let cursor = field.value[..field.cursor.min(field.value.len())]
                    .chars()
                    .count();
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
                let cursor = field.value[..field.cursor.min(field.value.len())]
                    .chars()
                    .count();
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
