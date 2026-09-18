use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use papyrus_core::{Action, Motion};
use ratatui::backend::Backend;
use ratatui::Terminal;

use crate::app::{ActivePanel, App};
use crate::ui;

/// Maps a crossterm KeyEvent to an application Action in the context of the running App.
pub fn map_key_event_for_app(key: KeyEvent, app: &App) -> Option<Action> {
    if app.is_editing_metadata {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
                KeyCode::Char('s') => return Some(Action::EditMetadataSave),
                _ => return None,
            }
        }
        match key.code {
            KeyCode::Esc => return Some(Action::EditMetadataCancel),
            KeyCode::Enter => return Some(Action::EditMetadataSave),
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    return Some(Action::EditMetadataPrevField);
                } else {
                    return Some(Action::EditMetadataNextField);
                }
            }
            KeyCode::BackTab => return Some(Action::EditMetadataPrevField),
            KeyCode::Down => return Some(Action::EditMetadataNextField),
            KeyCode::Up => return Some(Action::EditMetadataPrevField),
            KeyCode::Backspace => return Some(Action::EditMetadataBackspace),
            KeyCode::Char(c) => return Some(Action::EditMetadataInput(c)),
            _ => return None,
        }
    }

    if app.is_editing_toc() {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
                KeyCode::Char('s') => return Some(Action::TocEditModalSave),
                _ => return None,
            }
        }
        match key.code {
            KeyCode::Esc => return Some(Action::TocEditModalCancel),
            KeyCode::Enter => return Some(Action::TocEditModalSave),
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    return Some(Action::TocEditModalPrevField);
                } else {
                    return Some(Action::TocEditModalNextField);
                }
            }
            KeyCode::BackTab => return Some(Action::TocEditModalPrevField),
            KeyCode::Down => return Some(Action::TocEditModalNextField),
            KeyCode::Up => return Some(Action::TocEditModalPrevField),
            KeyCode::Backspace => return Some(Action::TocEditModalBackspace),
            KeyCode::Char(c) => return Some(Action::TocEditModalInput(c)),
            _ => return None,
        }
    }

    if app.is_importing_toc() {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
                _ => return None,
            }
        }

        let active_field = app
            .toc_import_state
            .as_ref()
            .map(|s| s.active_field)
            .unwrap_or(0);

        match key.code {
            KeyCode::Esc => return Some(Action::TocImportModalCancel),
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    return Some(Action::TocImportModalPrevField);
                } else {
                    return Some(Action::TocImportModalNextField);
                }
            }
            KeyCode::BackTab => return Some(Action::TocImportModalPrevField),
            KeyCode::Down => return Some(Action::TocImportModalNextField),
            KeyCode::Up => return Some(Action::TocImportModalPrevField),
            KeyCode::Right if active_field != 0 => return Some(Action::TocImportModalNextField),
            KeyCode::Left if active_field != 0 => return Some(Action::TocImportModalPrevField),

            // Field 0: Source selection
            KeyCode::Char(' ') if active_field == 0 => {
                return Some(Action::TocImportModalToggleSource);
            }
            KeyCode::Left | KeyCode::Right if active_field == 0 => {
                return Some(Action::TocImportModalToggleSource);
            }

            // Field 1: File path input
            KeyCode::Backspace if active_field == 1 => {
                return Some(Action::TocImportModalBackspace);
            }
            KeyCode::Char(c) if active_field == 1 => {
                return Some(Action::TocImportModalInput(c));
            }

            // Field 2: Merge toggle
            KeyCode::Char(' ') if active_field == 2 => {
                return Some(Action::TocImportModalToggleMerge);
            }

            // Enter confirms import (common Enter across fields)
            KeyCode::Enter => return Some(Action::TocImportModalConfirm),
            _ => return None,
        }
    }

    if app.is_searching {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
                _ => {}
            }
        }
        match key.code {
            KeyCode::Esc => return Some(Action::SearchCancel),
            KeyCode::Enter => return Some(Action::SearchConfirm),
            KeyCode::Tab => return Some(Action::SearchCycleCollection),
            KeyCode::Backspace => return Some(Action::SearchBackspace),
            KeyCode::Char(c) => return Some(Action::SearchInput(c)),
            _ => return None,
        }
    }

    if app.is_adding_paper {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
                _ => return None,
            }
        }
        match key.code {
            KeyCode::Esc => return Some(Action::AddPaperModalCancel),
            KeyCode::Enter => return Some(Action::AddPaperModalConfirm),
            KeyCode::Tab => return Some(Action::AddPaperModalAutocomplete),
            KeyCode::Backspace => return Some(Action::AddPaperModalBackspace),
            KeyCode::Char(c) => return Some(Action::AddPaperModalInput(c)),
            _ => return None,
        }
    }

    if app.is_showing_help {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
                _ => {}
            }
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Enter => {
                return Some(Action::HelpModalToggle);
            }
            _ => return None,
        }
    }

    if app.is_confirming_delete {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
                _ => {}
            }
        }
        match key.code {
            KeyCode::Enter => return Some(Action::DeleteConfirmExecute),
            KeyCode::Esc | KeyCode::Char('q') => return Some(Action::DeleteConfirmCancel),
            _ => return None,
        }
    }

    if app.is_editing_tags {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
                _ => return None,
            }
        }
        match key.code {
            KeyCode::Esc => return Some(Action::EditTagsModalCancel),
            KeyCode::Enter => return Some(Action::EditTagsModalConfirm),
            KeyCode::Backspace => return Some(Action::EditTagsModalBackspace),
            KeyCode::Char(c) => return Some(Action::EditTagsModalInput(c)),
            _ => return None,
        }
    }

    if app.is_viewing_fullscreen_toc {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
                KeyCode::Char('d') => return Some(Action::Motion(Motion::HalfPageDown)),
                KeyCode::Char('u') => return Some(Action::Motion(Motion::HalfPageUp)),
                _ => {}
            }
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => return Some(Action::MoveUp),
            KeyCode::Down | KeyCode::Char('j') => return Some(Action::MoveDown),
            KeyCode::Char('g') => {
                if app.pending_chord == Some('g') {
                    if let Some(count) = app.pending_count {
                        return Some(Action::Motion(Motion::Absolute(count)));
                    } else {
                        return Some(Action::Motion(Motion::First));
                    }
                } else {
                    return Some(Action::PendingChord('g'));
                }
            }
            KeyCode::Char('G') => {
                if let Some(count) = app.pending_count {
                    return Some(Action::Motion(Motion::Absolute(count)));
                } else {
                    return Some(Action::Motion(Motion::Last));
                }
            }
            KeyCode::Char('0') => {
                if app.pending_count.is_some() {
                    return Some(Action::CountDigit(0));
                } else {
                    return Some(Action::Motion(Motion::First));
                }
            }
            KeyCode::Char(c @ '1'..='9') => {
                let digit = c.to_digit(10).unwrap() as usize;
                return Some(Action::CountDigit(digit));
            }
            KeyCode::Enter => {
                let page = app.current_toc().map(|t| t.page_number);
                return page.map(Action::OpenAtPage);
            }
            KeyCode::Esc => {
                if app.pending_count.is_some() || app.pending_chord.is_some() {
                    return Some(Action::ResetNavigationState);
                }
                return Some(Action::CloseFullscreenToc);
            }
            KeyCode::Char('q') | KeyCode::Char('t') => {
                return Some(Action::CloseFullscreenToc);
            }
            KeyCode::Char('?') => return Some(Action::HelpModalToggle),
            _ => return None,
        }
    }

    if app.is_creating_collection {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
                _ => return None,
            }
        }
        match key.code {
            KeyCode::Esc => return Some(Action::CreateCollectionModalCancel),
            KeyCode::Enter => return Some(Action::CreateCollectionModalConfirm),
            KeyCode::Backspace => return Some(Action::CreateCollectionModalBackspace),
            KeyCode::Char(c) => return Some(Action::CreateCollectionModalInput(c)),
            _ => return None,
        }
    }

    if app.is_renaming_collection {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
                _ => return None,
            }
        }
        match key.code {
            KeyCode::Esc => return Some(Action::RenameCollectionModalCancel),
            KeyCode::Enter => return Some(Action::RenameCollectionModalConfirm),
            KeyCode::Backspace => return Some(Action::RenameCollectionModalBackspace),
            KeyCode::Char(c) => return Some(Action::RenameCollectionModalInput(c)),
            _ => return None,
        }
    }

    if app.is_exporting_collection {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
                _ => return None,
            }
        }
        match key.code {
            KeyCode::Esc => return Some(Action::ExportCollectionModalCancel),
            KeyCode::Enter => return Some(Action::ExportCollectionModalConfirm),
            KeyCode::Tab => return Some(Action::ExportCollectionModalAutocomplete),
            KeyCode::Backspace => return Some(Action::ExportCollectionModalBackspace),
            KeyCode::Char(c) => return Some(Action::ExportCollectionModalInput(c)),
            _ => return None,
        }
    }

    if app.is_importing_metadata {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
                _ => return None,
            }
        }
        match key.code {
            KeyCode::Esc => return Some(Action::ImportMetadataModalCancel),
            KeyCode::Enter => return Some(Action::ImportMetadataModalConfirm),
            KeyCode::Tab => return Some(Action::ImportMetadataModalAutocomplete),
            KeyCode::Backspace => return Some(Action::ImportMetadataModalBackspace),
            KeyCode::Char(c) => return Some(Action::ImportMetadataModalInput(c)),
            _ => return None,
        }
    }

    if key.code == KeyCode::Esc {
        if app.pending_count.is_some() || app.pending_chord.is_some() {
            return Some(Action::ResetNavigationState);
        }
        if !app.search_query.is_empty() {
            return Some(Action::SearchCancel);
        }
        if app.layout_mode == crate::app::LayoutMode::SinglePanel {
            return Some(Action::ToggleLayoutMode);
        }
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
            KeyCode::Char('w') => return Some(Action::ToggleLayoutMode),
            KeyCode::Char('d') => return Some(Action::Motion(Motion::HalfPageDown)),
            KeyCode::Char('u') => return Some(Action::Motion(Motion::HalfPageUp)),
            _ => {}
        }
    }

    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('g') => {
                if app.pending_chord == Some('g') {
                    if let Some(count) = app.pending_count {
                        return Some(Action::Motion(Motion::Absolute(count)));
                    } else {
                        return Some(Action::Motion(Motion::First));
                    }
                } else {
                    return Some(Action::PendingChord('g'));
                }
            }
            KeyCode::Char('G') => {
                if let Some(count) = app.pending_count {
                    return Some(Action::Motion(Motion::Absolute(count)));
                } else {
                    return Some(Action::Motion(Motion::Last));
                }
            }
            KeyCode::Char('0') => {
                if app.pending_count.is_some() {
                    return Some(Action::CountDigit(0));
                } else {
                    return Some(Action::Motion(Motion::First));
                }
            }
            KeyCode::Char(c @ '1'..='9') => {
                let digit = c.to_digit(10).unwrap() as usize;
                return Some(Action::CountDigit(digit));
            }
            _ => {}
        }
    }

    if app.active_panel == ActivePanel::Details {
        let current_toc_id = app.current_toc().map(|t| t.id);
        match key.code {
            KeyCode::Char('a') => return Some(Action::TocAddEntry { parent_id: None }),
            KeyCode::Char('A') => {
                return Some(Action::TocAddEntry {
                    parent_id: current_toc_id,
                })
            }
            KeyCode::Char('e') => {
                if let Some(id) = current_toc_id {
                    return Some(Action::TocEditEntry { id });
                }
            }
            KeyCode::Char('d') => {
                if let Some(id) = current_toc_id {
                    return Some(Action::TocDeleteEntry { id });
                }
            }
            KeyCode::Char('H') => {
                if let Some(id) = current_toc_id {
                    return Some(Action::TocOutdent { id });
                }
            }
            KeyCode::Char('L') => {
                if let Some(id) = current_toc_id {
                    return Some(Action::TocIndent { id });
                }
            }
            KeyCode::Char('K') => {
                if let Some(id) = current_toc_id {
                    return Some(Action::TocMoveUp { id });
                }
            }
            KeyCode::Char('J') => {
                if let Some(id) = current_toc_id {
                    return Some(Action::TocMoveDown { id });
                }
            }
            KeyCode::Char('E') => return Some(Action::TocEmbed),
            KeyCode::Char('i') => return Some(Action::TocImportModalOpen),
            _ => {}
        }
    }

    let selected_toc_page = if app.active_panel == ActivePanel::Details {
        app.current_toc().map(|t| t.page_number)
    } else {
        None
    };
    map_key_event_with_context(key, app.active_panel, selected_toc_page)
}

/// Maps a crossterm KeyEvent to an application Action given the active panel and optional selected TOC page.
pub fn map_key_event_with_context(
    key: KeyEvent,
    active_panel: ActivePanel,
    selected_toc_page: Option<u32>,
) -> Option<Action> {
    // Handle Ctrl combinations
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
            KeyCode::Char('w') => return Some(Action::ToggleLayoutMode),
            KeyCode::Char('d') => return Some(Action::Motion(Motion::HalfPageDown)),
            KeyCode::Char('u') => return Some(Action::Motion(Motion::HalfPageUp)),
            _ => {}
        }
    }

    match key.code {
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                Some(Action::PreviousPanel)
            } else {
                Some(Action::NextPanel)
            }
        }
        KeyCode::BackTab => Some(Action::PreviousPanel),
        KeyCode::Char('h') => Some(Action::PanelLeft),
        KeyCode::Char('l') => Some(Action::PanelRight),
        KeyCode::Char('S') => Some(Action::CyclePaperSort),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveDown),
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('/') | KeyCode::Char('s') => Some(Action::Search),
        KeyCode::Char('e') => Some(Action::EditMetadata),
        KeyCode::Esc => Some(Action::SearchCancel),
        KeyCode::Char('?') => Some(Action::HelpModalToggle),
        KeyCode::Char('t') => match active_panel {
            ActivePanel::Papers | ActivePanel::Details => Some(Action::OpenFullscreenToc),
            _ => None,
        },
        KeyCode::Char('T') => match active_panel {
            ActivePanel::Papers => Some(Action::EditTagsModalOpen),
            _ => None,
        },
        KeyCode::Char('d') => match active_panel {
            ActivePanel::Collections | ActivePanel::Papers => Some(Action::DeleteConfirmOpen),
            _ => None,
        },
        KeyCode::Char('o') => match active_panel {
            ActivePanel::Papers => Some(Action::Open),
            _ => None,
        },
        KeyCode::Char('a') => match active_panel {
            ActivePanel::Papers => Some(Action::AddPaperModalOpen),
            ActivePanel::Collections => Some(Action::CreateCollectionModalOpen),
            _ => None,
        },
        KeyCode::Enter => match active_panel {
            ActivePanel::Papers => Some(Action::Open),
            ActivePanel::Details => selected_toc_page.map(Action::OpenAtPage),
            ActivePanel::Collections => Some(Action::FocusPapers),
        },
        KeyCode::Char('A') => match active_panel {
            ActivePanel::Collections => Some(Action::CreateSubcollectionModalOpen),
            _ => None,
        },
        KeyCode::Char('r') => match active_panel {
            ActivePanel::Collections => Some(Action::RenameCollectionModalOpen),
            _ => None,
        },
        KeyCode::Char('E') => match active_panel {
            ActivePanel::Collections => Some(Action::ExportCollectionModalOpen),
            _ => None,
        },
        KeyCode::Char('m') => match active_panel {
            ActivePanel::Papers => Some(Action::ImportMetadataModalOpen),
            _ => None,
        },
        KeyCode::Char('0') => Some(Action::Motion(Motion::First)),
        KeyCode::Char(c @ '1'..='9') => Some(Action::CountDigit(c.to_digit(10).unwrap() as usize)),
        KeyCode::Char('G') => Some(Action::Motion(Motion::Last)),
        _ => None,
    }
}

/// Maps a crossterm KeyEvent to an application Action using default Papers panel context.
pub fn map_key_event(key: KeyEvent) -> Option<Action> {
    map_key_event_with_context(key, ActivePanel::Papers, None)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_key_event_navigation() {
        let tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(map_key_event(tab), Some(Action::NextPanel));

        let backtab = KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE);
        assert_eq!(map_key_event(backtab), Some(Action::PreviousPanel));

        let shift_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT);
        assert_eq!(map_key_event(shift_tab), Some(Action::PreviousPanel));

        let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(map_key_event(up), Some(Action::MoveUp));

        let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        assert_eq!(map_key_event(k), Some(Action::MoveUp));

        let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(map_key_event(down), Some(Action::MoveDown));

        let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(map_key_event(j), Some(Action::MoveDown));
    }

    #[test]
    fn test_map_key_event_quit() {
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(map_key_event(q), Some(Action::Quit));

        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(map_key_event(ctrl_c), Some(Action::Quit));

        let ctrl_q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert_eq!(map_key_event(ctrl_q), Some(Action::Quit));
    }

    #[test]
    fn test_map_key_event_unhandled() {
        let x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        assert_eq!(map_key_event(x), None);
    }

    #[test]
    fn test_map_key_event_open_in_papers_panel() {
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            map_key_event_with_context(enter, ActivePanel::Papers, None),
            Some(Action::Open)
        );

        let o_key = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE);
        assert_eq!(
            map_key_event_with_context(o_key, ActivePanel::Papers, None),
            Some(Action::Open)
        );

        // In collections panel, 'o' is not handled
        assert_eq!(
            map_key_event_with_context(o_key, ActivePanel::Collections, None),
            None
        );
    }

    #[test]
    fn test_map_key_event_open_at_page_in_details_panel() {
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            map_key_event_with_context(enter, ActivePanel::Details, Some(42)),
            Some(Action::OpenAtPage(42))
        );

        assert_eq!(
            map_key_event_with_context(enter, ActivePanel::Details, None),
            None
        );
    }

    #[test]
    fn test_map_key_event_search_trigger_in_normal_mode() {
        let slash = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
        assert_eq!(map_key_event(slash), Some(Action::Search));

        let s_key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(map_key_event(s_key), Some(Action::Search));
    }

    #[test]
    fn test_map_key_event_for_app_in_search_mode() {
        let mut app = App::new();
        app.is_searching = true;

        // Normal character
        let a_key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(
            map_key_event_for_app(a_key, &app),
            Some(Action::SearchInput('a'))
        );

        // Even 's' or '/' become input characters
        let s_key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(
            map_key_event_for_app(s_key, &app),
            Some(Action::SearchInput('s'))
        );

        let slash_key = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
        assert_eq!(
            map_key_event_for_app(slash_key, &app),
            Some(Action::SearchInput('/'))
        );

        // Backspace
        let bs_key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(
            map_key_event_for_app(bs_key, &app),
            Some(Action::SearchBackspace)
        );

        // Enter confirms
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            map_key_event_for_app(enter, &app),
            Some(Action::SearchConfirm)
        );

        // Esc cancels
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(map_key_event_for_app(esc, &app), Some(Action::SearchCancel));

        // Ctrl+c quits
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(map_key_event_for_app(ctrl_c, &app), Some(Action::Quit));
    }

    #[test]
    fn test_map_key_event_edit_metadata_trigger() {
        let e_key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
        assert_eq!(map_key_event(e_key), Some(Action::EditMetadata));
    }

    #[test]
    fn test_map_key_event_for_app_in_edit_metadata_mode() {
        let mut app = App::new();
        app.is_editing_metadata = true;

        // Characters are input to the field
        let a_key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(
            map_key_event_for_app(a_key, &app),
            Some(Action::EditMetadataInput('a'))
        );

        // Tab moves to next field
        let tab_key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(
            map_key_event_for_app(tab_key, &app),
            Some(Action::EditMetadataNextField)
        );

        // Down moves to next field
        let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(
            map_key_event_for_app(down_key, &app),
            Some(Action::EditMetadataNextField)
        );

        // BackTab and Shift+Tab move to previous field
        let backtab_key = KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE);
        assert_eq!(
            map_key_event_for_app(backtab_key, &app),
            Some(Action::EditMetadataPrevField)
        );

        let shift_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT);
        assert_eq!(
            map_key_event_for_app(shift_tab, &app),
            Some(Action::EditMetadataPrevField)
        );

        // Up moves to previous field
        let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(
            map_key_event_for_app(up_key, &app),
            Some(Action::EditMetadataPrevField)
        );

        // Backspace
        let bs_key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(
            map_key_event_for_app(bs_key, &app),
            Some(Action::EditMetadataBackspace)
        );

        // Enter saves
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            map_key_event_for_app(enter, &app),
            Some(Action::EditMetadataSave)
        );

        // Ctrl+s saves
        let ctrl_s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(
            map_key_event_for_app(ctrl_s, &app),
            Some(Action::EditMetadataSave)
        );

        // Esc cancels
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(
            map_key_event_for_app(esc, &app),
            Some(Action::EditMetadataCancel)
        );

        // Ctrl+c quits
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(map_key_event_for_app(ctrl_c, &app), Some(Action::Quit));
    }

    #[test]
    fn test_map_key_event_details_toc_operations() {
        let mut app = App::new();
        app.active_panel = ActivePanel::Details;

        let toc_id = uuid::Uuid::now_v7();
        let toc = papyrus_core::db::TocEntry {
            id: toc_id,
            paper_id: uuid::Uuid::now_v7(),
            parent_id: None,
            title: "Section 1".to_string(),
            page_number: 10,
            order_index: 0,
            source: papyrus_core::db::TocSource::Manual,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        app.toc_preview = vec![toc];
        app.selected_toc = 0;

        // 'a' -> Add top-level TOC
        assert_eq!(
            map_key_event_for_app(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE), &app),
            Some(Action::TocAddEntry { parent_id: None })
        );

        // 'A' -> Add child TOC
        assert_eq!(
            map_key_event_for_app(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE), &app),
            Some(Action::TocAddEntry {
                parent_id: Some(toc_id)
            })
        );

        // 'e' -> Edit selected TOC
        assert_eq!(
            map_key_event_for_app(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE), &app),
            Some(Action::TocEditEntry { id: toc_id })
        );

        // 'd' -> Delete selected TOC
        assert_eq!(
            map_key_event_for_app(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE), &app),
            Some(Action::TocDeleteEntry { id: toc_id })
        );

        // 'H' -> Outdent
        assert_eq!(
            map_key_event_for_app(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::NONE), &app),
            Some(Action::TocOutdent { id: toc_id })
        );

        // 'L' -> Indent
        assert_eq!(
            map_key_event_for_app(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::NONE), &app),
            Some(Action::TocIndent { id: toc_id })
        );

        // 'K' -> Move Up
        assert_eq!(
            map_key_event_for_app(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE), &app),
            Some(Action::TocMoveUp { id: toc_id })
        );

        // 'J' -> Move Down
        assert_eq!(
            map_key_event_for_app(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::NONE), &app),
            Some(Action::TocMoveDown { id: toc_id })
        );

        // 'E' -> Embed in PDF
        assert_eq!(
            map_key_event_for_app(KeyEvent::new(KeyCode::Char('E'), KeyModifiers::NONE), &app),
            Some(Action::TocEmbed)
        );

        // 'i' -> Open TOC import
        assert_eq!(
            map_key_event_for_app(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE), &app),
            Some(Action::TocImportModalOpen)
        );

        // Enter -> Open at page
        assert_eq!(
            map_key_event_for_app(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &app),
            Some(Action::OpenAtPage(10))
        );
    }
}
