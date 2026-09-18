use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

use papyrus_core::db::{Collection, CollectionRepo, PaperRepo};
use papyrus_core::doctor::{run_doctor, DoctorVerdict};
use papyrus_core::export::{export_collection, ExportOptions};
use papyrus_core::importer::{import_paper, ImportError};
use papyrus_core::opener::{open_paper, resolve_paper_path, MockCommandRunner, OpenerError};
use papyrus_core::search::SearchIndex;

use super::helpers::{
    create_corrupt_pdf, create_corrupt_tantivy_index, create_valid_pdf_with_text,
    setup_doctor_test_env,
};

#[test]
fn test_rt59_corrupted_pdf_rejected_no_panic() {
    let tmp = tempdir().unwrap();
    let (config, mut conn) = setup_doctor_test_env(tmp.path());
    let corrupt_pdf = tmp.path().join("broken.pdf");
    create_corrupt_pdf(&corrupt_pdf);

    let res = import_paper(&mut conn, &config, &corrupt_pdf, None);
    assert!(res.is_err(), "Corrupted PDF import must fail");
    match res.unwrap_err() {
        ImportError::InvalidPdf { path, reason } => {
            assert_eq!(path, corrupt_pdf);
            assert!(!reason.is_empty());
        }
        other => panic!("Expected InvalidPdf error, got: {other:?}"),
    }

    let papers = PaperRepo::list(&conn).unwrap();
    assert!(papers.is_empty(), "Database must remain pristine on error");
}

#[test]
fn test_rt60_missing_metadata_fallback_to_filename() {
    let tmp = tempdir().unwrap();
    let (config, mut conn) = setup_doctor_test_env(tmp.path());
    let pdf_path = tmp.path().join("quantum_entanglement.pdf");
    create_valid_pdf_with_text(&pdf_path, "Content without PDF metadata dictionary");

    let paper = import_paper(&mut conn, &config, &pdf_path, None).unwrap();
    assert_eq!(
        paper.title.as_deref(),
        Some("quantum_entanglement"),
        "Missing title should fall back to file stem"
    );
}

#[test]
fn test_rt61_deleted_pdf_graceful_error() {
    let tmp = tempdir().unwrap();
    let (config, mut conn) = setup_doctor_test_env(tmp.path());
    let pdf_path = tmp.path().join("paper_to_delete.pdf");
    create_valid_pdf_with_text(&pdf_path, "Temporary content");

    let paper = import_paper(&mut conn, &config, &pdf_path, None).unwrap();
    fs::remove_file(&pdf_path).expect("delete pdf from disk");

    let res = resolve_paper_path(&paper, &config);
    assert!(matches!(res, Err(OpenerError::FileNotFound(_))));

    let runner = MockCommandRunner::new();
    let open_res = open_paper(&paper, None, &config, &runner);
    assert!(open_res.is_err(), "Opening missing PDF must return error");

    let report = run_doctor(&conn, &config, false);
    assert_eq!(report.verdict(), DoctorVerdict::Failed);
    assert!(report
        .errors
        .iter()
        .any(|e| e.contains("original PDF file not found")));
}

#[test]
fn test_rt62_deleted_txt_warning_no_crash() {
    let tmp = tempdir().unwrap();
    let (config, mut conn) = setup_doctor_test_env(tmp.path());
    let pdf_path = tmp.path().join("paper.pdf");
    create_valid_pdf_with_text(&pdf_path, "Text");

    let mut paper = import_paper(&mut conn, &config, &pdf_path, None).unwrap();
    let missing_txt = tmp.path().join("deleted_extracted.txt");
    paper.text_path = Some(missing_txt.to_string_lossy().to_string());
    PaperRepo::update(&conn, &paper).unwrap();

    let report = run_doctor(&conn, &config, false);
    assert_eq!(
        report.verdict(),
        DoctorVerdict::Warnings,
        "Missing text file must be warning, not failed error"
    );
    assert!(report.errors.is_empty());
    assert!(report
        .warnings
        .iter()
        .any(|w| w.contains("text_path points to missing file")));
}

#[test]
fn test_rt63_corrupted_tantivy_index_doctor_and_fallback() {
    let tmp = tempdir().unwrap();
    let (config, conn) = setup_doctor_test_env(tmp.path());
    let search_dir = create_corrupt_tantivy_index(tmp.path());

    let report = run_doctor(&conn, &config, false);
    assert_eq!(report.verdict(), DoctorVerdict::Failed);
    assert!(report
        .errors
        .iter()
        .any(|e| e.contains("Search index at") && e.contains("is corrupt")));

    let fallback_res = SearchIndex::open_or_create(&search_dir);
    assert!(
        fallback_res.is_err(),
        "Opening corrupted index must return error without panic"
    );
}

#[test]
fn test_rt64_empty_collection_export_no_panic() {
    let tmp = tempdir().unwrap();
    let (config, conn) = setup_doctor_test_env(tmp.path());
    let collection = Collection {
        id: Uuid::now_v7(),
        name: "Empty Collection".into(),
        parent_id: None,
    };
    CollectionRepo::insert(&conn, &collection).unwrap();

    let zip_path = tmp.path().join("empty_col.zip");
    let opts = ExportOptions {
        output_path: zip_path.clone(),
        overwrite: true,
        include_database: true,
        skip_missing_files: false,
    };

    let res = export_collection(&conn, &config, collection.id, &opts);
    assert!(res.is_ok(), "Exporting empty collection must not panic");
    assert!(zip_path.exists(), "Export zip file must be produced");
    let export_res = res.unwrap();
    assert_eq!(export_res.paper_count, 0);
}
