use super::helpers::{create_synthetic_pdf, insert_test_paper, setup_db};
use papyrus_core::db::{TocRepo, TocSource};
use papyrus_core::toc::{add_entry, delete_entry, indent, move_down, move_up, outdent};
use papyrus_core::Action;
use papyrus_tui::App;
use tempfile::tempdir;

/// RT-27: Adding top-level TOC entry via Action updates TUI state and database.
#[test]
fn test_rt_27_add_top_level_entry_via_action() {
    let conn = setup_db();
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("paper.pdf");
    create_synthetic_pdf(&pdf_path, 10);
    let paper = insert_test_paper(&conn, &pdf_path);

    let mut app = App::from_db_conn(conn).unwrap();
    assert_eq!(app.papers.len(), 1);
    assert_eq!(app.papers[0].id, paper.id);
    assert!(app.toc_preview.is_empty());

    // Trigger Add Entry
    app.dispatch(Action::TocAddEntry { parent_id: None });
    assert!(app.is_editing_toc());

    // Type title and page
    for c in "Introduction".chars() {
        app.dispatch(Action::TocEditModalInput(c));
    }
    app.dispatch(Action::TocEditModalNextField);
    app.dispatch(Action::TocEditModalBackspace); // Clear default "1"
    for c in "2".chars() {
        app.dispatch(Action::TocEditModalInput(c));
    }

    // Save
    app.dispatch(Action::TocEditModalSave);
    assert!(!app.is_editing_toc());
    assert_eq!(app.toc_preview.len(), 1);
    assert_eq!(app.toc_preview[0].title, "Introduction");
    assert_eq!(app.toc_preview[0].page_number, 2);
    assert_eq!(app.toc_preview[0].parent_id, None);
    assert_eq!(app.toc_preview[0].source, TocSource::Manual);

    // Verify in DB
    let db_tocs = TocRepo::get_by_paper(app.db_conn().unwrap(), paper.id).unwrap();
    assert_eq!(db_tocs.len(), 1);
    assert_eq!(db_tocs[0].title, "Introduction");
    assert_eq!(db_tocs[0].page_number, 2);
    assert_eq!(db_tocs[0].parent_id, None);
    assert_eq!(db_tocs[0].source, TocSource::Manual);
}

/// RT-28: Adding child TOC entry creates correct parent_id link.
#[test]
fn test_rt_28_add_child_entry() {
    let mut conn = setup_db();
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("paper.pdf");
    create_synthetic_pdf(&pdf_path, 10);
    let paper = insert_test_paper(&conn, &pdf_path);

    let root = add_entry(
        &mut conn,
        paper.id,
        None,
        "Chapter 1".to_string(),
        1,
        TocSource::Manual,
    )
    .unwrap();

    let mut app = App::from_db_conn(conn).unwrap();
    assert_eq!(app.toc_preview.len(), 1);

    // Add child to root
    app.dispatch(Action::TocAddEntry {
        parent_id: Some(root.id),
    });
    assert!(app.is_editing_toc());

    for c in "Section 1.1".chars() {
        app.dispatch(Action::TocEditModalInput(c));
    }
    app.dispatch(Action::TocEditModalNextField);
    app.dispatch(Action::TocEditModalBackspace);
    app.dispatch(Action::TocEditModalInput('3'));

    app.dispatch(Action::TocEditModalSave);
    assert_eq!(app.toc_preview.len(), 2);

    let child = app
        .toc_preview
        .iter()
        .find(|t| t.title == "Section 1.1")
        .unwrap();
    assert_eq!(child.parent_id, Some(root.id));
    assert_eq!(child.page_number, 3);
}

/// RT-29: Indent / Outdent re-parenting preserves grandchildren.
#[test]
fn test_rt_29_indent_outdent_hierarchy_preservation() {
    let mut conn = setup_db();
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("paper.pdf");
    create_synthetic_pdf(&pdf_path, 10);
    let paper = insert_test_paper(&conn, &pdf_path);

    // Root entries: A, B
    let a = add_entry(
        &mut conn,
        paper.id,
        None,
        "A".to_string(),
        1,
        TocSource::Manual,
    )
    .unwrap();
    let b = add_entry(
        &mut conn,
        paper.id,
        None,
        "B".to_string(),
        2,
        TocSource::Manual,
    )
    .unwrap();
    // Child under B: C (grandchild once B is indented under A)
    let c = add_entry(
        &mut conn,
        paper.id,
        Some(b.id),
        "C".to_string(),
        3,
        TocSource::Manual,
    )
    .unwrap();

    // 1. Indent B: B should become child of A; C should remain child of B
    let b_indented = indent(&mut conn, b.id).expect("indent B");
    assert_eq!(b_indented.parent_id, Some(a.id));

    let c_after_indent = TocRepo::get_by_id(&conn, c.id).unwrap().unwrap();
    assert_eq!(
        c_after_indent.parent_id,
        Some(b.id),
        "Grandchild C must remain child of B"
    );

    // 2. Outdent B: B should become root (parent_id: None); C must still be child of B
    let b_outdented = outdent(&mut conn, b.id).expect("outdent B");
    assert_eq!(b_outdented.parent_id, None);

    let c_after_outdent = TocRepo::get_by_id(&conn, c.id).unwrap().unwrap();
    assert_eq!(
        c_after_outdent.parent_id,
        Some(b.id),
        "Grandchild C must remain child of B"
    );
}

/// RT-30: Reordering via move_up/move_down (K/J) maintains contiguous order_index.
#[test]
fn test_rt_30_reorder_stability() {
    let mut conn = setup_db();
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("paper.pdf");
    create_synthetic_pdf(&pdf_path, 10);
    let paper = insert_test_paper(&conn, &pdf_path);

    let s1 = add_entry(
        &mut conn,
        paper.id,
        None,
        "S1".to_string(),
        1,
        TocSource::Manual,
    )
    .unwrap();
    let s2 = add_entry(
        &mut conn,
        paper.id,
        None,
        "S2".to_string(),
        2,
        TocSource::Manual,
    )
    .unwrap();
    let s3 = add_entry(
        &mut conn,
        paper.id,
        None,
        "S3".to_string(),
        3,
        TocSource::Manual,
    )
    .unwrap();

    assert_eq!(s1.order_index, 0);
    assert_eq!(s2.order_index, 1);
    assert_eq!(s3.order_index, 2);

    // Move S2 up (swap with S1)
    move_up(&mut conn, s2.id).unwrap();
    let entries = TocRepo::get_by_paper(&conn, paper.id).unwrap();
    assert_eq!(entries[0].id, s2.id);
    assert_eq!(entries[0].order_index, 0);
    assert_eq!(entries[1].id, s1.id);
    assert_eq!(entries[1].order_index, 1);
    assert_eq!(entries[2].id, s3.id);
    assert_eq!(entries[2].order_index, 2);

    // Move S2 down (swap back with S1)
    move_down(&mut conn, s2.id).unwrap();
    let entries_back = TocRepo::get_by_paper(&conn, paper.id).unwrap();
    assert_eq!(entries_back[0].id, s1.id);
    assert_eq!(entries_back[0].order_index, 0);
    assert_eq!(entries_back[1].id, s2.id);
    assert_eq!(entries_back[1].order_index, 1);
    assert_eq!(entries_back[2].id, s3.id);
    assert_eq!(entries_back[2].order_index, 2);
}

/// RT-31: Cascading deletion of subtree via foreign key / delete_entry.
#[test]
fn test_rt_31_cascading_subtree_deletion() {
    let mut conn = setup_db();
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("paper.pdf");
    create_synthetic_pdf(&pdf_path, 10);
    let paper = insert_test_paper(&conn, &pdf_path);

    // Tree: Root -> Child -> Grandchild, and Root2
    let root = add_entry(
        &mut conn,
        paper.id,
        None,
        "Root".to_string(),
        1,
        TocSource::Manual,
    )
    .unwrap();
    let child = add_entry(
        &mut conn,
        paper.id,
        Some(root.id),
        "Child".to_string(),
        2,
        TocSource::Manual,
    )
    .unwrap();
    let grandchild = add_entry(
        &mut conn,
        paper.id,
        Some(child.id),
        "Grandchild".to_string(),
        3,
        TocSource::Manual,
    )
    .unwrap();
    let root2 = add_entry(
        &mut conn,
        paper.id,
        None,
        "Root2".to_string(),
        4,
        TocSource::Manual,
    )
    .unwrap();

    // Delete Root
    delete_entry(&mut conn, root.id).expect("delete root");

    assert!(TocRepo::get_by_id(&conn, root.id).unwrap().is_none());
    assert!(TocRepo::get_by_id(&conn, child.id).unwrap().is_none());
    assert!(TocRepo::get_by_id(&conn, grandchild.id).unwrap().is_none());

    // Root2 must still exist
    assert!(TocRepo::get_by_id(&conn, root2.id).unwrap().is_some());
}
