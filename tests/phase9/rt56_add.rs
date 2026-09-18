use std::fs;

use papyrus_cli::run_with_args;
use papyrus_core::db::{open_database, Collection, CollectionRepo, PaperRepo, TocRepo};
use papyrus_core::search::SearchIndex;
use tempfile::tempdir;
use uuid::Uuid;

use super::helpers::{
    compute_sha256, create_pdf_with_outline, create_test_pdf, setup_cli_test_env,
};

#[test]
fn test_rt56_cli_add_equivalent_to_core() {
    let tmp = tempdir().unwrap();
    let (config, cfg_path) = setup_cli_test_env(tmp.path());
    let pdf_path = tmp.path().join("quantum.pdf");
    create_pdf_with_outline(&pdf_path, "Introduction");

    let (cfg, pdf) = (cfg_path.to_str().unwrap(), pdf_path.to_str().unwrap());
    run_with_args(&["papyrus", "--config", cfg, "add", pdf]).expect("cli add succeeds");

    let conn = open_database(&config.database_path).unwrap();
    let papers = PaperRepo::list(&conn).unwrap();
    assert_eq!(papers.len(), 1);

    let paper = &papers[0];
    assert_eq!(
        paper.content_hash,
        compute_sha256(&fs::read(&pdf_path).unwrap())
    );
    assert_eq!(paper.file_path, pdf);

    let tocs = TocRepo::get_by_paper(&conn, paper.id).unwrap();
    assert_eq!(tocs.len(), 1);
    assert_eq!(tocs[0].title, "Introduction");

    let search_dir = config.library_path.join(".papyrus").join("search");
    let search_index = SearchIndex::open_or_create(&search_dir).unwrap();
    let results = search_index.search("quantum", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, paper.id);
}

#[test]
fn test_rt56_cli_add_deduplication() {
    let tmp = tempdir().unwrap();
    let (config, cfg_path) = setup_cli_test_env(tmp.path());
    let pdf_path = tmp.path().join("paper.pdf");
    create_test_pdf(&pdf_path, "Unique Content 42");

    let (cfg, pdf) = (cfg_path.to_str().unwrap(), pdf_path.to_str().unwrap());
    run_with_args(&["papyrus", "--config", cfg, "add", pdf]).expect("first add succeeds");

    let conn = open_database(&config.database_path).unwrap();
    assert_eq!(PaperRepo::list(&conn).unwrap().len(), 1);

    let err = run_with_args(&["papyrus", "--config", cfg, "add", pdf]).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("duplicate"));
    assert_eq!(PaperRepo::list(&conn).unwrap().len(), 1);
}

#[test]
fn test_rt56_cli_add_with_collection() {
    let tmp = tempdir().unwrap();
    let (config, cfg_path) = setup_cli_test_env(tmp.path());
    let conn = open_database(&config.database_path).unwrap();
    let (col1_id, col2_id) = (Uuid::now_v7(), Uuid::now_v7());
    CollectionRepo::insert(
        &conn,
        &Collection {
            id: col1_id,
            name: "ML".into(),
            parent_id: None,
        },
    )
    .unwrap();
    CollectionRepo::insert(
        &conn,
        &Collection {
            id: col2_id,
            name: "Physics".into(),
            parent_id: None,
        },
    )
    .unwrap();

    let (pdf1, pdf2) = (tmp.path().join("p1.pdf"), tmp.path().join("p2.pdf"));
    create_test_pdf(&pdf1, "Machine Learning 101");
    create_test_pdf(&pdf2, "Quantum Field Theory");

    let cfg = cfg_path.to_str().unwrap();
    let (p1, p2, c2) = (
        pdf1.to_str().unwrap(),
        pdf2.to_str().unwrap(),
        col2_id.to_string(),
    );
    run_with_args(&["papyrus", "--config", cfg, "add", p1, "--collection", "ML"]).unwrap();
    assert_eq!(CollectionRepo::get_papers(&conn, col1_id).unwrap().len(), 1);

    run_with_args(&["papyrus", "--config", cfg, "add", p2, "-c", &c2]).unwrap();
    assert_eq!(CollectionRepo::get_papers(&conn, col2_id).unwrap().len(), 1);
}

#[test]
fn test_rt56_cli_add_no_extract() {
    let tmp = tempdir().unwrap();
    let (config, cfg_path) = setup_cli_test_env(tmp.path());
    let pdf_path = tmp.path().join("toc_paper.pdf");
    create_pdf_with_outline(&pdf_path, "Chapter 1");

    let (cfg, pdf) = (cfg_path.to_str().unwrap(), pdf_path.to_str().unwrap());
    run_with_args(&["papyrus", "--config", cfg, "add", pdf, "--no-extract"])
        .expect("add with --no-extract");

    let conn = open_database(&config.database_path).unwrap();
    let papers = PaperRepo::list(&conn).unwrap();
    assert_eq!(papers.len(), 1);
    assert!(TocRepo::get_by_paper(&conn, papers[0].id)
        .unwrap()
        .is_empty());
}
