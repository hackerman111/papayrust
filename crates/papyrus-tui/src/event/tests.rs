use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use papyrus_core::{Action, Motion};

use crate::app::{ActivePanel, App};

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

#[test]
fn test_map_key_event_picker_modal() {
    let mut app = App::new();
    let item = crate::app::PickerItem::new(uuid::Uuid::now_v7(), "Item");
    let picker = crate::app::GenericPicker::new("Test Picker", vec![item], true);
    app.active_picker = Some(picker);

    // Esc cancels
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &app),
        Some(Action::PickerCancel)
    );

    // Enter confirms
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &app),
        Some(Action::PickerConfirm)
    );

    // Space toggles item in multi_select
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE), &app),
        Some(Action::PickerToggleItem)
    );

    // j / Down moves down
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE), &app),
        Some(Action::PickerMoveDown)
    );
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &app),
        Some(Action::PickerMoveDown)
    );

    // k / Up moves up
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE), &app),
        Some(Action::PickerMoveUp)
    );
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), &app),
        Some(Action::PickerMoveUp)
    );

    // PageDown / Ctrl-d
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE), &app),
        Some(Action::PickerPageDown)
    );
    assert_eq!(
        map_key_event_for_app(
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
            &app
        ),
        Some(Action::PickerPageDown)
    );

    // PageUp / Ctrl-u
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE), &app),
        Some(Action::PickerPageUp)
    );
    assert_eq!(
        map_key_event_for_app(
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
            &app
        ),
        Some(Action::PickerPageUp)
    );

    // Backspace
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE), &app),
        Some(Action::PickerBackspace)
    );

    // Character typing
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE), &app),
        Some(Action::PickerInput('x'))
    );
}

#[test]
fn test_map_key_event_picker_triggers_and_batch_delete() {
    let mut app = App::new();
    app.active_panel = ActivePanel::Papers;

    // Ctrl-p -> QuickOpen
    assert_eq!(
        map_key_event_for_app(
            KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
            &app
        ),
        Some(Action::QuickOpenModalOpen)
    );

    // 'c' in Papers panel -> CollectionMembership
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE), &app),
        Some(Action::CollectionMembershipModalOpen)
    );

    // 't' in Papers panel -> TagModalOpen
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE), &app),
        Some(Action::TagModalOpen)
    );

    // 't' in Details panel -> OpenFullscreenToc
    app.active_panel = ActivePanel::Details;
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE), &app),
        Some(Action::OpenFullscreenToc)
    );

    // Normal mode: 'd' removes from collection, 'D' deletes permanently from DB
    app.active_panel = ActivePanel::Papers;
    app.visual_mode = false;
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE), &app),
        Some(Action::RemoveFromCollection)
    );
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE), &app),
        Some(Action::DeleteConfirmOpen)
    );

    // Visual mode: 'd' batch removes from collection, 'D' / Delete batch deletes permanently
    app.visual_mode = true;
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE), &app),
        Some(Action::BatchRemoveFromCollection)
    );
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE), &app),
        Some(Action::BatchDeleteConfirm)
    );
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE), &app),
        Some(Action::BatchDeleteConfirm)
    );
}

#[test]
fn test_map_key_event_vim_analogs_and_chords() {
    let mut app = App::new();
    app.active_panel = ActivePanel::Papers;

    // 1. 'p' is vim analog for Ctrl-p (QuickOpen)
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE), &app),
        Some(Action::QuickOpenModalOpen)
    );

    // 2. 'W' and 'z' are vim analogs for Ctrl-w (ToggleLayoutMode)
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('W'), KeyModifiers::NONE), &app),
        Some(Action::ToggleLayoutMode)
    );
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE), &app),
        Some(Action::ToggleLayoutMode)
    );

    // 3. '}' and '{' are vim analogs for Ctrl-d / Ctrl-u (HalfPageDown / HalfPageUp)
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('}'), KeyModifiers::NONE), &app),
        Some(Action::Motion(Motion::HalfPageDown))
    );
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('{'), KeyModifiers::NONE), &app),
        Some(Action::Motion(Motion::HalfPageUp))
    );

    // 4. 'J' and 'K' in Papers and Collections panels map to HalfPageDown / HalfPageUp
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::NONE), &app),
        Some(Action::Motion(Motion::HalfPageDown))
    );
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE), &app),
        Some(Action::Motion(Motion::HalfPageUp))
    );

    app.active_panel = ActivePanel::Collections;
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::NONE), &app),
        Some(Action::Motion(Motion::HalfPageDown))
    );
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE), &app),
        Some(Action::Motion(Motion::HalfPageUp))
    );

    // 5. 'ZZ' and 'ZQ' quit chords
    app.active_panel = ActivePanel::Papers;
    assert_eq!(app.pending_chord, None);

    // First 'Z' -> PendingChord('Z')
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::NONE), &app),
        Some(Action::PendingChord('Z'))
    );
    app.pending_chord = Some('Z');

    // Second 'Z' -> Quit (ZZ)
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::NONE), &app),
        Some(Action::Quit)
    );

    // 'Z' followed by 'Q' -> Quit (ZQ)
    app.pending_chord = Some('Z');
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::NONE), &app),
        Some(Action::Quit)
    );

    // 'Z' followed by unrelated key -> ResetNavigationState
    app.pending_chord = Some('Z');
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE), &app),
        Some(Action::ResetNavigationState)
    );

    // 6. Picker '}' and '{' page scroll
    app.open_quick_open();
    assert!(app.active_picker.is_some());
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('}'), KeyModifiers::NONE), &app),
        Some(Action::PickerPageDown)
    );
    assert_eq!(
        map_key_event_for_app(KeyEvent::new(KeyCode::Char('{'), KeyModifiers::NONE), &app),
        Some(Action::PickerPageUp)
    );
}
