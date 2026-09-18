use tempfile::tempdir;
use uuid::Uuid;

use papyrus_core::db::{PaperRepo, TocEntry, TocRepo, TocSource};
use papyrus_core::doctor::{check_database, run_doctor, DoctorVerdict};
use papyrus_core::importer::import_paper;

use super::helpers::{create_corrupt_sqlite, create_valid_pdf_with_text, setup_doctor_test_env};

#[test]
fn test_rt70_doctor_detects_dead_annotated_pdf_and_fk_violations() {
    let tmp = tempdir().unwrap();
    let (config, mut conn) = setup_doctor_test_env(tmp.path());

    let pdf_path = tmp.path().join("paper1.pdf");
    create_valid_pdf_with_text(&pdf_path, "Paper 1 content");
    let mut paper1 = import_paper(&mut conn, &config, &pdf_path, None).unwrap();

    let pdf_path2 = tmp.path().join("paper2.pdf");
    create_valid_pdf_with_text(&pdf_path2, "Paper 2 content");
    let paper2 = import_paper(&mut conn, &config, &pdf_path2, None).unwrap();

    // 1. Inject dead annotated_pdf_path
    let missing_annotated = tmp.path().join("missing_annotated.pdf");
    paper1.annotated_pdf_path = Some(missing_annotated.to_string_lossy().to_string());
    PaperRepo::update(&conn, &paper1).unwrap();

    // 2. Inject Foreign Key violation (bypassing FK constraints)
    conn.execute_batch(
        "
        PRAGMA foreign_keys = OFF;
        INSERT INTO paper_collections (paper_id, collection_id)
        VALUES ('00000000-0000-0000-0000-000000000000', '11111111-1111-1111-1111-111111111111');
        PRAGMA foreign_keys = ON;
        ",
    )
    .unwrap();

    // 3. Inject TOC hierarchy violation: child entry for paper2 has parent entry from paper1
    let parent_toc = TocEntry {
        id: Uuid::now_v7(),
        paper_id: paper1.id,
        parent_id: None,
        title: "Paper 1 Section".into(),
        page_number: 1,
        order_index: 0,
        source: TocSource::Manual,
        created_at: "2026-01-01 00:00:00".into(),
        updated_at: "2026-01-01 00:00:00".into(),
    };
    TocRepo::insert(&conn, &parent_toc).unwrap();

    let cross_paper_child_toc = TocEntry {
        id: Uuid::now_v7(),
        paper_id: paper2.id,            // belongs to paper2
        parent_id: Some(parent_toc.id), // points to parent from paper1!
        title: "Paper 2 Subsection with Paper 1 Parent".into(),
        page_number: 1,
        order_index: 0,
        source: TocSource::Manual,
        created_at: "2026-01-01 00:00:00".into(),
        updated_at: "2026-01-01 00:00:00".into(),
    };
    TocRepo::insert(&conn, &cross_paper_child_toc).unwrap();

    // Run doctor
    let report = run_doctor(&conn, &config, false);
    assert_eq!(report.verdict(), DoctorVerdict::Failed);

    let has_dead_ann = report
        .errors
        .iter()
        .any(|e| e.contains("dead annotated_pdf_path points to missing file"));
    assert!(has_dead_ann, "Must detect dead annotated_pdf_path");

    let has_fk_violation = report
        .errors
        .iter()
        .any(|e| e.contains("Foreign key violation in table 'paper_collections'"));
    assert!(has_fk_violation, "Must detect foreign key violation");

    let has_toc_violation = report
        .errors
        .iter()
        .any(|e| e.contains("TOC hierarchy mismatch"));
    assert!(has_toc_violation, "Must detect cross-paper TOC mismatch");
}

#[test]
fn test_rt71_doctor_full_integrity_check_detects_db_corruption() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("corrupted.db");
    create_corrupt_sqlite(&db_path);

    let conn = rusqlite::Connection::open(&db_path).expect("open raw sqlite connection");
    let report = check_database(&conn, true);

    assert_eq!(
        report.verdict(),
        DoctorVerdict::Failed,
        "Corrupted database must fail integrity_check"
    );
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("Integrity check failure")
                || e.contains("database disk image is malformed")),
        "Error report must indicate integrity check failure: {:?}",
        report.errors
    );
}
