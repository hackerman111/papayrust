use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use papyrus_core::db::{TocRepo, TocSource};
use papyrus_core::toc::{add_entry, export_to_json, import_toc, parse_json, parse_text};
use papyrus_core::{Action, TocImportSource};
use papyrus_tui::{map_key_event_for_app, App};
use tempfile::tempdir;

use super::helpers::{create_synthetic_pdf, insert_test_paper, setup_db};

/// RT-43: Preorder depth-first traversal of complex multi-root tree on DB retrieval, JSON export, and import roundtrip.
#[test]
fn test_rt_43_complex_tree_preorder_depth_first_roundtrip() {
    let mut conn = setup_db();
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("paper.pdf");
    create_synthetic_pdf(&pdf_path, 10);
    let paper = insert_test_paper(&conn, &pdf_path);

    // Build tree:
    // Root 1 (p1)
    //   -> Section 1.1 (p2)
    //        -> Subsection 1.1.1 (p3)
    //   -> Section 1.2 (p4)
    // Root 2 (p5)
    //   -> Section 2.1 (p6)
    // Root 3 (p7)
    let r1 = add_entry(
        &mut conn,
        paper.id,
        None,
        "Root 1".to_string(),
        1,
        TocSource::Manual,
    )
    .unwrap();
    let s1_1 = add_entry(
        &mut conn,
        paper.id,
        Some(r1.id),
        "Section 1.1".to_string(),
        2,
        TocSource::Manual,
    )
    .unwrap();
    let _sub1_1_1 = add_entry(
        &mut conn,
        paper.id,
        Some(s1_1.id),
        "Subsection 1.1.1".to_string(),
        3,
        TocSource::Manual,
    )
    .unwrap();
    let _s1_2 = add_entry(
        &mut conn,
        paper.id,
        Some(r1.id),
        "Section 1.2".to_string(),
        4,
        TocSource::Manual,
    )
    .unwrap();

    let r2 = add_entry(
        &mut conn,
        paper.id,
        None,
        "Root 2".to_string(),
        5,
        TocSource::Manual,
    )
    .unwrap();
    let _s2_1 = add_entry(
        &mut conn,
        paper.id,
        Some(r2.id),
        "Section 2.1".to_string(),
        6,
        TocSource::Manual,
    )
    .unwrap();

    let _r3 = add_entry(
        &mut conn,
        paper.id,
        None,
        "Root 3".to_string(),
        7,
        TocSource::Manual,
    )
    .unwrap();

    // 1. Verify TocRepo::get_by_paper returns preorder depth-first sequence
    let entries = TocRepo::get_by_paper(&conn, paper.id).unwrap();
    let titles: Vec<&str> = entries.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(
        titles,
        vec![
            "Root 1",
            "Section 1.1",
            "Subsection 1.1.1",
            "Section 1.2",
            "Root 2",
            "Section 2.1",
            "Root 3"
        ]
    );

    // 2. Export to JSON and verify levels
    let json_str = export_to_json(&entries).expect("export to json");
    let pending = parse_json(&json_str).expect("parse exported json");
    assert_eq!(pending.len(), 7);
    assert_eq!(pending[0].title, "Root 1");
    assert_eq!(pending[0].level, 0);
    assert_eq!(pending[1].title, "Section 1.1");
    assert_eq!(pending[1].level, 1);
    assert_eq!(pending[2].title, "Subsection 1.1.1");
    assert_eq!(pending[2].level, 2);
    assert_eq!(pending[3].title, "Section 1.2");
    assert_eq!(pending[3].level, 1);
    assert_eq!(pending[4].title, "Root 2");
    assert_eq!(pending[4].level, 0);
    assert_eq!(pending[5].title, "Section 2.1");
    assert_eq!(pending[5].level, 1);
    assert_eq!(pending[6].title, "Root 3");
    assert_eq!(pending[6].level, 0);

    // 3. Re-import from exported JSON (replace mode) and verify full preservation
    let json_path = dir.path().join("export.json");
    fs::write(&json_path, &json_str).unwrap();

    let reimported = import_toc(
        &mut conn,
        paper.id,
        &pdf_path,
        &TocImportSource::JsonFile(json_path),
        false,
    )
    .expect("import from json");

    assert_eq!(reimported.len(), 7);
    let re_titles: Vec<&str> = reimported.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(titles, re_titles);

    // Check parent-child hierarchy in reimported entries
    let r1_re = &reimported[0];
    let s1_1_re = &reimported[1];
    let sub1_1_1_re = &reimported[2];
    let s1_2_re = &reimported[3];
    let r2_re = &reimported[4];
    let s2_1_re = &reimported[5];
    let r3_re = &reimported[6];

    assert_eq!(r1_re.parent_id, None);
    assert_eq!(s1_1_re.parent_id, Some(r1_re.id));
    assert_eq!(sub1_1_1_re.parent_id, Some(s1_1_re.id));
    assert_eq!(s1_2_re.parent_id, Some(r1_re.id));
    assert_eq!(r2_re.parent_id, None);
    assert_eq!(s2_1_re.parent_id, Some(r2_re.id));
    assert_eq!(r3_re.parent_id, None);

    // Verify subsequent get_by_paper also returns exact preorder
    let db_entries = TocRepo::get_by_paper(&conn, paper.id).unwrap();
    let db_titles: Vec<&str> = db_entries.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(db_titles, titles);
}

/// RT-44: Text import handles trailing whitespace and trailing tabs gracefully.
#[test]
fn test_rt_44_parse_text_trailing_spaces_and_tabs() {
    // 1. 4-space indented text with trailing spaces
    let raw_spaces = "Root 1 1   \n    Child 1.1 2   \nRoot 2 3 \n    Child 2.1 4  ";
    let entries = parse_text(raw_spaces).expect("parse spaces text with trailing spaces");
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].title, "Root 1");
    assert_eq!(entries[0].page, 1);
    assert_eq!(entries[0].level, 0);

    assert_eq!(entries[1].title, "Child 1.1");
    assert_eq!(entries[1].page, 2);
    assert_eq!(entries[1].level, 1);

    assert_eq!(entries[2].title, "Root 2");
    assert_eq!(entries[2].page, 3);
    assert_eq!(entries[2].level, 0);

    assert_eq!(entries[3].title, "Child 2.1");
    assert_eq!(entries[3].page, 4);
    assert_eq!(entries[3].level, 1);

    // 2. Tab indented text with trailing tabs/spaces
    let raw_tabs = "Root 1\t1   \n\tChild 1.1\t2 \t \nRoot 2\t3\t  \n\tChild 2.1\t4 \t";
    let entries_tabs = parse_text(raw_tabs).expect("parse tabs text with trailing tabs");
    assert_eq!(entries_tabs.len(), 4);
    assert_eq!(entries_tabs[0].title, "Root 1");
    assert_eq!(entries_tabs[0].page, 1);
    assert_eq!(entries_tabs[0].level, 0);

    assert_eq!(entries_tabs[1].title, "Child 1.1");
    assert_eq!(entries_tabs[1].page, 2);
    assert_eq!(entries_tabs[1].level, 1);
}

/// RT-45: TOC import modal key event mappings, field cycling, and inputs.
#[test]
fn test_rt_45_toc_import_modal_field_switching_and_inputs() {
    let conn = setup_db();
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("paper.pdf");
    create_synthetic_pdf(&pdf_path, 5);
    let _paper = insert_test_paper(&conn, &pdf_path);

    let mut app = App::from_db_conn(conn).unwrap();

    // Open import modal
    app.start_import_toc();
    assert!(app.is_importing_toc());
    assert_eq!(app.toc_import_state.as_ref().unwrap().active_field, 0);

    // Field 0: Source selection
    // Space, Left, Right toggle source
    let key_space = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(key_space, &app),
        Some(Action::TocImportModalToggleSource)
    );
    let key_right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(key_right, &app),
        Some(Action::TocImportModalToggleSource)
    );
    let key_left = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(key_left, &app),
        Some(Action::TocImportModalToggleSource)
    );

    // Tab or Down cycles to field 1 (Path)
    let key_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(key_tab, &app),
        Some(Action::TocImportModalNextField)
    );
    app.dispatch(Action::TocImportModalNextField);
    assert_eq!(app.toc_import_state.as_ref().unwrap().active_field, 1);

    // Field 1: Path input
    let key_char = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(key_char, &app),
        Some(Action::TocImportModalInput('t'))
    );
    app.dispatch(Action::TocImportModalInput('t'));
    app.dispatch(Action::TocImportModalInput('e'));
    app.dispatch(Action::TocImportModalInput('s'));
    app.dispatch(Action::TocImportModalInput('t'));
    assert_eq!(
        app.toc_import_state.as_ref().unwrap().file_path_buffer,
        "test"
    );

    let key_bs = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(key_bs, &app),
        Some(Action::TocImportModalBackspace)
    );
    app.dispatch(Action::TocImportModalBackspace);
    assert_eq!(
        app.toc_import_state.as_ref().unwrap().file_path_buffer,
        "tes"
    );

    // BackTab / Up / Left returns to field 0
    let key_up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(key_up, &app),
        Some(Action::TocImportModalPrevField)
    );
    app.dispatch(Action::TocImportModalPrevField);
    assert_eq!(app.toc_import_state.as_ref().unwrap().active_field, 0);

    // Move to field 2 (Merge) via BackTab from 0
    let key_backtab = KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(key_backtab, &app),
        Some(Action::TocImportModalPrevField)
    );
    app.dispatch(Action::TocImportModalPrevField);
    assert_eq!(app.toc_import_state.as_ref().unwrap().active_field, 2);

    // Field 2: Space toggles merge
    assert_eq!(
        map_key_event_for_app(key_space, &app),
        Some(Action::TocImportModalToggleMerge)
    );
    app.dispatch(Action::TocImportModalToggleMerge);
    assert!(app.toc_import_state.as_ref().unwrap().merge_mode);
    app.dispatch(Action::TocImportModalToggleMerge);
    assert!(!app.toc_import_state.as_ref().unwrap().merge_mode);

    // Enter confirms import from field 2
    let key_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(key_enter, &app),
        Some(Action::TocImportModalConfirm)
    );

    // Esc cancels modal
    let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(key_esc, &app),
        Some(Action::TocImportModalCancel)
    );
    app.dispatch(Action::TocImportModalCancel);
    assert!(!app.is_importing_toc());
}
