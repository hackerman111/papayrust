use std::fs;

use papyrus_core::db::{TocRepo, TocSource};
use papyrus_core::toc::{
    add_entry, export_to_json, import_toc, parse_json, parse_text, TocError, TocImportError,
};
use papyrus_core::TocImportSource;
use tempfile::tempdir;

use super::helpers::{
    create_synthetic_pdf, create_synthetic_pdf_with_outline, insert_test_paper, setup_db,
};

/// RT-37: Text import with tabs and 4-space indentation.
#[test]
fn test_rt_37_text_import_tabs_and_spaces() {
    // 1. Tab indentation
    let tab_text = "Introduction\t1\n\tBackground\t2\n\tRelated Work\t3\nMethods\t4";
    let entries_tab = parse_text(tab_text).expect("parse tab text");
    assert_eq!(entries_tab.len(), 4);
    assert_eq!(entries_tab[0].title, "Introduction");
    assert_eq!(entries_tab[0].level, 0);
    assert_eq!(entries_tab[1].title, "Background");
    assert_eq!(entries_tab[1].level, 1);
    assert_eq!(entries_tab[2].title, "Related Work");
    assert_eq!(entries_tab[2].level, 1);
    assert_eq!(entries_tab[3].title, "Methods");
    assert_eq!(entries_tab[3].level, 0);

    // 2. 4-space indentation
    let space_text = "Introduction 1\n    Background 2\n    Related Work 3\nMethods 4";
    let entries_space = parse_text(space_text).expect("parse space text");
    assert_eq!(entries_space.len(), 4);
    assert_eq!(entries_space[0].level, 0);
    assert_eq!(entries_space[1].level, 1);
    assert_eq!(entries_space[2].level, 1);
    assert_eq!(entries_space[3].level, 0);
}

/// RT-38: JSON import produces identical entries to text import.
#[test]
fn test_rt_38_json_import_equivalence() {
    let text = "Intro\t1\n\tDetails\t5";
    let pending_from_text = parse_text(text).unwrap();

    let json_text = serde_json::to_string(&pending_from_text).unwrap();
    let pending_from_json = parse_json(&json_text).unwrap();

    assert_eq!(pending_from_text.len(), pending_from_json.len());
    for (t, j) in pending_from_text.iter().zip(pending_from_json.iter()) {
        assert_eq!(t.title, j.title);
        assert_eq!(t.page, j.page);
        assert_eq!(t.level, j.level);
    }
}

/// RT-39: Import with page > page_count is rejected and transaction is rolled back.
#[test]
fn test_rt_39_page_out_of_bounds_rollback() {
    let mut conn = setup_db();
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("paper.pdf");
    create_synthetic_pdf(&pdf_path, 5); // 5 pages
    let paper = insert_test_paper(&conn, &pdf_path);

    // Initial entry
    add_entry(
        &mut conn,
        paper.id,
        None,
        "Existing Entry".to_string(),
        1,
        TocSource::Manual,
    )
    .unwrap();

    let text_file = dir.path().join("invalid.txt");
    fs::write(&text_file, "Chapter 1\t2\nChapter 2\t99").unwrap(); // 99 > 5

    let source = TocImportSource::TextFile(text_file);
    let res = import_toc(&mut conn, paper.id, &pdf_path, &source, false);

    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(matches!(
        err,
        TocError::Import(TocImportError::PageOutOfBounds {
            page: 99,
            page_count: 5
        })
    ));

    // Verify rollback: original entry untouched
    let entries = TocRepo::get_by_paper(&conn, paper.id).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "Existing Entry");
}

/// RT-40: Level jump too large returns line number in error.
#[test]
fn test_rt_40_indent_jump_error_line_number() {
    let text = "Root\t1\n\t\tDouble Indent\t2"; // Jump 0 -> 2
    let res = parse_text(text);
    assert!(res.is_err());
    match res.unwrap_err() {
        TocImportError::Line { line_no, reason } => {
            assert_eq!(line_no, 2);
            assert!(reason.contains("jump too large"));
        }
        other => panic!("Unexpected error: {other:?}"),
    }
}

/// RT-41: Import mode --merge vs replacement.
#[test]
fn test_rt_41_import_merge_vs_replace() {
    let mut conn = setup_db();
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("paper.pdf");
    create_synthetic_pdf(&pdf_path, 10);
    let paper = insert_test_paper(&conn, &pdf_path);

    let file_a = dir.path().join("a.txt");
    fs::write(&file_a, "A\t1").unwrap();

    let file_b = dir.path().join("b.txt");
    fs::write(&file_b, "B\t2").unwrap();

    // 1. Initial import of A (replace)
    import_toc(
        &mut conn,
        paper.id,
        &pdf_path,
        &TocImportSource::TextFile(file_a.clone()),
        false,
    )
    .unwrap();
    let tocs1 = TocRepo::get_by_paper(&conn, paper.id).unwrap();
    assert_eq!(tocs1.len(), 1);
    assert_eq!(tocs1[0].title, "A");

    // 2. Import B with merge = true -> contains A and B
    import_toc(
        &mut conn,
        paper.id,
        &pdf_path,
        &TocImportSource::TextFile(file_b.clone()),
        true,
    )
    .unwrap();
    let tocs2 = TocRepo::get_by_paper(&conn, paper.id).unwrap();
    assert_eq!(tocs2.len(), 2);
    assert_eq!(tocs2[0].title, "A");
    assert_eq!(tocs2[1].title, "B");
    assert_eq!(tocs2[0].order_index, 0);
    assert_eq!(tocs2[1].order_index, 1);

    // 3. Import A again with merge = false -> only A remains
    import_toc(
        &mut conn,
        paper.id,
        &pdf_path,
        &TocImportSource::TextFile(file_a),
        false,
    )
    .unwrap();
    let tocs3 = TocRepo::get_by_paper(&conn, paper.id).unwrap();
    assert_eq!(tocs3.len(), 1);
    assert_eq!(tocs3[0].title, "A");
}

/// RT-42: PDF outline import sets source = Imported, export -> import isomorphism.
#[test]
fn test_rt_42_from_pdf_source_imported_and_isomorphism() {
    let mut conn = setup_db();
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("with_outline.pdf");
    create_synthetic_pdf_with_outline(&pdf_path);
    let paper = insert_test_paper(&conn, &pdf_path);

    // Import from PDF outline
    let imported = import_toc(
        &mut conn,
        paper.id,
        &pdf_path,
        &TocImportSource::PdfOutline,
        false,
    )
    .expect("import from pdf");

    assert_eq!(imported.len(), 2);
    for entry in &imported {
        assert_eq!(
            entry.source,
            TocSource::Imported,
            "All entries from PDF must be marked Imported"
        );
    }

    // Export to JSON
    let json_str = export_to_json(&imported).expect("export to json");

    // Re-import from JSON
    let json_file = dir.path().join("exported.json");
    fs::write(&json_file, &json_str).unwrap();

    let reimported = import_toc(
        &mut conn,
        paper.id,
        &pdf_path,
        &TocImportSource::JsonFile(json_file),
        false,
    )
    .expect("reimport from json");

    assert_eq!(imported.len(), reimported.len());
    for (orig, re) in imported.iter().zip(reimported.iter()) {
        assert_eq!(orig.title, re.title);
        assert_eq!(orig.page_number, re.page_number);
        assert_eq!(orig.order_index, re.order_index);
    }
}
