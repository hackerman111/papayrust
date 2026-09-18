use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;
use uuid::Uuid;
use zip::ZipArchive;

use papyrus_core::db::{open_database, Paper, PaperRepo};
use papyrus_core::export::{export_library, validate_archive_entry_path, ExportOptions};

use super::helpers::{compute_sha256, create_test_pdf, read_zip_entry, setup_test_library};

/// RT-49: Path traversal protection.
#[test]
fn test_rt49_path_traversal_protection() {
    assert!(validate_archive_entry_path("../secret.txt").is_err());
    assert!(validate_archive_entry_path("../../etc/shadow").is_err());
    assert!(validate_archive_entry_path("papers/../../root.pdf").is_err());
    assert!(validate_archive_entry_path("/absolute/path.pdf").is_err());
    assert!(validate_archive_entry_path("\\windows\\system32").is_err());
    assert!(validate_archive_entry_path("").is_err());
    assert!(validate_archive_entry_path("invalid\0byte").is_err());

    assert!(validate_archive_entry_path("papers/normal.pdf").is_ok());
    assert!(validate_archive_entry_path("annotated/paper_1.annotated.pdf").is_ok());
    assert!(validate_archive_entry_path(".library/library.db").is_ok());
    assert!(validate_archive_entry_path("manifest.json").is_ok());
}

/// RT-50: Concurrent writes hot backup consistency.
#[test]
fn test_rt50_concurrent_writes_hot_backup_consistency() {
    let dir = tempdir().expect("tempdir");
    let (conn, config) = setup_test_library(dir.path());

    let p_path = dir.path().join("init.pdf");
    create_test_pdf(&p_path, "Initial paper");
    let p_bytes = std::fs::read(&p_path).unwrap();
    let p = Paper {
        id: Uuid::now_v7(),
        file_path: p_path.to_str().unwrap().into(),
        content_hash: compute_sha256(&p_bytes),
        title: Some("Initial".into()),
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
    PaperRepo::insert(&conn, &p).unwrap();

    let running = Arc::new(AtomicBool::new(true));
    let r_clone = running.clone();
    let db_path_clone = config.database_path.clone();

    let writer_handle = thread::spawn(move || {
        let writer_conn = open_database(&db_path_clone).expect("writer opens db");
        let mut count = 0;
        while r_clone.load(Ordering::Relaxed) {
            count += 1;
            let title = format!("Concurrent Paper {count}");
            let dummy = Paper {
                id: Uuid::now_v7(),
                file_path: format!("/tmp/dummy_{count}.pdf"),
                content_hash: format!("hash_{count}"),
                title: Some(title),
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
            let _ = PaperRepo::insert(&writer_conn, &dummy);
            thread::sleep(Duration::from_millis(5));
        }
    });

    let zip_out = dir.path().join("concurrent.zip");
    let options = ExportOptions {
        output_path: zip_out.clone(),
        overwrite: true,
        include_database: true,
        skip_missing_files: true,
    };
    let res = export_library(&conn, &config, &options);

    running.store(false, Ordering::Relaxed);
    writer_handle.join().unwrap();

    let export_result = res.expect("export should succeed during concurrent writes");
    assert!(export_result.archive_path.exists());

    let file = File::open(&zip_out).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let db_bytes = read_zip_entry(&mut archive, ".library/library.db");

    let backup_db_path = dir.path().join("extracted.db");
    std::fs::write(&backup_db_path, db_bytes).unwrap();

    let extracted_conn = rusqlite::Connection::open(&backup_db_path).unwrap();
    let check: String = extracted_conn
        .query_row("PRAGMA integrity_check;", [], |row| row.get(0))
        .unwrap();
    assert_eq!(check, "ok");
}
