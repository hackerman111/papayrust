use std::fs;
use std::path::PathBuf;

use lopdf::Document;
use papyrus_core::config::Config;
use papyrus_core::db::{Paper, TocRepo, TocSource};
use papyrus_core::opener::{open_paper, MockCommandRunner};
use papyrus_core::toc::{add_entry, edit_entry, embed_toc_in_pdf};
use papyrus_core::Action;
use papyrus_tui::App;
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use uuid::Uuid;

use super::helpers::{create_synthetic_pdf, insert_test_paper, setup_db};

/// RT-32: Embed TOC into PDF, roundtrip reading with lopdf.
#[test]
fn test_rt_32_embed_toc_roundtrip() {
    let mut conn = setup_db();
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("paper.pdf");
    create_synthetic_pdf(&pdf_path, 10);
    let paper = insert_test_paper(&conn, &pdf_path);

    add_entry(
        &mut conn,
        paper.id,
        None,
        "Chapter 1".to_string(),
        1,
        TocSource::Manual,
    )
    .unwrap();
    add_entry(
        &mut conn,
        paper.id,
        None,
        "Chapter 2".to_string(),
        5,
        TocSource::Manual,
    )
    .unwrap();

    let mut config = Config {
        library_path: dir.path().to_path_buf(),
        ..Default::default()
    };
    config.toc.annotated_dir = PathBuf::from("annotated");

    let annotated_path = embed_toc_in_pdf(&mut conn, &config, paper.id).expect("embed toc");
    assert!(annotated_path.exists());

    // Load generated PDF and verify Outlines
    let doc = Document::load(&annotated_path).expect("load annotated PDF");
    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .unwrap();
    let catalog_dict = doc
        .get_object(catalog_id)
        .and_then(|o| o.as_dict())
        .unwrap();
    let outlines_ref = catalog_dict
        .get(b"Outlines")
        .and_then(|o| o.as_reference())
        .unwrap();
    let outlines_dict = doc
        .get_object(outlines_ref)
        .and_then(|o| o.as_dict())
        .unwrap();

    let first_ref = outlines_dict
        .get(b"First")
        .and_then(|o| o.as_reference())
        .unwrap();
    let first_item = doc.get_object(first_ref).and_then(|o| o.as_dict()).unwrap();
    let title = first_item.get(b"Title").and_then(|o| o.as_str()).unwrap();
    assert_eq!(String::from_utf8_lossy(title), "Chapter 1");
}

/// RT-33: Immutability of original PDF bytes (SHA-256 matches before and after embed).
#[test]
fn test_rt_33_original_pdf_immutability() {
    let mut conn = setup_db();
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("original.pdf");
    create_synthetic_pdf(&pdf_path, 5);
    let paper = insert_test_paper(&conn, &pdf_path);

    let original_bytes_before = fs::read(&pdf_path).unwrap();
    let hash_before = format!("{:x}", Sha256::digest(&original_bytes_before));

    add_entry(
        &mut conn,
        paper.id,
        None,
        "Title".to_string(),
        1,
        TocSource::Manual,
    )
    .unwrap();

    let mut config = Config {
        library_path: dir.path().to_path_buf(),
        ..Default::default()
    };
    config.toc.annotated_dir = PathBuf::from("annotated");

    let _ = embed_toc_in_pdf(&mut conn, &config, paper.id).expect("embed");

    let original_bytes_after = fs::read(&pdf_path).unwrap();
    let hash_after = format!("{:x}", Sha256::digest(&original_bytes_after));

    assert_eq!(hash_before, hash_after, "Original PDF was mutated!");
    assert_eq!(original_bytes_before, original_bytes_after);
}

/// RT-34: Repeated embed updates copy and resets outdated flag in App.
#[test]
fn test_rt_34_repeated_embed_resets_outdated_flag() {
    let conn = setup_db();
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("paper.pdf");
    create_synthetic_pdf(&pdf_path, 5);
    let _paper = insert_test_paper(&conn, &pdf_path);

    let mut config = Config {
        library_path: dir.path().to_path_buf(),
        ..Default::default()
    };
    config.toc.annotated_dir = PathBuf::from("annotated");

    let mut app = App::from_db_conn(conn).unwrap();
    app.set_config(config);

    // Add entry
    app.dispatch(Action::TocAddEntry { parent_id: None });
    for c in "Intro".chars() {
        app.dispatch(Action::TocEditModalInput(c));
    }
    app.dispatch(Action::TocEditModalSave);

    // Initial embed
    app.dispatch(Action::TocEmbed);
    assert!(
        !app.is_current_paper_annotated_outdated(),
        "Should not be outdated right after embed"
    );

    // Age toc_embedded_at to simulate time passing
    if let Some(conn) = app.db_conn_mut() {
        conn.execute(
            "UPDATE papers SET toc_embedded_at = '2020-01-01T00:00:00Z' WHERE id = ?1",
            rusqlite::params![_paper.id.to_string()],
        )
        .unwrap();
    }
    app.reload_current_paper_and_tocs();

    // Modify entry -> marks paper updated_at newer than toc_embedded_at
    let toc_id = app.toc_preview[0].id;
    app.dispatch(Action::TocEditEntry { id: toc_id });
    app.dispatch(Action::TocEditModalInput('!'));
    app.dispatch(Action::TocEditModalSave);

    assert!(
        app.is_current_paper_annotated_outdated(),
        "Should be outdated after modifying TOC"
    );

    // Re-embed resets outdated flag
    app.dispatch(Action::TocEmbed);
    assert!(
        !app.is_current_paper_annotated_outdated(),
        "Flag should be reset after re-embed"
    );
}

/// RT-35: Editing TOC entry preserves source = Auto.
#[test]
fn test_rt_35_preserve_source_auto_on_edit() {
    let mut conn = setup_db();
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("paper.pdf");
    create_synthetic_pdf(&pdf_path, 5);
    let paper = insert_test_paper(&conn, &pdf_path);

    // Insert entry with Auto source
    let auto_entry = add_entry(
        &mut conn,
        paper.id,
        None,
        "Auto Title".to_string(),
        1,
        TocSource::Auto,
    )
    .unwrap();
    assert_eq!(auto_entry.source, TocSource::Auto);

    // Edit title
    let edited = edit_entry(
        &mut conn,
        auto_entry.id,
        Some("Edited Title".to_string()),
        None,
    )
    .unwrap();
    assert_eq!(edited.title, "Edited Title");
    assert_eq!(
        edited.source,
        TocSource::Auto,
        "Source must remain Auto after edit"
    );

    let db_entry = TocRepo::get_by_id(&conn, auto_entry.id).unwrap().unwrap();
    assert_eq!(db_entry.source, TocSource::Auto);
}

/// RT-36: OpenAtPage without embed opens original PDF with {page} substitution.
#[test]
fn test_rt_36_open_at_page_without_embed_uses_original() {
    let dir = tempdir().unwrap();
    let pdf_path = dir.path().join("original.pdf");
    create_synthetic_pdf(&pdf_path, 10);

    let paper = Paper {
        id: Uuid::now_v7(),
        file_path: pdf_path.to_string_lossy().to_string(),
        content_hash: "hash".to_string(),
        title: Some("Paper".to_string()),
        authors: None,
        year: None,
        journal: None,
        doi: None,
        abstract_text: None,
        text_path: None,
        annotated_pdf_path: None,
        toc_embedded_at: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };

    let config = Config {
        pdf: papyrus_core::config::PdfConfig {
            page_open_template: "zathura --page={page} {path}".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    let runner = MockCommandRunner::new();
    open_paper(&paper, Some(7), &config, &runner).expect("open paper at page 7");

    assert_eq!(runner.command_count(), 1);
    let (prog, args) = runner.last_command().unwrap();
    assert_eq!(prog, "zathura");
    assert_eq!(
        args,
        vec![
            "--page=7".to_string(),
            pdf_path.to_string_lossy().to_string()
        ]
    );
}
