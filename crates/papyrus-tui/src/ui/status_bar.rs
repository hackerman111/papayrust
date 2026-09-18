use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{ActivePanel, App};

/// Renders the status and keybindings help bar at the bottom of the screen.
pub fn render_status_bar(app: &App, frame: &mut Frame, area: Rect) {
    let status_content = if let Some(ref msg) = app.status_message {
        format!(" {msg} | Tab: Switch panel | ?: Help | q: Quit")
    } else if app.is_showing_help {
        " Help Mode | Esc / q / ?: Close Help".to_string()
    } else if app.is_viewing_fullscreen_toc {
        " Fullscreen TOC | j/k: Scroll | Enter: Jump to page | Esc/q/t: Exit TOC | ?: Help"
            .to_string()
    } else if app.is_confirming_delete {
        " Confirm Delete | Enter: Delete permanently | Esc / q: Cancel".to_string()
    } else if app.is_editing_tags {
        " Edit Tags | Enter: Save tags | Esc: Cancel".to_string()
    } else if app.is_searching {
        " Search Mode | Type to filter (title/tags) | Tab: Cycle collection | Enter: Done | Esc: Cancel"
            .to_string()
    } else if !app.search_query.is_empty() {
        format!(
            " Filtered: '{}' ({} papers) | Esc: Clear filter | /: Edit search | Tab: Switch panel | ?: Help | q: Quit",
            app.search_query,
            app.papers.len()
        )
    } else {
        match app.active_panel {
            ActivePanel::Collections => {
                " [Collections] j/k: Navigate | a: New folder | d: Delete | Tab/BackTab: Switch panel | /: Search | ?: Help | q: Quit"
                    .to_string()
            }
            ActivePanel::Papers => {
                " [Papers] j/k: Navigate | Enter/o: Open | t: Fullscreen TOC | T: Tags | a: Add | e: Edit | d: Delete | Tab/BackTab: Switch panel | /: Search | ?: Help | q: Quit"
                    .to_string()
            }
            ActivePanel::Details => {
                " [Details/TOC] j/k: Navigate | Enter: Jump to page | t: Fullscreen | a: Add | e: Edit | d: Delete | H/L: Indent | Tab/BackTab: Switch panel | ?: Help | q: Quit"
                    .to_string()
            }
        }
    };

    let status_bar =
        Paragraph::new(status_content).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, area);
}
