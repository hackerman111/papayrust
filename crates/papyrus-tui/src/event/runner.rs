use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::backend::Backend;
use ratatui::Terminal;

use super::app_mapping::map_key_event_for_app;
use crate::app::App;
use crate::ui;

/// Runs the interactive terminal event loop until Quit is received or app stops running.
pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    while app.running {
        if app.needs_clear {
            terminal.clear()?;
            app.needs_clear = false;
        }
        terminal.draw(|f| ui::render(app, f))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.kind == KeyEventKind::Press {
                    if let Some(action) = map_key_event_for_app(key_event, app) {
                        app.dispatch(action);
                    }
                }
            }
        }
    }

    Ok(())
}
