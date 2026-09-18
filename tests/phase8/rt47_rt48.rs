use std::fs::File;
use tempfile::tempdir;
use uuid::Uuid;
use zip::ZipArchive;

use papyrus_core::db::{Paper, PaperRepo};
use papyrus_core::export::{export_library, ExportError, ExportOptions};

use super::helpers::{compute_sha256, create_test_pdf, read_zip_manifest, setup_test_library};

/// RT-47: Missing PDF graceful handling.
#[test]
fn test_rt47_missing_pdf_graceful_handling() {
    let dir = tempdir().expect("tempdir");
    let (conn, config) = setup_test_library(dir.path());

    let p1_path = dir.path().join("present.pdf");
    create_test_pdf(&p1_path, "Present file");
    let p1_bytes = std::fs::read(&p1_path).unwrap();

    let p2_path = dir.path().join("missing.pdf");
    create_test_pdf(&p2_path, "To be deleted");
    let p2_bytes = std::fs::read(&p2_path).unwrap();
    std::fs::remove_file(&p2_path).unwrap();

    let p1 = Paper {
        id: Uuid::now_v7(),
        file_path: p1_path.to_str().unwrap().into(),
        content_hash: compute_sha256(&p1_bytes),
        title: Some("Present".into()),
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
    let p2 = Paper {
        id: Uuid::now_v7(),
        file_path: p2_path.to_str().unwrap().into(),
        content_hash: compute_sha256(&p2_bytes),
        title: Some("Missing".into()),
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
    PaperRepo::insert(&conn, &p1).unwrap();
    PaperRepo::insert(&conn, &p2).unwrap();

    // 1. skip_missing_files = false should fail
    let zip_strict = dir.path().join("strict.zip");
    let strict_opts = ExportOptions {
        output_path: zip_strict,
        overwrite: true,
        include_database: true,
        skip_missing_files: false,
    };
    let err = export_library(&conn, &config, &strict_opts).unwrap_err();
    match err {
        ExportError::PaperFileNotFound { id, .. } => assert_eq!(id, p2.id),
        other => panic!("expected PaperFileNotFound, got: {other:?}"),
    }

    // 2. skip_missing_files = true should succeed with warnings
    let zip_lenient = dir.path().join("lenient.zip");
    let lenient_opts = ExportOptions {
        output_path: zip_lenient.clone(),
        overwrite: true,
        include_database: true,
        skip_missing_files: true,
    };
    let res = export_library(&conn, &config, &lenient_opts).expect("should succeed with skip");
    assert!(!res.warnings.is_empty());
    assert!(res.warnings[0].contains(&p2.id.to_string()));

    let file = File::open(&zip_lenient).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let manifest = read_zip_manifest(&mut archive);
    assert_eq!(manifest.papers.len(), 2);
}

/// RT-48: Unicode metadata and filenames.
#[test]
fn test_rt48_unicode_metadata_and_filenames() {
    let dir = tempdir().expect("tempdir");
    let (conn, config) = setup_test_library(dir.path());

    let filename = "статья_шеннон_🌟.pdf";
    let pdf_path = dir.path().join(filename);
    create_test_pdf(&pdf_path, "Квантовая информация");
    let pdf_bytes = std::fs::read(&pdf_path).unwrap();

    let paper = Paper {
        id: Uuid::now_v7(),
        file_path: pdf_path.to_str().unwrap().into(),
        content_hash: compute_sha256(&pdf_bytes),
        title: Some("Теория информации и энтропия".into()),
        authors: Some("Клод Шеннон & 艾伦·图灵".into()),
        year: Some(2026),
        journal: Some("Журнал квантовых вычислений".into()),
        doi: Some("10.1000/182".into()),
        abstract_text: Some("Экспериментальные данные: E = mc².".into()),
        text_path: None,
        annotated_pdf_path: None,
        toc_embedded_at: None,
        created_at: "2026-01-01 00:00:00".into(),
        updated_at: "2026-01-01 00:00:00".into(),
    };
    PaperRepo::insert(&conn, &paper).unwrap();

    let zip_out = dir.path().join("unicode.zip");
    let options = ExportOptions {
        output_path: zip_out.clone(),
        overwrite: true,
        include_database: true,
        skip_missing_files: false,
    };
    export_library(&conn, &config, &options).expect("unicode export succeeds");

    let file = File::open(&zip_out).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let manifest = read_zip_manifest(&mut archive);

    assert_eq!(manifest.papers.len(), 1);
    let p = &manifest.papers[0];
    assert_eq!(p.title.as_deref(), Some("Теория информации и энтропия"));
    assert_eq!(p.authors.as_deref(), Some("Клод Шеннон & 艾伦·图灵"));
    assert_eq!(
        p.abstract_text.as_deref(),
        Some("Экспериментальные данные: E = mc².")
    );
    assert!(p.file_path.contains("статья_шеннон_🌟.pdf"));
    assert!(archive.by_name(&p.file_path).is_ok());
}
