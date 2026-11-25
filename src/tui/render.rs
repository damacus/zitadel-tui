use super::{directory::Entry, widgets::render_form_line, App, CanvasMode, Focus, ResourceKind};
use ratatui::{
    layout::{Constraint, Layout, Rect, Spacing},
    style::{Color, Modifier, Style},
    symbols::merge::MergeStrategy,
    text::Line,
    widgets::{Block, List, Paragraph, Wrap},
    Frame,
};

const INK: Color = Color::Rgb(14, 22, 34);
const TEXT: Color = Color::Rgb(230, 238, 247);
const ICE: Color = Color::Rgb(166, 237, 243);
const BORDER: Color = Color::Rgb(95, 117, 140);
fn selected() -> Style {
    Style::default()
        .fg(INK)
        .bg(ICE)
        .add_modifier(Modifier::BOLD)
}
fn heading() -> Style {
    Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
}

#[derive(Clone, Copy)]
pub enum Target {
    Section(usize),
    Entry(usize),
    Field(usize),
    Filter,
    Submit,
    Back,
}
pub type Hits = Vec<(Rect, Target)>;
fn border() -> Block<'static> {
    Block::bordered()
        .border_style(Style::default().fg(BORDER))
        .merge_borders(MergeStrategy::Exact)
}
pub fn draw(frame: &mut Frame, app: &App, hits: &mut Hits) {
    hits.clear();
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(INK).fg(TEXT)),
        area,
    );
    let narrow = area.width < 76;
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(if narrow { 4 } else { 3 }),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .spacing(Spacing::Overlap(1))
    .areas(area);
    let context = format!("project {}   host {}", app.project, app.host);
    let title = if narrow {
        format!(" ZITADEL TUI   Directory\n {context}")
    } else {
        format!(" ZITADEL TUI   Directory     {context}")
    };
    frame.render_widget(Paragraph::new(title).block(border()), header);
    let browse = matches!(app.canvas_mode, CanvasMode::Browse);
    let show_rail = !narrow || (browse && app.focus == Focus::Resources);
    let content = if show_rail {
        let [rail, main] = Layout::horizontal([
            Constraint::Length(if narrow { body.width } else { 23 }),
            Constraint::Min(0),
        ])
        .spacing(Spacing::Overlap(1))
        .areas(body);
        let mut items = vec![];
        for (i, name) in ["Applications", "Users", "Identity providers", "Settings"]
            .iter()
            .enumerate()
        {
            if i == 3 {
                items.push(Line::default())
            }
            let y = rail.y + 1 + items.len() as u16;
            if browse && y < rail.bottom().saturating_sub(1) {
                hits.push((
                    Rect::new(rail.x + 1, y, rail.width.saturating_sub(2), 1),
                    Target::Section(i),
                ));
            }
            let active = i == app.selected_resource;
            let style = if active && browse && app.focus == Focus::Resources {
                selected()
            } else if active {
                heading()
            } else {
                Style::default()
            };
            items.push(Line::styled(
                format!(" {} {name}", if active { "▸" } else { " " }),
                style,
            ));
        }
        frame.render_widget(List::new(items).block(border()), rail);
        if narrow {
            Rect::default()
        } else {
            main
        }
    } else {
        body
    };
    if content.width > 1 {
        let inner = border().inner(content);
        frame.render_widget(border(), content);
        if !inner.is_empty() {
            draw_content(frame, inner, app, hits)
        }
    }
    let hint = match &app.canvas_mode {
        CanvasMode::Browse if app.filtering => "Enter Apply   Esc Clear",
        CanvasMode::Browse => "↑↓ Move   Enter Open   Esc Back   q Quit",
        CanvasMode::EditForm(f) | CanvasMode::Setup(f) => {
            &format!("{}   Tab Field   Esc Back", f.submit_label)
        }
        CanvasMode::Confirm(c) => &format!("{}   Esc Cancel", c.submit_label),
        _ => "Enter Continue   ↑↓ Scroll",
    };
    let foot = border().inner(footer);
    if !foot.is_empty() {
        for (text, target) in [("Enter", Target::Submit), ("Esc", Target::Back)] {
            if let Some(i) = hint.find(text) {
                let x = hint[..i].chars().count() as u16;
                let end = hint[i..].find("   ").unwrap_or(hint.len() - i);
                hits.push((
                    Rect::new(
                        foot.x + x,
                        foot.y,
                        (end as u16).min(foot.width.saturating_sub(x)),
                        1,
                    ),
                    target,
                ));
            }
        }
    }
    frame.render_widget(Paragraph::new(hint).block(border()), footer);
}

fn draw_content(frame: &mut Frame, area: Rect, app: &App, hits: &mut Hits) {
    let area = Rect::new(area.x + 1, area.y, area.width - 1, area.height);
    let mut rows = vec![];
    let mut targets = Vec::new();
    let mut active = 0usize;
    let mut offsets = vec![];
    let mut height = 0usize;
    let mut push = |text: String, style: Style| {
        offsets.push(height);
        let para = Paragraph::new(text.clone()).wrap(Wrap { trim: false });
        height += para.line_count(area.width);
        rows.push(Line::styled(text, style));
    };
    push(canvas_title(app), heading());
    match &app.canvas_mode {
        CanvasMode::Browse => {
            if app.selected_resource < 3 {
                targets.push((1, Target::Filter));
                push(
                    if app.filtering {
                        format!("/ {}▏", app.filter)
                    } else if app.filter.is_empty() {
                        format!("/ Filter {}", app.section().to_lowercase())
                    } else {
                        format!("/ {}", app.filter)
                    },
                    Style::default().fg(ICE),
                );
            }
            let entries = app.entries();
            if app.selected_resource < 3 && !entries.iter().any(|e| matches!(e, Entry::Record(_))) {
                push(
                    if app.active_resource() == ResourceKind::Idps && app.idp_error.is_some() {
                        app.idp_error.as_deref().unwrap_or_default()
                    } else if app.filter.is_empty() {
                        "No records"
                    } else {
                        "No matches"
                    }
                    .into(),
                    Style::default(),
                );
            }
            push(String::new(), Style::default());
            let mut index = if app.selected_resource < 3 {
                3 + usize::from(!entries.iter().any(|e| matches!(e, Entry::Record(_))))
            } else {
                push(
                    format!("Credentials   {}", app.auth_label),
                    Style::default(),
                );
                push(
                    format!(
                        "Templates     {}",
                        app.templates_path.as_deref().unwrap_or("None configured")
                    ),
                    Style::default(),
                );
                push(String::new(), Style::default());
                5
            };
            for (pos, e) in entries.iter().enumerate() {
                let focused = pos == app.cursor && app.focus != Focus::Resources && !app.filtering;
                targets.push((index, Target::Entry(pos)));
                if focused {
                    active = index
                }
                let style = if focused {
                    selected()
                } else {
                    Style::default()
                };
                match *e {
                    Entry::Record(i) => {
                        let rec = &app.active_records()[i];
                        let open = app.expanded == Some(i);
                        push(
                            format!("{} {}", if open { "▾" } else { "▸" }, rec.name),
                            style,
                        );
                        if open {
                            let mut details = vec![
                                rec.kind.clone(),
                                format!("ID  {}", rec.id),
                                rec.detail.clone(),
                                rec.summary.clone(),
                            ];
                            if !rec.redirect_uris.is_empty() {
                                details.push("Redirect URIs".into());
                                details.extend(rec.redirect_uris.iter().cloned());
                            }
                            for detail in details.into_iter().filter(|s| !s.is_empty()) {
                                push(format!("    {detail}"), Style::default());
                                index += 1;
                            }
                        }
                    }
                    Entry::Action(kind, i, target) => {
                        let label = match kind {
                            ResourceKind::Auth => "Connection and authentication",
                            ResourceKind::Config => "Edit configuration",
                            _ => app.actions()[i].label,
                        };
                        push(
                            format!("{}› {label}", if target.is_some() { "    " } else { "" }),
                            style,
                        );
                    }
                }
                index += 1;
            }
        }
        CanvasMode::EditForm(f) | CanvasMode::Setup(f) => {
            push(f.description.clone(), Style::default());
            push(String::new(), Style::default());
            for (i, field) in f.fields.iter().enumerate() {
                targets.push((i + 3 + usize::from(i > f.selected_field), Target::Field(i)));
                let chosen = i == f.selected_field;
                push(
                    render_form_line(field, chosen),
                    if chosen { selected() } else { Style::default() },
                );
                if chosen {
                    active = i + 3;
                    push(field.help.clone(), Style::default().fg(ICE));
                }
            }
            if f.fields.is_empty() {
                push("No templates configured".into(), Style::default());
            }
        }
        _ => {
            for line in canvas_lines(app) {
                push(line, Style::default());
            }
        }
    }
    let bottom = height.saturating_sub(area.height as usize);
    let offset = if active == 0 {
        app.scroll.min(bottom)
    } else {
        offsets[active]
            .saturating_sub(area.height.saturating_sub(2) as usize)
            .min(bottom)
    };
    for (index, target) in targets {
        let start = offsets[index].max(offset);
        let end = offsets
            .get(index + 1)
            .copied()
            .unwrap_or(height)
            .min(offset + area.height as usize);
        if start < end {
            hits.push((
                Rect::new(
                    area.x,
                    area.y + (start - offset) as u16,
                    area.width,
                    (end - start) as u16,
                ),
                target,
            ));
        }
    }
    frame.render_widget(
        Paragraph::new(rows)
            .wrap(Wrap { trim: false })
            .scroll((offset.min(u16::MAX as usize) as u16, 0)),
        area,
    );
}

pub(super) fn canvas_title(app: &App) -> String {
    match &app.canvas_mode {
        CanvasMode::Browse => app.section().into(),
        CanvasMode::EditForm(f) | CanvasMode::Setup(f) => f.title.clone(),
        CanvasMode::Confirm(c) => c.title.clone(),
        CanvasMode::Error(m) | CanvasMode::Success(m) => m.title.clone(),
    }
}
pub(super) fn canvas_lines(app: &App) -> Vec<String> {
    match &app.canvas_mode {
        CanvasMode::Browse => vec![],
        CanvasMode::EditForm(f) | CanvasMode::Setup(f) => f
            .fields
            .iter()
            .enumerate()
            .map(|(i, v)| render_form_line(v, i == f.selected_field))
            .collect(),
        CanvasMode::Confirm(c) => c.lines.clone(),
        CanvasMode::Error(m) | CanvasMode::Success(m) => m.lines.clone(),
    }
}
