use super::{
    directory::Entry, widgets::render_form_line, App, CanvasMode, Focus, Overlay, ResourceKind,
    Scope,
};
use ratatui::{
    layout::{Constraint, Layout, Rect, Spacing},
    style::{Color, Modifier, Style},
    symbols::merge::MergeStrategy,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap},
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
    Scope(Scope),
    ScopeChoice(usize),
    Keep,
    Discard,
    Refresh,
    Record(usize),
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
    let [header, tab_bar, body, footer] = Layout::vertical([
        Constraint::Length(if narrow { 4 } else { 3 }),
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .spacing(Spacing::Overlap(1))
    .areas(area);
    draw_header(frame, app, header, hits);
    let names = if area.width >= 56 {
        ["Applications", "Users", "Identity providers", "Settings"]
    } else {
        ["Apps", "Users", "IDPs", "Settings"]
    };
    let inner = border().inner(tab_bar);
    let mut x = inner.x;
    for (i, name) in names.iter().enumerate() {
        let width = name.len() as u16 + 2;
        if !app.setup_required && x < inner.right() {
            hits.push((
                Rect::new(x, inner.y, width.min(inner.right() - x), inner.height),
                Target::Section(i),
            ));
        }
        x += width + 1;
    }
    frame.render_widget(
        Tabs::new(names)
            .block(border())
            .select(app.selected_resource)
            .highlight_style(selected())
            .divider("│"),
        tab_bar,
    );
    let browse = matches!(app.canvas_mode, CanvasMode::Browse);
    if !narrow && app.selected_resource < 3 && !app.setup_required {
        let [left, right] = Layout::horizontal([Constraint::Percentage(33), Constraint::Fill(1)])
            .spacing(Spacing::Overlap(1))
            .areas(body);
        frame.render_widget(border(), body);
        // Keep the tab rule uninterrupted. Some terminal fonts place the
        // horizontal strokes of tee glyphs below the regular border glyph.
        let right_inner = border().inner(right);
        frame.render_widget(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(BORDER)),
            Rect::new(
                right.x,
                right_inner.y.saturating_add(1),
                1,
                right_inner.height.saturating_sub(2),
            ),
        );
        if browse {
            draw_content(frame, border().inner(left), app, hits);
            draw_details(frame, border().inner(right), app);
        } else {
            draw_record_list(frame, border().inner(left), app, hits);
            draw_content(frame, border().inner(right), app, hits);
        }
    } else {
        frame.render_widget(border(), body);
        draw_content(frame, border().inner(body), app, hits);
    }
    let hint = match &app.canvas_mode {
        CanvasMode::Browse if app.filtering => "Enter Apply   Esc Clear",
        CanvasMode::Browse if app.focus == Focus::Resources => {
            "←→ Tabs   Enter Open   / Search   F5 Refresh   q Quit"
        }
        CanvasMode::Browse => "↑↓ Move   Enter Open   / Search   F5 Refresh   F1–F4 Tabs   q Quit",
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
    if browse && !app.filtering {
        if let Some(start) = hint.find("F5 Refresh") {
            let x = hint[..start].chars().count() as u16;
            if x < foot.width {
                hits.push((
                    Rect::new(foot.x + x, foot.y, 10.min(foot.width - x), 1),
                    Target::Refresh,
                ));
            }
        }
    }
    frame.render_widget(Paragraph::new(hint).block(border()), footer);
    if let Some(overlay) = &app.overlay {
        draw_overlay(frame, app, overlay, hits);
    }
}

fn section_error(app: &App) -> Option<&str> {
    match app.active_resource() {
        ResourceKind::Applications => app.app_error.as_deref().or(app.project_error.as_deref()),
        ResourceKind::Users => app.user_error.as_deref(),
        ResourceKind::Idps => app.idp_error.as_deref(),
        _ => None,
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect, hits: &mut Hits) {
    let inner = border().inner(area);
    frame.render_widget(border(), area);
    if inner.is_empty() {
        return;
    }
    let org = app
        .organizations
        .iter()
        .find(|(id, _)| Some(id) == app.organization.as_ref())
        .map(|(_, n)| n.as_str())
        .or(app.organization.as_deref())
        .unwrap_or("Current organisation");
    let project = app
        .projects
        .iter()
        .find(|(id, _)| id == &app.project)
        .map(|(_, n)| n.as_str())
        .unwrap_or(&app.project);
    let narrow = inner.height > 1;
    let (brand, context) = if narrow {
        let [a, b] = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(inner);
        (a, b)
    } else {
        let [a, b] = Layout::horizontal([Constraint::Length(11), Constraint::Fill(1)]).areas(inner);
        (a, b)
    };
    frame.render_widget(Paragraph::new(" ZITADEL").style(heading()), brand);
    let [org_area, project_area, host_area] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Percentage(if narrow { 0 } else { 27 }),
    ])
    .areas(context);
    for (area, scope, title) in [
        (org_area, Scope::Organisation, format!("F6 Org ▾ {org}")),
        (
            project_area,
            Scope::Project,
            format!("F7 Project ▾ {project}"),
        ),
    ] {
        if !app.setup_required {
            hits.push((area, Target::Scope(scope)));
        }
        frame.render_widget(Paragraph::new(title), area);
    }
    frame.render_widget(
        Paragraph::new(app.host.trim_start_matches("https://")),
        host_area,
    );
}

fn draw_details(frame: &mut Frame, area: Rect, app: &App) {
    let entries = app.entries();
    let index = match entries.get(app.cursor) {
        Some(Entry::Record(i)) => Some(*i),
        Some(Entry::Action(_, _, i)) => *i,
        None => None,
    };
    let Some(index) = index else {
        if let Some(Entry::Action(_, i, _)) = entries.get(app.cursor) {
            frame.render_widget(
                Paragraph::new(app.actions()[*i].label).style(heading()),
                area,
            );
        }
        return;
    };
    let record = &app.active_records()[index];
    let mut lines = vec![
        Line::styled(record.name.clone(), heading()),
        Line::from(record.kind.clone()),
        Line::from(format!("ID  {}", record.id)),
    ];
    if app.active_resource() == ResourceKind::Applications {
        lines.push(Line::from(format!("Client ID  {}", record.detail)));
        if record.redirect_uris.is_empty() {
            lines.push(Line::from(record.summary.clone()));
        } else {
            lines.push(Line::default());
            lines.push(Line::styled("Redirect URIs", heading()));
            lines.extend(record.redirect_uris.iter().cloned().map(Line::from));
        }
    } else {
        for value in [&record.detail, &record.summary] {
            if !value.is_empty() {
                lines.push(Line::from(value.clone()));
            }
        }
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}
fn draw_record_list(frame: &mut Frame, area: Rect, app: &App, hits: &mut Hits) {
    let query = app.filter.to_lowercase();
    let mut lines = vec![Line::styled(app.section(), heading())];
    for (i, r) in app
        .active_records()
        .iter()
        .enumerate()
        .filter(|(_, r)| r.name.to_lowercase().contains(&query))
        .take(area.height.saturating_sub(1) as usize)
    {
        hits.push((
            Rect::new(area.x, area.y + lines.len() as u16, area.width, 1),
            Target::Record(i),
        ));
        lines.push(Line::styled(
            r.name.clone(),
            if app.expanded == Some(i) {
                selected()
            } else {
                Style::default()
            },
        ));
    }
    frame.render_widget(Paragraph::new(lines), area);
}
fn draw_overlay(frame: &mut Frame, app: &App, overlay: &Overlay, hits: &mut Hits) {
    hits.clear();
    let area = frame.area();
    let width = area.width.min(68);
    let height = area.height.min(match overlay {
        Overlay::Pick { .. } => 16,
        _ => 6,
    });
    let box_area = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, box_area);
    let block = border().style(Style::default().bg(INK).fg(TEXT));
    let inner = block.inner(box_area);
    if inner.is_empty() {
        return;
    }
    match overlay {
        Overlay::SubmissionError(message, offset) => {
            frame.render_widget(block.title(format!(" {} ", message.title)), box_area);
            let [body, footer] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(inner);
            let paragraph = Paragraph::new(message.lines.join("\n")).wrap(Wrap { trim: false });
            let max = paragraph
                .line_count(body.width)
                .saturating_sub(usize::from(body.height));
            frame.render_widget(
                paragraph.scroll(((*offset).min(max).min(u16::MAX as usize) as u16, 0)),
                body,
            );
            frame.render_widget(Paragraph::new("Enter  Return to form"), footer);
            hits.push((footer, Target::Keep));
        }
        Overlay::Discard(_) => {
            frame.render_widget(block.title(" Unsaved changes "), box_area);
            let mut lines = vec![
                Line::from("Leave this form and discard your changes?"),
                Line::default(),
            ];
            for (text, target) in [
                ("Enter  Keep editing", Target::Keep),
                ("D      Discard changes", Target::Discard),
            ] {
                let y = inner.y + lines.len() as u16;
                if y < inner.bottom() {
                    hits.push((Rect::new(inner.x, y, inner.width, 1), target));
                }
                lines.push(Line::from(text));
            }
            frame.render_widget(Paragraph::new(lines), inner);
        }
        Overlay::Pick {
            scope,
            query,
            cursor,
        } => {
            let (title, error) = match scope {
                Scope::Organisation => (" Organisation ", app.org_error.as_deref()),
                Scope::Project => (" Project ", app.project_error.as_deref()),
            };
            frame.render_widget(block.title(title), box_area);
            let choices = app.matches_scope(*scope, query);
            let mut lines = vec![Line::from(format!("/ {query}▏"))];
            if let Some(err) = error {
                lines.push(Line::from(err));
            }
            if choices.is_empty() && error.is_none() {
                lines.push(Line::from("No matches"));
            }
            let count = inner.height.saturating_sub(lines.len() as u16) as usize;
            for (i, (id, name)) in choices
                .iter()
                .enumerate()
                .skip((cursor + 1).saturating_sub(count))
                .take(count)
            {
                let y = inner.y + lines.len() as u16;
                hits.push((
                    Rect::new(inner.x, y, inner.width, 1),
                    Target::ScopeChoice(i),
                ));
                lines.push(Line::styled(
                    format!(
                        "{} {name}   {id}",
                        if app.selected_scope(*scope) == Some(id.as_str()) {
                            "✓"
                        } else {
                            " "
                        }
                    ),
                    if i == *cursor {
                        selected()
                    } else {
                        Style::default()
                    },
                ));
            }
            frame.render_widget(Paragraph::new(lines), inner);
        }
    }
}

fn draw_content(frame: &mut Frame, area: Rect, app: &App, hits: &mut Hits) {
    if area.width <= 1 || area.height == 0 {
        return;
    }
    let area = Rect::new(area.x + 1, area.y, area.width - 1, area.height);
    if area.is_empty() {
        return;
    }
    let mut area = area;
    if let CanvasMode::EditForm(form) | CanvasMode::Setup(form) = &app.canvas_mode {
        if let Some(field) = form.fields.get(form.selected_field) {
            let wrap = Wrap { trim: true };
            let lines = form
                .fields
                .iter()
                .map(|f| {
                    Paragraph::new(f.help.as_str())
                        .wrap(wrap)
                        .line_count(area.width)
                })
                .max()
                .unwrap_or(0);
            let height = (lines + 1).min(usize::from(area.height / 2)) as u16;
            if height >= 2 {
                let [fields, help] =
                    Layout::vertical([Constraint::Fill(1), Constraint::Length(height)]).areas(area);
                frame.render_widget(
                    Paragraph::new(field.help.as_str())
                        .wrap(wrap)
                        .style(Style::default().fg(ICE))
                        .block(
                            Block::default()
                                .borders(Borders::TOP)
                                .title(" Field help ")
                                .border_style(Style::default().fg(BORDER)),
                        ),
                    help,
                );
                area = fields;
            }
        }
    }
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
                    if let Some(error) = section_error(app) {
                        error
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
                        if open && frame.area().width < 76 {
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
                targets.push((i + 3, Target::Field(i)));
                let chosen = i == f.selected_field;
                push(
                    render_form_line(field, chosen),
                    if chosen { selected() } else { Style::default() },
                );
                if chosen {
                    active = i + 3;
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
        CanvasMode::EditForm(f) | CanvasMode::Setup(f) if app.dirty() => {
            format!("{} · Unsaved changes", f.title)
        }
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
