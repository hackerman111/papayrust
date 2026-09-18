use std::fs;

use papyrus_cli::run_with_args;
use papyrus_core::db::{open_database, PaperRepo, TocRepo};
use papyrus_core::toc::embed_toc_in_pdf;
use tempfile::tempdir;

use super::helpers::{compute_sha256, create_pdf_with_outline, setup_cli_test_env};

#[test]
fn test_rt58_cli_toc_embed_byte_identical() {
    let tmp = tempdir().unwrap();
    let (config, cfg_path) = setup_cli_test_env(tmp.path());
    let pdf_path = tmp.path().join("source.pdf");
    create_pdf_with_outline(&pdf_path, "Section 1");

    let cfg_str = cfg_path.to_str().unwrap();
    let pdf_str = pdf_path.to_str().unwrap();

    // 1. Add paper with TOC into the library
    run_with_args(&["papyrus", "--config", cfg_str, "add", pdf_str])
        .expect("import paper with outlines");

    let mut conn = open_database(&config.database_path).unwrap();
    let papers = PaperRepo::list(&conn).unwrap();
    assert_eq!(papers.len(), 1);
    let paper = &papers[0];
    let tocs = TocRepo::get_by_paper(&conn, paper.id).unwrap();
    assert_eq!(tocs.len(), 1);

    // 2. Materialize TOC via CLI: `papyrus toc embed <id>`
    let paper_id_str = paper.id.to_string();
    run_with_args(&[
        "papyrus",
        "--config",
        cfg_str,
        "toc",
        "embed",
        &paper_id_str,
    ])
    .expect("cli toc embed should succeed");

    let paper_after_cli = PaperRepo::get_by_id(&conn, paper.id)
        .unwrap()
        .expect("paper exists");
    let cli_ann_path = paper_after_cli
        .annotated_pdf_path
        .expect("annotated_pdf_path must be set by CLI");
    let cli_bytes = fs::read(&cli_ann_path).expect("read CLI-generated annotated PDF");
    let cli_hash = compute_sha256(&cli_bytes);

    // 3. Remove annotated PDF file from disk to ensure fresh generation
    fs::remove_file(&cli_ann_path).expect("remove annotated PDF");

    // 4. Materialize TOC via core/TUI pathway: `embed_toc_in_pdf`
    let core_ann_path =
        embed_toc_in_pdf(&mut conn, &config, paper.id).expect("core embed_toc_in_pdf succeeds");
    let core_bytes = fs::read(&core_ann_path).expect("read core-generated annotated PDF");
    let core_hash = compute_sha256(&core_bytes);

    // 5. Verify byte-for-byte identity and sha256 equivalence
    assert_eq!(
        cli_ann_path,
        core_ann_path.to_string_lossy(),
        "paths must match"
    );
    assert_eq!(
        cli_hash, core_hash,
        "SHA-256 hashes of CLI and TUI/core annotated PDFs must be byte-for-byte identical!"
    );
    assert_eq!(
        cli_bytes, core_bytes,
        "Raw bytes of CLI and TUI/core annotated PDFs must be identical!"
    );
}
