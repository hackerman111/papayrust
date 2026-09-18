pub mod modals;
pub mod panels;
pub mod search_bar;
pub mod status_bar;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::app::App;
use modals::{
    render_add_paper_modal, render_create_collection_modal, render_delete_confirm_modal,
    render_edit_tags_modal, render_export_collection_modal, render_help_modal,
    render_import_metadata_modal, render_metadata_modal, render_rename_collection_modal,
    render_toc_import_modal, render_toc_modal,
};
use panels::{render_collections, render_details, render_fullscreen_toc, render_papers};
use search_bar::render_search_bar;
use status_bar::render_status_bar;

/// Renders the complete 3-column TUI and status bar.
pub fn render(app: &App, frame: &mut Frame) {
    let show_search_bar = app.is_searching || !app.search_query.is_empty();

    let vertical_chunks = if show_search_bar {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),    // Main 3-panel area
                Constraint::Length(3), // Search input bar
                Constraint::Length(1), // Bottom status / help bar
            ])
            .split(frame.area())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),    // Main 3-panel area
                Constraint::Length(1), // Bottom status / help bar
            ])
            .split(frame.area())
    };

    if app.is_viewing_fullscreen_toc {
        render_fullscreen_toc(app, frame, vertical_chunks[0]);
    } else if app.layout_mode == crate::app::LayoutMode::SinglePanel {
        match app.active_panel {
            crate::app::ActivePanel::Collections => {
                render_collections(app, frame, vertical_chunks[0]);
            }
            crate::app::ActivePanel::Papers => {
                render_papers(app, frame, vertical_chunks[0]);
            }
            crate::app::ActivePanel::Details => {
                render_details(app, frame, vertical_chunks[0]);
            }
        }
    } else {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // Collections panel
                Constraint::Percentage(45), // Papers panel
                Constraint::Percentage(30), // Details / Metadata panel
            ])
            .split(vertical_chunks[0]);

        // 1. Render Collections Panel
        render_collections(app, frame, main_chunks[0]);

        // 2. Render Papers Panel
        render_papers(app, frame, main_chunks[1]);

        // 3. Render Details Panel
        render_details(app, frame, main_chunks[2]);
    }

    // 4. Render Search Bar (if visible)
    if show_search_bar {
        render_search_bar(app, frame, vertical_chunks[1]);
    }

    // 5. Render Status Bar
    let status_chunk = if show_search_bar {
        vertical_chunks[2]
    } else {
        vertical_chunks[1]
    };
    render_status_bar(app, frame, status_chunk);

    // 6. Floating Modals (if active)
    if app.is_creating_collection {
        render_create_collection_modal(app, frame);
    } else if app.is_renaming_collection {
        render_rename_collection_modal(app, frame);
    } else if app.is_exporting_collection {
        render_export_collection_modal(app, frame);
    } else if app.is_importing_metadata {
        render_import_metadata_modal(app, frame);
    } else if app.is_adding_paper {
        render_add_paper_modal(app, frame);
    } else if app.is_editing_metadata {
        render_metadata_modal(app, frame);
    } else if app.is_editing_toc() {
        render_toc_modal(app, frame);
    } else if app.is_importing_toc() {
        render_toc_import_modal(app, frame);
    } else if app.is_editing_tags {
        render_edit_tags_modal(app, frame);
    } else if app.is_confirming_delete {
        render_delete_confirm_modal(app, frame);
    } else if app.is_showing_help {
        render_help_modal(app, frame);
    }
}
