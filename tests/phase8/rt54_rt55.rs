use std::fs::File;
use tempfile::tempdir;
use uuid::Uuid;
use zip::ZipArchive;

use papyrus_core::db::{Paper, PaperRepo};
use papyrus_core::export::{export_library, ExportError, ExportOptions};

use super::helpers::{compute_sha256, create_test_pdf, read_zip_manifest, setup_test_library};

/// RT-54: Force overwrite flag.
#[test]
fn test_rt54_force_overwrite_flag() {
    let dir = tempdir().expect("tempdir");
    let (conn, config) = setup_test_library(dir.path());

    let zip_out = dir.path().join("force_test.zip");
    let opt_create = ExportOptions {
        output_path: zip_out.clone(),
        overwrite: false,
        include_database: false,
        skip_missing_files: false,
    };
    export_library(&conn, &config, &opt_create).expect("first export succeeds");
    assert!(zip_out.exists());

    // Without force, should return AlreadyExists
    let opt_no_force = ExportOptions {
        output_path: zip_out.clone(),
        overwrite: false,
        include_database: false,
        skip_missing_files: false,
    };
    let err = export_library(&conn, &config, &opt_no_force).unwrap_err();
    match err {
        ExportError::AlreadyExists(p) => assert_eq!(p, zip_out),
        other => panic!("expected AlreadyExists, got: {other:?}"),
    }

    // With force, should overwrite successfully
    let opt_force = ExportOptions {
        output_path: zip_out.clone(),
        overwrite: true,
        include_database: false,
        skip_missing_files: false,
    };
    assert!(export_library(&conn, &config, &opt_force).is_ok());
}

/// RT-55: Annotated PDF export only when fresh.
#[test]
fn test_rt55_annotated_pdf_export_only_when_fresh() {
    let dir = tempdir().expect("tempdir");
    let (conn, config) = setup_test_library(dir.path());

    let orig_a = dir.path().join("orig_a.pdf");
    let ann_a = dir.path().join("ann_a.annotated.pdf");
    create_test_pdf(&orig_a, "Original A");
    create_test_pdf(&ann_a, "Annotated A");

    let orig_b = dir.path().join("orig_b.pdf");
    let ann_b = dir.path().join("ann_b.annotated.pdf");
    create_test_pdf(&orig_b, "Original B");
    create_test_pdf(&ann_b, "Annotated B");

    // Paper A: Fresh (embedded_at >= updated_at)
    let paper_a = Paper {
        id: Uuid::now_v7(),
        file_path: orig_a.to_str().unwrap().into(),
        content_hash: compute_sha256(&std::fs::read(&orig_a).unwrap()),
        title: Some("Paper A (Fresh)".into()),
        authors: None,
        year: None,
        journal: None,
        doi: None,
        abstract_text: None,
        text_path: None,
        annotated_pdf_path: Some(ann_a.to_str().unwrap().into()),
        toc_embedded_at: Some("2026-06-01 12:00:00".into()),
        created_at: "2026-01-01 00:00:00".into(),
        updated_at: "2026-06-01 10:00:00".into(),
    };

    // Paper B: Outdated (updated_at > embedded_at)
    let paper_b = Paper {
        id: Uuid::now_v7(),
        file_path: orig_b.to_str().unwrap().into(),
        content_hash: compute_sha256(&std::fs::read(&orig_b).unwrap()),
        title: Some("Paper B (Outdated)".into()),
        authors: None,
        year: None,
        journal: None,
        doi: None,
        abstract_text: None,
        text_path: None,
        annotated_pdf_path: Some(ann_b.to_str().unwrap().into()),
        toc_embedded_at: Some("2026-06-01 08:00:00".into()),
        created_at: "2026-01-01 00:00:00".into(),
        updated_at: "2026-06-01 10:00:00".into(),
    };

    PaperRepo::insert(&conn, &paper_a).unwrap();
    PaperRepo::insert(&conn, &paper_b).unwrap();

    let zip_out = dir.path().join("annotated_freshness.zip");
    let options = ExportOptions {
        output_path: zip_out.clone(),
        overwrite: true,
        include_database: false,
        skip_missing_files: false,
    };
    export_library(&conn, &config, &options).expect("export succeeds");

    let file = File::open(&zip_out).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let manifest = read_zip_manifest(&mut archive);

    let ma = manifest.papers.iter().find(|p| p.id == paper_a.id).unwrap();
    assert!(ma.annotated_pdf_path.is_some());
    assert!(archive
        .by_name(ma.annotated_pdf_path.as_ref().unwrap())
        .is_ok());

    let mb = manifest.papers.iter().find(|p| p.id == paper_b.id).unwrap();
    assert!(mb.annotated_pdf_path.is_none());
}
