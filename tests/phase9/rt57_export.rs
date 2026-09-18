use std::fs::File;

use papyrus_cli::run_with_args;
use papyrus_core::db::{open_database, Collection, CollectionRepo, PaperRepo};
use tempfile::tempdir;
use uuid::Uuid;
use zip::ZipArchive;

use super::helpers::{create_test_pdf, setup_cli_test_env};

#[test]
fn test_rt57_cli_export_options_and_force() {
    let tmp = tempdir().unwrap();
    let (config, cfg_path) = setup_cli_test_env(tmp.path());
    let conn = open_database(&config.database_path).unwrap();

    let col_id = Uuid::now_v7();
    CollectionRepo::insert(
        &conn,
        &Collection {
            id: col_id,
            name: "Robotics".into(),
            parent_id: None,
        },
    )
    .unwrap();

    let p1_path = tmp.path().join("p1.pdf");
    create_test_pdf(&p1_path, "Robotics Paper");
    let cfg = cfg_path.to_str().unwrap();

    let p1 = p1_path.to_str().unwrap();
    run_with_args(&["papyrus", "--config", cfg, "add", p1, "-c", "Robotics"]).unwrap();

    let export_all = tmp.path().join("library_export.zip");
    let all_str = export_all.to_str().unwrap();

    // 1. Export library to specified output path
    run_with_args(&["papyrus", "--config", cfg, "export", "-o", all_str])
        .expect("export library should succeed");
    assert!(export_all.exists());

    // 2. Fail when destination exists and --force is not specified
    let err = run_with_args(&["papyrus", "--config", cfg, "export", "-o", all_str]).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("already exists"));

    // 3. Overwrite destination when --force is provided
    run_with_args(&[
        "papyrus", "--config", cfg, "export", "-o", all_str, "--force",
    ])
    .unwrap();

    // 4. Export specific collection by name
    let export_col = tmp.path().join("col_export.zip");
    let col_str = export_col.to_str().unwrap();
    run_with_args(&[
        "papyrus", "--config", cfg, "export", "-c", "Robotics", "-o", col_str,
    ])
    .unwrap();

    let mut archive = ZipArchive::new(File::open(&export_col).unwrap()).unwrap();
    assert!(archive.by_name("manifest.json").is_ok());

    // 5. Test missing file warning with --skip-missing
    let papers = PaperRepo::list(&conn).unwrap();
    let mut broken = papers[0].clone();
    broken.id = Uuid::now_v7();
    broken.content_hash = "missinghash".into();
    broken.file_path = tmp.path().join("nonexistent.pdf").to_str().unwrap().into();
    PaperRepo::insert(&conn, &broken).unwrap();

    let export_warn = tmp.path().join("warn_export.zip");
    let warn_str = export_warn.to_str().unwrap();
    run_with_args(&[
        "papyrus",
        "--config",
        cfg,
        "export",
        "-o",
        warn_str,
        "--skip-missing",
    ])
    .expect("export with --skip-missing should succeed");
    assert!(export_warn.exists());
}
