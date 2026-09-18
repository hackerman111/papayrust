use std::fs::File;
use tempfile::tempdir;
use uuid::Uuid;
use zip::ZipArchive;

use papyrus_core::db::{Collection, CollectionRepo, Paper, PaperRepo};
use papyrus_core::export::{export_collection, export_library, ExportOptions};

use super::helpers::{compute_sha256, create_test_pdf, read_zip_manifest, setup_test_library};

/// RT-43: Export single collection.
#[test]
fn test_rt43_export_single_collection() {
    let dir = tempdir().expect("tempdir");
    let (conn, config) = setup_test_library(dir.path());

    let col_ml = Collection {
        id: Uuid::now_v7(),
        name: "Machine Learning".into(),
        parent_id: None,
    };
    let col_phys = Collection {
        id: Uuid::now_v7(),
        name: "Physics".into(),
        parent_id: None,
    };
    CollectionRepo::insert(&conn, &col_ml).unwrap();
    CollectionRepo::insert(&conn, &col_phys).unwrap();

    let p1_path = dir.path().join("ml_paper1.pdf");
    create_test_pdf(&p1_path, "Deep Learning Paper");
    let p1_bytes = std::fs::read(&p1_path).unwrap();
    let p1 = Paper {
        id: Uuid::now_v7(),
        file_path: p1_path.to_str().unwrap().into(),
        content_hash: compute_sha256(&p1_bytes),
        title: Some("DL Paper".into()),
        authors: Some("Goodfellow".into()),
        year: Some(2016),
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
    CollectionRepo::add_paper(&conn, p1.id, col_ml.id).unwrap();

    let p2_path = dir.path().join("phys_paper.pdf");
    create_test_pdf(&p2_path, "Quantum Physics");
    let p2_bytes = std::fs::read(&p2_path).unwrap();
    let p2 = Paper {
        id: Uuid::now_v7(),
        file_path: p2_path.to_str().unwrap().into(),
        content_hash: compute_sha256(&p2_bytes),
        title: Some("Quantum Paper".into()),
        authors: Some("Planck".into()),
        year: Some(1900),
        journal: None,
        doi: None,
        abstract_text: None,
        text_path: None,
        annotated_pdf_path: None,
        toc_embedded_at: None,
        created_at: "2026-01-01 00:00:00".into(),
        updated_at: "2026-01-01 00:00:00".into(),
    };
    PaperRepo::insert(&conn, &p2).unwrap();
    CollectionRepo::add_paper(&conn, p2.id, col_phys.id).unwrap();

    let zip_out = dir.path().join("ml_export.zip");
    let options = ExportOptions {
        output_path: zip_out.clone(),
        overwrite: false,
        include_database: true,
        skip_missing_files: false,
    };

    let result =
        export_collection(&conn, &config, col_ml.id, &options).expect("export should succeed");
    assert_eq!(result.paper_count, 1);
    assert_eq!(result.collection_count, 1);
    assert!(zip_out.exists());

    let file = File::open(&zip_out).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let manifest = read_zip_manifest(&mut archive);

    assert_eq!(manifest.collections.len(), 1);
    assert_eq!(manifest.collections[0].name, "Machine Learning");
    assert_eq!(manifest.papers.len(), 1);
    assert_eq!(manifest.papers[0].id, p1.id);
    assert!(archive.by_name(&manifest.papers[0].file_path).is_ok());
}

/// RT-44: Paper in multiple collections and deduplication.
#[test]
fn test_rt44_paper_in_multiple_collections_deduplication() {
    let dir = tempdir().expect("tempdir");
    let (conn, config) = setup_test_library(dir.path());

    let col_a = Collection {
        id: Uuid::now_v7(),
        name: "Col A".into(),
        parent_id: None,
    };
    let col_b = Collection {
        id: Uuid::now_v7(),
        name: "Col B".into(),
        parent_id: None,
    };
    CollectionRepo::insert(&conn, &col_a).unwrap();
    CollectionRepo::insert(&conn, &col_b).unwrap();

    let pdf_path = dir.path().join("shared.pdf");
    create_test_pdf(&pdf_path, "Shared Content");
    let pdf_bytes = std::fs::read(&pdf_path).unwrap();
    let hash = compute_sha256(&pdf_bytes);

    let p1 = Paper {
        id: Uuid::now_v7(),
        file_path: pdf_path.to_str().unwrap().into(),
        content_hash: hash,
        title: Some("Shared Paper".into()),
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
    CollectionRepo::add_paper(&conn, p1.id, col_a.id).unwrap();
    CollectionRepo::add_paper(&conn, p1.id, col_b.id).unwrap();

    let p2_path = dir.path().join("other.pdf");
    create_test_pdf(&p2_path, "Other Content");
    let p2_bytes = std::fs::read(&p2_path).unwrap();
    let p2 = Paper {
        id: Uuid::now_v7(),
        file_path: p2_path.to_str().unwrap().into(),
        content_hash: compute_sha256(&p2_bytes),
        title: Some("Other Paper".into()),
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
    PaperRepo::insert(&conn, &p2).unwrap();
    CollectionRepo::add_paper(&conn, p2.id, col_b.id).unwrap();

    let zip_out = dir.path().join("shared_export.zip");
    let options = ExportOptions {
        output_path: zip_out.clone(),
        overwrite: true,
        include_database: true,
        skip_missing_files: false,
    };

    let result = export_library(&conn, &config, &options).expect("export should succeed");
    assert_eq!(result.paper_count, 2);

    let file = File::open(&zip_out).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let manifest = read_zip_manifest(&mut archive);

    let m1 = manifest.papers.iter().find(|p| p.id == p1.id).unwrap();
    assert_eq!(m1.collections.len(), 2);
    assert!(m1.collections.contains(&col_a.id));
    assert!(m1.collections.contains(&col_b.id));

    let paper_files: Vec<_> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .filter(|name| name.starts_with("papers/"))
        .collect();
    assert_eq!(paper_files.len(), 2);
}
