use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{ActivePanel, App};

/// Renders the status and keybindings help bar at the bottom of the screen.
pub fn render_status_bar(app: &App, frame: &mut Frame, area: Rect) {
    let pending_indicator = match (app.pending_count, app.pending_chord) {
        (Some(cnt), Some(ch)) => format!("[{cnt}{ch}] "),
        (Some(cnt), None) => format!("[{cnt}] "),
        (None, Some(ch)) => format!("[{ch}] "),
        (None, None) => String::new(),
    };

    let base_content = if let Some(ref msg) = app.status_message {
        format!("{msg} | Tab/h/l: Panels | ?: Help | q: Quit")
    } else if app.is_showing_help {
        "Help Mode | Esc / q / ?: Close Help".to_string()
    } else if app.is_renaming_collection {
        "Rename Collection | Enter: Confirm | Esc: Cancel".to_string()
    } else if app.is_exporting_collection {
        "Export Collection | Enter: Export | Tab: Autocomplete | Esc: Cancel".to_string()
    } else if app.is_importing_metadata {
        "Import Metadata | Enter: Import | Tab: Autocomplete | Esc: Cancel".to_string()
    } else if app.is_adding_paper {
        "Add Paper | Enter: Import | Tab: Autocomplete | Esc: Cancel".to_string()
    } else if app.is_viewing_fullscreen_toc {
        "Fullscreen TOC | j/k: Scroll | Enter: Jump to page | Esc/q/t: Exit TOC | ?: Help"
            .to_string()
    } else if app.is_confirming_delete {
        "Confirm Delete | Enter: Delete permanently | Esc / q: Cancel".to_string()
    } else if app.is_editing_tags {
        "Edit Tags | Enter: Save tags | Esc: Cancel".to_string()
    } else if app.is_searching {
        "Search Mode | Type to filter (title/tags) | Tab: Cycle collection | Enter: Done | Esc: Cancel"
            .to_string()
    } else if !app.search_query.is_empty() {
        format!(
            "Filtered: '{}' ({} papers) | Esc: Clear filter | /: Edit search | Tab/h/l: Panels | ?: Help | q: Quit",
            app.search_query,
            app.papers.len()
        )
    } else {
        match app.active_panel {
            ActivePanel::Collections => {
                "[Collections] h/l: Focus | j/k: Nav | a/A: New | r: Rename | d: Del | Tab/BackTab | ?: Help | q: Quit"
                    .to_string()
            }
            ActivePanel::Papers => {
                "[Papers] h/l: Focus | j/k: Nav | Enter: Open | S: Sort | t: TOC | a: Add | d: Del | Tab/BackTab | ?: Help | q: Quit"
                    .to_string()
            }
            ActivePanel::Details => {
                "[Details/TOC] h/l: Focus | j/k: Nav | Enter: Jump | t: Full | a/e/d: Edit | H/L: Tree | Tab/BackTab | ?: Help | q: Quit"
                    .to_string()
            }
        }
    };

    let status_content = format!(" {pending_indicator}{base_content}");
    let status_bar =
        Paragraph::new(status_content).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, area);
}
