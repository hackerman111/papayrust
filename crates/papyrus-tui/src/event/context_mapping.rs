use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use papyrus_core::{Action, Motion};

use crate::app::ActivePanel;

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
            KeyCode::Char('p') => return Some(Action::QuickOpenModalOpen),
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
        KeyCode::Char('p') => Some(Action::QuickOpenModalOpen),
        KeyCode::Char('W') | KeyCode::Char('z') => Some(Action::ToggleLayoutMode),
        KeyCode::Char('}') => Some(Action::Motion(Motion::HalfPageDown)),
        KeyCode::Char('{') => Some(Action::Motion(Motion::HalfPageUp)),
        KeyCode::Char('J') => match active_panel {
            ActivePanel::Collections | ActivePanel::Papers => {
                Some(Action::Motion(Motion::HalfPageDown))
            }
            _ => None,
        },
        KeyCode::Char('K') => match active_panel {
            ActivePanel::Collections | ActivePanel::Papers => {
                Some(Action::Motion(Motion::HalfPageUp))
            }
            _ => None,
        },
        KeyCode::Char('c') => match active_panel {
            ActivePanel::Papers => Some(Action::CollectionMembershipModalOpen),
            _ => None,
        },
        KeyCode::Char('S') => Some(Action::CyclePaperSort),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveDown),
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('/') | KeyCode::Char('s') => Some(Action::Search),
        KeyCode::Char('e') => Some(Action::EditMetadata),
        KeyCode::Esc => Some(Action::SearchCancel),
        KeyCode::Char('?') => Some(Action::HelpModalToggle),
        KeyCode::Char('t') => match active_panel {
            ActivePanel::Papers => Some(Action::TagModalOpen),
            ActivePanel::Details => Some(Action::OpenFullscreenToc),
            _ => None,
        },
        KeyCode::Char('V') => match active_panel {
            ActivePanel::Papers => Some(Action::VisualModeToggle),
            _ => None,
        },
        KeyCode::Char('T') => match active_panel {
            ActivePanel::Papers => Some(Action::EditTagsModalOpen),
            _ => None,
        },
        KeyCode::Char('d') => match active_panel {
            ActivePanel::Collections => Some(Action::DeleteConfirmOpen),
            ActivePanel::Papers => Some(Action::RemoveFromCollection),
            _ => None,
        },
        KeyCode::Char('D') => match active_panel {
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
