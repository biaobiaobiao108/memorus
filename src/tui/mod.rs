pub mod app;
pub mod event;
pub mod ui;

use std::io::{self, Stdout};

use anyhow::{anyhow, Result};
use crossterm::{
    cursor::Show,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::db::Database;
use crate::tui::app::App;

pub fn run(db: Database) -> Result<()> {
    let mut terminal_guard = TerminalGuard::enter()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(db)?;
    let run_result = run_app(&mut terminal, &mut app);
    let restore_result = terminal_guard.restore();

    match (run_result, restore_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(run_error), Ok(())) => Err(run_error),
        (Ok(()), Err(restore_error)) => Err(restore_error),
        (Err(run_error), Err(restore_error)) => Err(anyhow!(
            "TUI 运行失败: {run_error:#}; 同时无法完整恢复终端: {restore_error:#}"
        )),
    }
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    let mut prev_mode = app.mode;
    while !app.should_quit {
        if prev_mode != app.mode {
            terminal.clear()?;
            prev_mode = app.mode;
        }
        terminal.draw(|f| ui::draw(f, app))?;
        event::handle_events(app)?;
    }
    Ok(())
}

struct TerminalGuard {
    active: bool,
}

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let guard = Self { active: true };
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            crossterm::cursor::SetCursorStyle::BlinkingBar
        )?;
        Ok(guard)
    }

    fn restore(&mut self) -> Result<()> {
        if !self.active {
            return Ok(());
        }

        let raw_mode_result = disable_raw_mode();
        let mut stdout = io::stdout();
        let screen_result = execute!(
            stdout,
            DisableMouseCapture,
            LeaveAlternateScreen,
            crossterm::cursor::SetCursorStyle::DefaultUserShape,
            Show
        );

        match (raw_mode_result, screen_result) {
            (Ok(()), Ok(())) => {
                self.active = false;
                Ok(())
            }
            (Err(raw_error), Ok(())) => Err(raw_error.into()),
            (Ok(()), Err(screen_error)) => Err(screen_error.into()),
            (Err(raw_error), Err(screen_error)) => Err(anyhow!(
                "关闭 raw mode 失败: {raw_error}; 恢复终端屏幕失败: {screen_error}"
            )),
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
