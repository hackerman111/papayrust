//! Phase 11: End-to-end acceptance scenario (7 steps).

use std::fs::{self, File};
use std::io::Read;

use papyrus_cli::run_with_args;
use papyrus_core::db::models::TocSource;
use papyrus_core::db::{open_database, Collection, CollectionRepo, PaperRepo, TocRepo};
use papyrus_core::doctor::{run_doctor, DoctorVerdict};
use papyrus_core::search::SearchIndex;
use papyrus_core::toc::add_entry;
use tempfile::tempdir;
use uuid::Uuid;
use zip::ZipArchive;

#[path = "phase11/helpers.rs"]
mod helpers;

use helpers::{cli, compute_sha256, create_multi_page_pdf, setup_acceptance_env};

#[test]
fn test_phase11_end_to_end_acceptance_scenario() {
    let tmp = tempdir().unwrap();
    let (config, cfg_path) = setup_acceptance_env(tmp.path());
    let cfg = cfg_path.to_str().unwrap();

    // Step 1: Create collections
    let conn = open_database(&config.database_path).unwrap();
    let col1 = Collection {
        id: Uuid::now_v7(),
        name: "Quantum Physics".into(),
        parent_id: None,
    };
    let col2 = Collection {
        id: Uuid::now_v7(),
        name: "Computer Science".into(),
        parent_id: None,
    };
    CollectionRepo::insert(&conn, &col1).unwrap();
    CollectionRepo::insert(&conn, &col2).unwrap();
    drop(conn);

    // Step 2: Add papers (including deduplication check)
    let pdf1_path = tmp.path().join("quantum_paper.pdf");
    let pdf2_path = tmp.path().join("neural_paper.pdf");
    create_multi_page_pdf(&pdf1_path, 5, "Quantum Computing and Entanglement");
    create_multi_page_pdf(&pdf2_path, 5, "Neural Networks and Backpropagation");
    let (p1, p2) = (pdf1_path.to_str().unwrap(), pdf2_path.to_str().unwrap());

    cli(cfg, &["add", p1, "-c", "Quantum Physics"]);
    cli(cfg, &["add", p2, "-c", "Computer Science"]);

    let dup_err = run_with_args(&["papyrus", "--config", cfg, "add", p1]).unwrap_err();
    assert!(dup_err.to_string().to_lowercase().contains("duplicate"));

    let conn = open_database(&config.database_path).unwrap();
    let papers = PaperRepo::list(&conn).unwrap();
    assert_eq!(papers.len(), 2);
    let paper1 = papers.iter().find(|p| p.file_path == p1).unwrap();
    let paper1_id = paper1.id;
    let p1_id_str = paper1_id.to_string();
    drop(conn);

    // Step 3: Full-text search in Tantivy
    let search_dir = config.library_path.join(".papyrus").join("search");
    let search_index = SearchIndex::open_or_create(&search_dir).unwrap();
    let q_res = search_index.search("quantum", 10).unwrap();
    assert_eq!(q_res.len(), 1);
    assert_eq!(q_res[0].id, paper1_id);
    assert_eq!(search_index.search("neural", 10).unwrap().len(), 1);

    // Step 4: Import TOC from text file
    let toc_path = tmp.path().join("toc.txt");
    fs::write(
        &toc_path,
        "Introduction 1\nAlgorithms 2\n    Grover 2\n    Shor 3\nConclusion 4\n",
    )
    .unwrap();
    let toc_str = toc_path.to_str().unwrap();
    cli(cfg, &["toc", "import", &p1_id_str, "--from-text", toc_str]);

    let mut conn = open_database(&config.database_path).unwrap();
    assert_eq!(TocRepo::get_by_paper(&conn, paper1_id).unwrap().len(), 5);

    // Step 5: Edit TOC and embed into PDF
    add_entry(
        &mut conn,
        paper1_id,
        None,
        "Appendix".into(),
        5,
        TocSource::Manual,
    )
    .unwrap();
    drop(conn);

    cli(cfg, &["toc", "embed", &p1_id_str]);
    let conn = open_database(&config.database_path).unwrap();
    let p1_after = PaperRepo::get_by_id(&conn, paper1_id).unwrap().unwrap();
    let ann_path_str = p1_after.annotated_pdf_path.expect("annotated copy set");
    assert!(fs::metadata(&ann_path_str).is_ok());
    drop(conn);

    // Step 6: Application restart and doctor health check
    cli(cfg, &["doctor", "--full"]);
    let conn = open_database(&config.database_path).unwrap();
    let report = run_doctor(&conn, &config, true);
    assert_eq!(report.verdict(), DoctorVerdict::Ok);
    assert!(report.is_ok());

    // Step 7: Export to ZIP and verify archive contents
    let zip_out = tmp.path().join("acceptance_backup.zip");
    let zip_str = zip_out.to_str().unwrap();
    cli(cfg, &["export", "-o", zip_str]);

    let file = File::open(&zip_out).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();

    let manifest_json = {
        let mut f = archive.by_name("manifest.json").unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        s
    };
    assert!(
        manifest_json.contains("Quantum Physics") && manifest_json.contains("Computer Science")
    );

    let db_bytes = {
        let mut f = archive.by_name(".library/library.db").unwrap();
        let mut b = Vec::new();
        f.read_to_end(&mut b).unwrap();
        b
    };
    let ext_db = tmp.path().join("ext.db");
    fs::write(&ext_db, &db_bytes).unwrap();
    let db_conn = rusqlite::Connection::open(&ext_db).unwrap();
    let check: String = db_conn
        .query_row("PRAGMA integrity_check;", [], |r| r.get(0))
        .unwrap();
    assert_eq!(check, "ok");

    let p1_bytes = fs::read(&pdf1_path).unwrap();
    assert_eq!(compute_sha256(&p1_bytes), paper1.content_hash);

    let ann_hash = compute_sha256(&fs::read(&ann_path_str).unwrap());
    let ann_zip_bytes = {
        let mut f = archive
            .by_name("annotated/quantum_paper.annotated.pdf")
            .unwrap();
        let mut b = Vec::new();
        f.read_to_end(&mut b).unwrap();
        b
    };
    assert_eq!(compute_sha256(&ann_zip_bytes), ann_hash);
}
