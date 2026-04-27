use crate::tui::{CanvasMode, MessageState};

pub(crate) fn error_mode(title: &str, message: &str) -> CanvasMode {
    CanvasMode::Error(MessageState {
        title: title.to_string(),
        lines: vec![message.to_string()],
    })
}
