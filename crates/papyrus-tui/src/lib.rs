//! Terminal user interface for Papyrus.

pub mod app;
pub mod event;
pub mod ui;

pub use app::{ActivePanel, App, CollectionItem};
pub use event::{map_key_event, map_key_event_for_app, map_key_event_with_context, run_app};
pub use ui::render;

/// Error type for TUI operations and terminal lifecycle.
#[derive(Debug, thiserror::Error)]
pub enum TuiError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Db(#[from] papyrus_core::db::RepoError),
    #[error("Database connection error: {0}")]
    DbConn(#[from] papyrus_core::db::DbError),
}

/// Returns the current version of the TUI crate.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Launches the interactive TUI application using the given configuration.
pub fn run_tui(config: &papyrus_core::config::Config) -> Result<(), TuiError> {
    let conn = papyrus_core::db::open_database(&config.database_path)?;
    let mut app = App::from_db_conn(conn)?;
    app.set_config(config.clone());
    app.active_panel = ActivePanel::Papers;

    let search_dir = config.library_path.join(".papyrus").join("search");
    if let Ok(search_index) = papyrus_core::search::SearchIndex::open_or_create(&search_dir) {
        app.set_search_index(std::sync::Arc::new(search_index));
    }

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let res = event::run_app(&mut terminal, &mut app);

    // Always restore terminal state even if run_app failed
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    );
    let _ = terminal.show_cursor();

    res.map_err(TuiError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), "0.1.0");
    }
}
