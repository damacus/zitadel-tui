use super::{App, AppCommand, Focus, ResourceKind};
use crossterm::event::KeyCode;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Entry {
    Record(usize),
    Action(ResourceKind, usize, Option<usize>),
}

impl App {
    pub(super) fn section(&self) -> &'static str {
        ["Applications", "Users", "Identity providers", "Settings"][self.selected_resource.min(3)]
    }

    pub(super) fn entries(&self) -> Vec<Entry> {
        use ResourceKind::*;
        let kind = self.active_resource();
        if matches!(kind, Auth | Config) {
            return vec![Entry::Action(Auth, 0, None), Entry::Action(Config, 0, None)];
        }
        let query = self.filter.to_lowercase();
        let mut entries = Vec::new();
        for (i, record) in self.active_records().iter().enumerate() {
            if !record.name.to_lowercase().contains(&query) {
                continue;
            }
            entries.push(Entry::Record(i));
            if self.expanded != Some(i) {
                continue;
            }
            if kind == Applications {
                if !matches!(record.kind.as_str(), "api" | "unknown") {
                    entries.push(Entry::Action(kind, 4, Some(i)))
                }
                if record.kind == "confidential" {
                    entries.push(Entry::Action(kind, 1, Some(i)))
                }
                entries.push(Entry::Action(kind, 2, Some(i)));
            } else if kind == Users {
                entries.push(Entry::Action(kind, 2, Some(i)));
            }
        }
        let general: &[usize] = match kind {
            Applications => &[0, 3],
            Users => &[0, 1, 3],
            _ => &[0],
        };
        entries.extend(general.iter().map(|i| Entry::Action(kind, *i, None)));
        entries
    }

    pub(super) fn directory_key(&mut self, key: KeyCode) -> AppCommand {
        if self.filtering {
            match key {
                KeyCode::Esc => {
                    self.filter.clear();
                    self.filtering = false;
                }
                KeyCode::Enter | KeyCode::Down => self.filtering = false,
                KeyCode::Backspace => {
                    self.filter.pop();
                }
                KeyCode::Char(c) => self.filter.push(c),
                _ => {}
            }
            self.cursor = 0;
            return AppCommand::Noop;
        }
        let rail = self.focus == Focus::Resources;
        let entries = self.entries();
        match key {
            KeyCode::Char('q') => return AppCommand::Quit,
            KeyCode::Char('/') if self.selected_resource < 3 => {
                self.filtering = true;
                self.focus = Focus::Records;
                self.expanded = None;
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                let down = matches!(key, KeyCode::Down | KeyCode::Char('j'));
                let (index, len) = if rail {
                    (&mut self.selected_resource, 4)
                } else {
                    (&mut self.cursor, entries.len())
                };
                *index = if down {
                    (*index + 1) % len
                } else {
                    (*index + len - 1) % len
                };
                if rail {
                    self.expanded = None;
                    self.cursor = 0;
                    self.filter.clear();
                }
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') if rail => {
                self.focus = Focus::Records
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                match entries.get(self.cursor) {
                    Some(Entry::Record(i)) => {
                        self.expanded = if self.expanded == Some(*i) {
                            None
                        } else {
                            Some(*i)
                        };
                        self.cursor = self
                            .entries()
                            .iter()
                            .position(|row| row == &Entry::Record(*i))
                            .unwrap_or(0);
                    }
                    Some(Entry::Action(kind, index, target)) if key == KeyCode::Enter => {
                        return AppCommand::BeginAction {
                            resource: *kind,
                            action_index: *index,
                            selected_record: target.map(|i| self.active_records()[i].clone()),
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                if let Some(i) = self.expanded.take() {
                    self.cursor = self
                        .entries()
                        .iter()
                        .position(|e| *e == Entry::Record(i))
                        .unwrap_or(0);
                } else if !self.filter.is_empty() {
                    self.filter.clear();
                    self.cursor = 0;
                } else {
                    self.focus = Focus::Resources;
                }
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = if rail {
                    Focus::Records
                } else {
                    Focus::Resources
                }
            }
            _ => {}
        }
        AppCommand::Noop
    }
}
