use std::io;

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;

use crate::{
    conductor::TuiConductor,
    tui::{
        draw, App, AppCommand, CanvasMode, Destination, FieldKind, Focus, Hits, MessageState,
        ResourceKind, Scope, Target,
    },
};

pub async fn run_tui(conductor: TuiConductor) -> Result<()> {
    run_tui_loading(std::future::ready(conductor)).await
}

pub async fn run_tui_loading(
    bootstrap: impl std::future::Future<Output = TuiConductor>,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let result = async {
        let mut terminal = ratatui::Terminal::new(CrosstermBackend::new(stdout))?;
        terminal.draw(|frame| {
            frame.render_widget(
                ratatui::widgets::Paragraph::new("Loading directory…")
                    .block(ratatui::widgets::Block::bordered().title(" ZITADEL ")),
                frame.area(),
            );
        })?;
        let wait_for_quit = async {
            loop {
                if event::poll(std::time::Duration::ZERO)? {
                    if let Event::Key(key) = event::read()? {
                        if key.kind == KeyEventKind::Press
                            && (key.code == KeyCode::Char('q')
                                || (key.code == KeyCode::Char('c')
                                    && key.modifiers.contains(event::KeyModifiers::CONTROL)))
                        {
                            return Ok::<(), std::io::Error>(());
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
        };
        let mut conductor = tokio::select! {
            conductor = bootstrap => conductor,
            quit = wait_for_quit => return Ok(quit?),
        };
        run_app(terminal, &mut conductor).await
    }
    .await;
    disable_raw_mode()?;
    execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen)?;
    result
}

async fn run_app(
    mut terminal: ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    conductor: &mut TuiConductor,
) -> Result<()> {
    let mut app = App::from_bootstrap(conductor.bootstrap_state());

    let mut hits = Hits::new();
    loop {
        terminal.draw(|frame| draw(frame, &app, &mut hits))?;
        let command = match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if key.modifiers.contains(event::KeyModifiers::CONTROL)
                    && key.code == KeyCode::Char('c')
                {
                    app.navigate(Destination::Quit)
                } else {
                    app.handle_key(key.code)
                }
            }
            Event::Mouse(mouse) => mouse_command(&mut app, &hits, mouse),
            _ => continue,
        };
        if !matches!(command, AppCommand::Quit | AppCommand::Noop) {
            terminal.draw(|frame| {
                draw(frame, &app, &mut hits);
                let area = frame.area();
                if area.height >= 2 {
                    let row = ratatui::layout::Rect::new(
                        area.x + 1,
                        area.bottom() - 2,
                        area.width.saturating_sub(2),
                        1,
                    );
                    frame.render_widget(ratatui::widgets::Clear, row);
                    frame.render_widget(ratatui::widgets::Paragraph::new("Loading…"), row);
                }
            })?;
        }
        if !handle_app_command(command, &mut app, conductor).await? {
            break;
        }
    }

    Ok(())
}

fn mouse_command(app: &mut App, hits: &Hits, event: event::MouseEvent) -> AppCommand {
    match event.kind {
        MouseEventKind::ScrollDown => return app.handle_key(KeyCode::Down),
        MouseEventKind::ScrollUp => return app.handle_key(KeyCode::Up),
        MouseEventKind::Down(MouseButton::Left) => {}
        _ => return AppCommand::Noop,
    }
    let Some((_, target)) = hits
        .iter()
        .find(|(rect, _)| rect.contains((event.column, event.row).into()))
    else {
        return AppCommand::Noop;
    };
    match *target {
        Target::Section(i) => return app.navigate(Destination::Section(i)),
        Target::Record(i) => return app.navigate(Destination::OpenRecord(i)),
        Target::Scope(scope) => return app.open_picker(scope),
        Target::ScopeChoice(i) => return app.choose_scope(i),
        Target::Discard => return app.handle_key(KeyCode::Char('d')),
        Target::Keep => return app.handle_key(KeyCode::Esc),
        Target::Refresh => return AppCommand::Refresh,
        Target::Entry(i) => {
            app.focus = Focus::Records;
            app.cursor = i;
            app.filtering = false;
            return app.handle_key(KeyCode::Enter);
        }
        Target::Field(i) => {
            if let CanvasMode::Setup(f) | CanvasMode::EditForm(f) = &mut app.canvas_mode {
                f.selected_field = i;
                if matches!(
                    f.fields[i].kind,
                    FieldKind::Checkbox | FieldKind::Choice(_) | FieldKind::Toggle
                ) {
                    return app.handle_key(KeyCode::Char(' '));
                }
            }
        }
        Target::Filter => {
            if !app.filtering {
                return app.handle_key(KeyCode::Char('/'));
            }
        }
        Target::Submit => return app.handle_key(KeyCode::Enter),
        Target::Back => return app.handle_key(KeyCode::Esc),
    }
    AppCommand::Noop
}

async fn handle_app_command(
    command: AppCommand,
    app: &mut App,
    conductor: &mut TuiConductor,
) -> Result<bool> {
    match command {
        AppCommand::Noop => Ok(true),
        AppCommand::Quit => Ok(false),
        AppCommand::Refresh => {
            conductor.refresh_runtime().await;
            app.sync_runtime(conductor.bootstrap_state());
            if app.setup_required {
                app.reset_to_browse();
            }
            Ok(true)
        }
        AppCommand::ChangeScope(scope, id) => {
            let result = match scope {
                Scope::Organisation => conductor.select_organization(&id).await,
                Scope::Project => conductor.select_project(&id).await,
            };
            app.sync_runtime(conductor.bootstrap_state());
            app.expanded = None;
            app.cursor = 0;
            app.filter.clear();
            app.filtering = false;
            match result {
                Ok(()) => app.reset_to_browse(),
                Err(error) => app.set_canvas_mode(CanvasMode::Error(MessageState {
                    title: "Scope change failed".into(),
                    lines: vec![error.to_string()],
                })),
            }
            Ok(true)
        }
        AppCommand::BeginAction {
            resource,
            action_index,
            selected_record,
        } => {
            let mode = match (resource, action_index) {
                (ResourceKind::Users, 4..=8) => {
                    conductor
                        .begin_user_action(action_index, selected_record.as_ref())
                        .await
                }
                (ResourceKind::Idps, 1) => {
                    conductor.edit_google_form(selected_record.as_ref()).await
                }
                _ => conductor.begin_action(resource, action_index, selected_record.as_ref()),
            };
            app.set_canvas_mode(mode);
            Ok(true)
        }
        AppCommand::SubmitForm(form) => {
            let next = conductor.submit_form(&form).await;
            app.finish_submission(next);
            app.sync_runtime(conductor.bootstrap_state());
            Ok(true)
        }
        AppCommand::Confirm(pending) => {
            let next = conductor.confirm(pending).await;
            app.finish_confirmation(next);
            app.sync_runtime(conductor.bootstrap_state());
            Ok(true)
        }
    }
}
