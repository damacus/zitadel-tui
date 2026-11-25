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
    tui::{draw, App, AppCommand, CanvasMode, FieldKind, Focus, Hits, Target},
};

pub async fn run_tui(mut conductor: TuiConductor) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let terminal = ratatui::Terminal::new(CrosstermBackend::new(stdout))?;
    let result = run_app(terminal, &mut conductor).await;
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
                    break;
                }
                app.handle_key(key.code)
            }
            Event::Mouse(mouse) => mouse_command(&mut app, &hits, mouse),
            _ => continue,
        };
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
        Target::Section(i) => {
            app.set_canvas_mode(CanvasMode::Browse);
            app.selected_resource = i;
            app.focus = Focus::Records;
            app.cursor = 0;
            app.expanded = None;
            app.filter.clear();
            app.filtering = false;
        }
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
        AppCommand::BeginAction {
            resource,
            action_index,
            selected_record,
        } => {
            let mode = conductor.begin_action(resource, action_index, selected_record.as_ref());
            app.set_canvas_mode(mode);
            Ok(true)
        }
        AppCommand::SubmitForm(form) => {
            let next = conductor.submit_form(&form).await;
            app.set_canvas_mode(next);
            app.sync_runtime(conductor.bootstrap_state());
            Ok(true)
        }
        AppCommand::Confirm(pending) => {
            let next = conductor.confirm(pending).await;
            app.set_canvas_mode(next);
            app.sync_runtime(conductor.bootstrap_state());
            Ok(true)
        }
    }
}
