use std::io;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;

use crate::{
    conductor::TuiConductor,
    tui::{draw, App, AppCommand},
};

pub async fn run_tui(mut conductor: TuiConductor) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let terminal = ratatui::Terminal::new(CrosstermBackend::new(stdout))?;
    let result = run_app(terminal, &mut conductor).await;
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    result
}

async fn run_app(
    mut terminal: ratatui::Terminal<CrosstermBackend<io::Stdout>>,
    conductor: &mut TuiConductor,
) -> Result<()> {
    let mut app = App::from_bootstrap(conductor.bootstrap_state());

    loop {
        terminal.draw(|frame| draw(frame, &app))?;

        let Some(key_code) = read_key_press(std::time::Duration::from_millis(100))? else {
            continue;
        };

        let command = app.handle_key(key_code);
        if !handle_app_command(command, &mut app, conductor).await? {
            break;
        }
    }

    Ok(())
}

fn read_key_press(timeout: std::time::Duration) -> Result<Option<KeyCode>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }

    let Event::Key(key) = event::read()? else {
        return Ok(None);
    };

    if key.kind != KeyEventKind::Press {
        return Ok(None);
    }

    Ok(Some(key.code))
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
