use super::{App, AppCommand, CanvasMode, Focus};
use crossterm::event::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Organisation,
    Project,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    Section(usize),
    Back,
    Switch(Scope, String),
    Quit,
    OpenRecord(usize),
}
#[derive(Debug, Clone)]
pub enum Overlay {
    Pick {
        scope: Scope,
        query: String,
        cursor: usize,
    },
    Discard(Destination),
    SubmissionError(super::MessageState, usize),
}

impl App {
    pub(super) fn field_values(mode: &CanvasMode) -> Vec<String> {
        match mode {
            CanvasMode::EditForm(f) | CanvasMode::Setup(f) => {
                f.fields.iter().map(|field| field.value.clone()).collect()
            }
            _ => vec![],
        }
    }
    pub fn dirty(&self) -> bool {
        let draft = match &self.canvas_mode {
            CanvasMode::EditForm(form) | CanvasMode::Setup(form) => Some(form),
            CanvasMode::Confirm(_) => self.review_draft.as_ref(),
            _ => None,
        };
        draft.is_some_and(|form| {
            !form
                .fields
                .iter()
                .map(|field| &field.value)
                .eq(self.initial_values.iter())
        })
    }
    pub fn navigate(&mut self, to: Destination) -> AppCommand {
        if self.dirty() {
            self.overlay = Some(Overlay::Discard(to));
            AppCommand::Noop
        } else {
            self.leave(to)
        }
    }
    fn leave(&mut self, to: Destination) -> AppCommand {
        self.overlay = None;
        match to {
            Destination::Section(i) => {
                self.set_canvas_mode(CanvasMode::Browse);
                self.selected_resource = i;
                self.cursor = 0;
                self.expanded = None;
                self.filter.clear();
                self.filtering = false;
                self.focus = Focus::Records;
            }
            Destination::OpenRecord(i) => {
                self.set_canvas_mode(CanvasMode::Browse);
                self.focus = Focus::Records;
                self.expanded = Some(i);
                self.cursor = self
                    .entries()
                    .iter()
                    .position(|e| *e == super::directory::Entry::Record(i))
                    .unwrap_or(0);
            }
            Destination::Back => {
                self.reset_to_browse();
                self.initial_values = Self::field_values(&self.canvas_mode);
            }
            Destination::Switch(scope, id) => return AppCommand::ChangeScope(scope, id),
            Destination::Quit => return AppCommand::Quit,
        }
        AppCommand::Noop
    }
    pub fn change_scope_choices(&self, scope: Scope) -> &[(String, String)] {
        match scope {
            Scope::Organisation => &self.organizations,
            Scope::Project => &self.projects,
        }
    }
    pub(super) fn selected_scope(&self, scope: Scope) -> Option<&str> {
        match scope {
            Scope::Organisation => self.organization.as_deref(),
            Scope::Project => Some(&self.project),
        }
    }

    pub fn open_picker(&mut self, scope: Scope) -> AppCommand {
        let cursor = self
            .change_scope_choices(scope)
            .iter()
            .position(|(id, _)| Some(id.as_str()) == self.selected_scope(scope))
            .unwrap_or(0);
        self.overlay = Some(Overlay::Pick {
            scope,
            query: String::new(),
            cursor,
        });
        AppCommand::Noop
    }
    pub fn matches_scope(&self, scope: Scope, query: &str) -> Vec<&(String, String)> {
        let query = query.to_lowercase();
        self.change_scope_choices(scope)
            .iter()
            .filter(|(id, name)| name.to_lowercase().contains(&query) || id.contains(&query))
            .collect()
    }
    pub fn choose_scope(&mut self, index: usize) -> AppCommand {
        let Some(Overlay::Pick { scope, query, .. }) = &self.overlay else {
            return AppCommand::Noop;
        };
        let scope = *scope;
        let Some((id, _)) = self.matches_scope(scope, query).get(index).copied() else {
            return AppCommand::Noop;
        };
        let id = id.clone();
        self.overlay = None;
        if (scope == Scope::Organisation && self.organization.as_ref() == Some(&id))
            || (scope == Scope::Project && self.project == id)
        {
            return AppCommand::Noop;
        }
        self.navigate(Destination::Switch(scope, id))
    }
    pub(super) fn overlay_key(&mut self, key: KeyCode) -> AppCommand {
        match self.overlay.clone() {
            Some(Overlay::SubmissionError(_, _)) => match key {
                KeyCode::Enter | KeyCode::Esc => self.overlay = None,
                KeyCode::Down | KeyCode::Up => {
                    if let Some(Overlay::SubmissionError(_, offset)) = &mut self.overlay {
                        *offset = if key == KeyCode::Down {
                            offset.saturating_add(1)
                        } else {
                            offset.saturating_sub(1)
                        };
                    }
                }
                _ => {}
            },
            Some(Overlay::Discard(to)) => match key {
                KeyCode::Char('d' | 'D') => return self.leave(to),
                KeyCode::Esc | KeyCode::Enter => self.overlay = None,
                _ => {}
            },
            Some(Overlay::Pick {
                scope,
                mut query,
                mut cursor,
            }) => {
                let len = self.matches_scope(scope, &query).len();
                match key {
                    KeyCode::Esc => {
                        self.overlay = None;
                        return AppCommand::Noop;
                    }
                    KeyCode::Enter => return self.choose_scope(cursor),
                    KeyCode::Down => cursor = (cursor + 1).min(len.saturating_sub(1)),
                    KeyCode::Up => cursor = cursor.saturating_sub(1),
                    KeyCode::Char(c) => {
                        query.push(c);
                        cursor = 0;
                    }
                    KeyCode::Backspace => {
                        query.pop();
                        cursor = 0;
                    }
                    _ => {}
                }
                self.overlay = Some(Overlay::Pick {
                    scope,
                    query,
                    cursor,
                });
            }
            None => {}
        }
        AppCommand::Noop
    }
}
