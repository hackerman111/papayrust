use std::fs::File;
use tempfile::tempdir;
use uuid::Uuid;
use zip::ZipArchive;

use papyrus_core::db::{Collection, CollectionRepo, Paper, PaperRepo};
use papyrus_core::export::{export_collection, export_library, ExportOptions};

use super::helpers::{
    compute_sha256, create_test_pdf, read_zip_entry, read_zip_manifest, setup_test_library,
};

/// RT-45: Filename collision handling.
#[test]
fn test_rt45_filename_collision_handling() {
    let dir = tempdir().expect("tempdir");
    let (conn, config) = setup_test_library(dir.path());

    let dir1 = dir.path().join("sub1");
    let dir2 = dir.path().join("sub2");
    std::fs::create_dir_all(&dir1).unwrap();
    std::fs::create_dir_all(&dir2).unwrap();

    let p1_path = dir1.join("paper.pdf");
    let p2_path = dir2.join("paper.pdf");
    create_test_pdf(&p1_path, "Content for Paper Alpha");
    create_test_pdf(&p2_path, "Content for Paper Beta (Different)");

    let p1_bytes = std::fs::read(&p1_path).unwrap();
    let p2_bytes = std::fs::read(&p2_path).unwrap();

    let p1 = Paper {
        id: Uuid::now_v7(),
        file_path: p1_path.to_str().unwrap().into(),
        content_hash: compute_sha256(&p1_bytes),
        title: Some("Alpha".into()),
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
        title: Some("Beta".into()),
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

    let zip_out = dir.path().join("collision.zip");
    let options = ExportOptions {
        output_path: zip_out.clone(),
        overwrite: true,
        include_database: true,
        skip_missing_files: false,
    };

    export_library(&conn, &config, &options).expect("export should succeed");

    let file = File::open(&zip_out).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let manifest = read_zip_manifest(&mut archive);

    let m1 = manifest.papers.iter().find(|p| p.id == p1.id).unwrap();
    let m2 = manifest.papers.iter().find(|p| p.id == p2.id).unwrap();
    assert_ne!(m1.file_path, m2.file_path);

    let data1 = read_zip_entry(&mut archive, &m1.file_path);
    let data2 = read_zip_entry(&mut archive, &m2.file_path);
    assert_eq!(compute_sha256(&data1), p1.content_hash);
    assert_eq!(compute_sha256(&data2), p2.content_hash);
}

/// RT-46: Empty collection export.
#[test]
fn test_rt46_empty_collection_export() {
    let dir = tempdir().expect("tempdir");
    let (conn, config) = setup_test_library(dir.path());

    let empty_col = Collection {
        id: Uuid::now_v7(),
        name: "Empty".into(),
        parent_id: None,
    };
    CollectionRepo::insert(&conn, &empty_col).unwrap();

    let zip_out = dir.path().join("empty_col.zip");
    let options = ExportOptions {
        output_path: zip_out.clone(),
        overwrite: true,
        include_database: true,
        skip_missing_files: false,
    };

    let res = export_collection(&conn, &config, empty_col.id, &options)
        .expect("empty collection export succeeds");
    assert_eq!(res.paper_count, 0);
    assert_eq!(res.collection_count, 1);

    let file = File::open(&zip_out).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let manifest = read_zip_manifest(&mut archive);

    assert_eq!(manifest.papers.len(), 0);
    assert_eq!(manifest.collections.len(), 1);
    assert_eq!(manifest.collections[0].name, "Empty");
    assert!(archive.by_name(".library/library.db").is_ok());
}
