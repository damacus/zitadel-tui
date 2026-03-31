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
    tui::{draw, App, CanvasMode, Focus},
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

        if event::poll(std::time::Duration::from_millis(100))? {
            let Event::Key(key) = event::read()? else {
                continue;
            };

            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('j') | KeyCode::Down => match app.focus {
                    Focus::Resources => app.next_resource(),
                    Focus::Actions => app.next_action(),
                    Focus::Form => app.form_next_field(),
                    Focus::Records => app.next_record(),
                },
                KeyCode::Char('k') | KeyCode::Up => match app.focus {
                    Focus::Resources => app.previous_resource(),
                    Focus::Actions => app.previous_action(),
                    Focus::Form => app.form_previous_field(),
                    Focus::Records => app.previous_record(),
                },
                KeyCode::Char('h') | KeyCode::Left => {
                    if app.focus == Focus::Form {
                        app.form_toggle_or_cycle(false);
                    } else {
                        app.previous_resource();
                    }
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    if app.focus == Focus::Form {
                        app.form_toggle_or_cycle(true);
                    } else {
                        app.next_resource();
                    }
                }
                KeyCode::Char('i') => app.toggle_inspector(),
                KeyCode::Char('n') => app.focus = Focus::Actions,
                KeyCode::Char('g') => app.focus = Focus::Resources,
                KeyCode::Enter => match &app.canvas_mode {
                    CanvasMode::Browse => {
                        let mode = conductor.begin_action(
                            app.active_resource(),
                            app.selected_action,
                            app.selected_record(),
                        );
                        app.set_canvas_mode(mode);
                    }
                    CanvasMode::EditForm(form) | CanvasMode::Setup(form) => {
                        let next = conductor.submit_form(form).await;
                        app.set_canvas_mode(next);
                        app.sync_runtime(conductor.bootstrap_state());
                    }
                    CanvasMode::Confirm(confirm) => {
                        let next = conductor.confirm(confirm.pending.clone()).await;
                        app.set_canvas_mode(next);
                        app.sync_runtime(conductor.bootstrap_state());
                    }
                    CanvasMode::Success(_) | CanvasMode::Error(_) => app.reset_to_browse(),
                },
                KeyCode::Esc => match app.canvas_mode {
                    CanvasMode::Browse => {}
                    CanvasMode::EditForm(_) | CanvasMode::Setup(_) | CanvasMode::Confirm(_) => {
                        app.reset_to_browse()
                    }
                    CanvasMode::Success(_) | CanvasMode::Error(_) => app.reset_to_browse(),
                },
                KeyCode::Backspace => {
                    if app.focus == Focus::Form {
                        app.form_backspace();
                    }
                }
                KeyCode::Char(' ') => {
                    if app.focus == Focus::Form {
                        app.form_toggle_or_cycle(true);
                    }
                }
                KeyCode::Char(ch) if app.focus == Focus::Form => app.form_insert_char(ch),
                KeyCode::Tab => app.advance_focus(),
                KeyCode::BackTab => app.reverse_focus(),
                _ => {}
            }
        }
    }

    Ok(())
}
