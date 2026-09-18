use std::fs::File;
use tempfile::tempdir;
use uuid::Uuid;
use zip::ZipArchive;

use papyrus_core::db::{
    Collection, CollectionRepo, Paper, PaperRepo, TocEntry, TocRepo, TocSource,
};
use papyrus_core::export::{export_library, ExportOptions};

use super::helpers::{
    compute_sha256, create_test_pdf, read_zip_entry, read_zip_manifest, setup_test_library,
};

/// RT-51: Manifest validity and TOC hierarchy level calculation.
#[test]
fn test_rt51_manifest_validity_and_toc_hierarchy() {
    let dir = tempdir().expect("tempdir");
    let (conn, config) = setup_test_library(dir.path());

    let p_path = dir.path().join("toc_paper.pdf");
    create_test_pdf(&p_path, "TOC paper content");
    let p_bytes = std::fs::read(&p_path).unwrap();

    let paper = Paper {
        id: Uuid::now_v7(),
        file_path: p_path.to_str().unwrap().into(),
        content_hash: compute_sha256(&p_bytes),
        title: Some("TOC Paper".into()),
        authors: None,
        year: None,
        journal: None,
        doi: None,
        abstract_text: None,
        text_path: None,
        annotated_pdf_path: None,
        toc_embedded_at: None,
        created_at: "2026-01-01 00:00:00".into(),
        updated_at: "2026-01-01 00:00:00".into(),
    };
    PaperRepo::insert(&conn, &paper).unwrap();

    let id1 = Uuid::now_v7();
    let id2 = Uuid::now_v7();
    let id3 = Uuid::now_v7();
    let id4 = Uuid::now_v7();
    let id5 = Uuid::now_v7();

    fn make_toc(
        id: Uuid,
        pid: Uuid,
        parent: Option<Uuid>,
        title: &str,
        page: u32,
        order: i32,
    ) -> TocEntry {
        TocEntry {
            id,
            paper_id: pid,
            parent_id: parent,
            title: title.into(),
            page_number: page,
            order_index: order,
            source: TocSource::Manual,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    let entries = vec![
        make_toc(id1, paper.id, None, "Introduction", 1, 0),
        make_toc(id2, paper.id, None, "Background", 2, 1),
        make_toc(id3, paper.id, Some(id2), "Related Work", 3, 0),
        make_toc(id4, paper.id, Some(id3), "Deep Dive", 4, 0),
        make_toc(id5, paper.id, None, "Conclusion", 10, 2),
    ];
    TocRepo::insert_batch(&conn, &entries).unwrap();

    let zip_out = dir.path().join("toc_hierarchy.zip");
    let options = ExportOptions {
        output_path: zip_out.clone(),
        overwrite: true,
        include_database: true,
        skip_missing_files: false,
    };
    export_library(&conn, &config, &options).expect("export succeeds");

    let file = File::open(&zip_out).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let manifest = read_zip_manifest(&mut archive);

    assert_eq!(manifest.version, 1);
    assert!(!manifest.exported_at.is_empty());
    assert_eq!(manifest.papers.len(), 1);

    let tocs = &manifest.papers[0].toc_entries;
    assert_eq!(tocs.len(), 5);
    assert_eq!(tocs[0].title, "Introduction");
    assert_eq!(tocs[0].level, 0);
    assert_eq!(tocs[1].title, "Background");
    assert_eq!(tocs[1].level, 0);
    assert_eq!(tocs[2].title, "Related Work");
    assert_eq!(tocs[2].level, 1);
    assert_eq!(tocs[3].title, "Deep Dive");
    assert_eq!(tocs[3].level, 2);
    assert_eq!(tocs[4].title, "Conclusion");
    assert_eq!(tocs[4].level, 0);
}

/// RT-52: Reopen ZIP roundtrip integrity.
#[test]
fn test_rt52_reopen_zip_roundtrip_integrity() {
    let dir = tempdir().expect("tempdir");
    let (conn, config) = setup_test_library(dir.path());

    let col = Collection {
        id: Uuid::now_v7(),
        name: "Algorithms".into(),
        parent_id: None,
    };
    CollectionRepo::insert(&conn, &col).unwrap();

    let p_path = dir.path().join("algo.pdf");
    create_test_pdf(&p_path, "Algorithm Design Manual");
    let p_bytes = std::fs::read(&p_path).unwrap();

    let paper = Paper {
        id: Uuid::now_v7(),
        file_path: p_path.to_str().unwrap().into(),
        content_hash: compute_sha256(&p_bytes),
        title: Some("Algorithms".into()),
        authors: Some("Skiena".into()),
        year: Some(2008),
        journal: None,
        doi: None,
        abstract_text: None,
        text_path: None,
        annotated_pdf_path: None,
        toc_embedded_at: None,
        created_at: "2026-01-01 00:00:00".into(),
        updated_at: "2026-01-01 00:00:00".into(),
    };
    PaperRepo::insert(&conn, &paper).unwrap();
    CollectionRepo::add_paper(&conn, paper.id, col.id).unwrap();

    let zip_out = dir.path().join("roundtrip.zip");
    let options = ExportOptions {
        output_path: zip_out.clone(),
        overwrite: true,
        include_database: true,
        skip_missing_files: false,
    };
    export_library(&conn, &config, &options).expect("export succeeds");

    let file = File::open(&zip_out).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let manifest = read_zip_manifest(&mut archive);

    for p in &manifest.papers {
        let extracted_bytes = read_zip_entry(&mut archive, &p.file_path);
        assert_eq!(compute_sha256(&extracted_bytes), p.content_hash);
    }

    let db_bytes = read_zip_entry(&mut archive, ".library/library.db");
    let extracted_db_path = dir.path().join("reopened.db");
    std::fs::write(&extracted_db_path, db_bytes).unwrap();

    let reopened_conn = rusqlite::Connection::open(&extracted_db_path).unwrap();
    let check: String = reopened_conn
        .query_row("PRAGMA integrity_check;", [], |row| row.get(0))
        .unwrap();
    assert_eq!(check, "ok");
}

/// RT-53: Atomicity on export interruption / failure.
#[test]
fn test_rt53_atomicity_on_export_interruption() {
    let dir = tempdir().expect("tempdir");
    let (conn, config) = setup_test_library(dir.path());

    let missing_path = dir.path().join("non_existent_paper.pdf");
    let paper = Paper {
        id: Uuid::now_v7(),
        file_path: missing_path.to_str().unwrap().into(),
        content_hash: "dummy_hash".into(),
        title: Some("Will Fail".into()),
        authors: None,
        year: None,
        journal: None,
        doi: None,
        abstract_text: None,
        text_path: None,
        annotated_pdf_path: None,
        toc_embedded_at: None,
        created_at: "2026-01-01 00:00:00".into(),
        updated_at: "2026-01-01 00:00:00".into(),
    };
    PaperRepo::insert(&conn, &paper).unwrap();

    let zip_out = dir.path().join("interrupted.zip");
    let options = ExportOptions {
        output_path: zip_out.clone(),
        overwrite: true,
        include_database: true,
        skip_missing_files: false,
    };

    let err = export_library(&conn, &config, &options);
    assert!(err.is_err());
    assert!(!zip_out.exists(), "Target file must not exist on failure");

    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        assert!(
            !name.starts_with(".tmp_export_"),
            "Found leftover temp file: {name}"
        );
    }
}
