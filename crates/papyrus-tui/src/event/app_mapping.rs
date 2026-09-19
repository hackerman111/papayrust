use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use papyrus_core::{Action, Motion};

use super::context_mapping::map_key_event_with_context;
use crate::app::{ActivePanel, App};

/// Maps a crossterm KeyEvent to an application Action in the context of the running App.
pub fn map_key_event_for_app(key: KeyEvent, app: &App) -> Option<Action> {
    if let Some(ref picker) = app.active_picker {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Action::Quit),
                KeyCode::Char('d') => return Some(Action::PickerPageDown),
                KeyCode::Char('u') => return Some(Action::PickerPageUp),
                KeyCode::Char('j') | KeyCode::Char('n') => return Some(Action::PickerMoveDown),
                KeyCode::Char('k') | KeyCode::Char('p') => return Some(Action::PickerMoveUp),
                _ => return None,
            }
        }
        match key.code {
            KeyCode::Esc => return Some(Action::PickerCancel),
            KeyCode::Enter => return Some(Action::PickerConfirm),
            KeyCode::PageDown | KeyCode::Char('}') => return Some(Action::PickerPageDown),
            KeyCode::PageUp | KeyCode::Char('{') => return Some(Action::PickerPageUp),
            KeyCode::Down | KeyCode::Char('j') => return Some(Action::PickerMoveDown),
            KeyCode::Up | KeyCode::Char('k') => return Some(Action::PickerMoveUp),
            KeyCode::Char(' ') if picker.multi_select => return Some(Action::PickerToggleItem),
            KeyCode::Backspace => return Some(Action::PickerBackspace),
            KeyCode::Char(c) => return Some(Action::PickerInput(c)),
            _ => return None,
        }
    }

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
            KeyCode::Char('}') => return Some(Action::Motion(Motion::HalfPageDown)),
            KeyCode::Char('{') => return Some(Action::Motion(Motion::HalfPageUp)),
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

    if app.visual_mode && !key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char(' ') => return Some(Action::VisualModeToggleItem),
            KeyCode::Esc => return Some(Action::VisualModeCancel),
            KeyCode::Char('d') => return Some(Action::BatchRemoveFromCollection),
            KeyCode::Char('D') | KeyCode::Delete => return Some(Action::BatchDeleteConfirm),
            _ => {}
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
            KeyCode::Char('p') => return Some(Action::QuickOpenModalOpen),
            KeyCode::Char('w') => return Some(Action::ToggleLayoutMode),
            KeyCode::Char('d') => return Some(Action::Motion(Motion::HalfPageDown)),
            KeyCode::Char('u') => return Some(Action::Motion(Motion::HalfPageUp)),
            _ => {}
        }
    }

    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        if app.pending_chord == Some('Z') {
            match key.code {
                KeyCode::Char('Z') | KeyCode::Char('Q') => return Some(Action::Quit),
                _ => return Some(Action::ResetNavigationState),
            }
        }
        if key.code == KeyCode::Char('Z') {
            return Some(Action::PendingChord('Z'));
        }

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
