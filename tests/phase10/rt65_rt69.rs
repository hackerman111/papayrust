use std::fs::{self, File};
use tempfile::tempdir;
use zip::ZipArchive;

use papyrus_core::doctor::{run_doctor, DoctorVerdict};
use papyrus_core::export::{export_library, ExportOptions};
use papyrus_core::importer::import_paper;
use papyrus_core::metadata_editor::{update_metadata, UpdatePaperMetadata};

use super::helpers::{create_valid_pdf_with_text, setup_doctor_test_env};

#[test]
fn test_rt65_empty_library_export_and_doctor() {
    let tmp = tempdir().unwrap();
    let (config, conn) = setup_doctor_test_env(tmp.path());

    let report = run_doctor(&conn, &config, true);
    assert_eq!(report.verdict(), DoctorVerdict::Ok);
    assert!(report.is_ok());

    let zip_path = tmp.path().join("empty_library.zip");
    let opts = ExportOptions {
        output_path: zip_path.clone(),
        overwrite: true,
        include_database: true,
        skip_missing_files: false,
    };
    let res = export_library(&conn, &config, &opts);
    assert!(res.is_ok());
    assert_eq!(res.unwrap().paper_count, 0);
    assert!(zip_path.exists());
}

#[test]
fn test_rt66_zip_creation_error_rollback_temp() {
    let tmp = tempdir().unwrap();
    let (config, mut conn) = setup_doctor_test_env(tmp.path());
    let pdf_path = tmp.path().join("doc.pdf");
    create_valid_pdf_with_text(&pdf_path, "Atomicity test");
    import_paper(&mut conn, &config, &pdf_path, None).unwrap();
    fs::remove_file(&pdf_path).unwrap();

    let target_zip = tmp.path().join("failed_export.zip");
    let opts = ExportOptions {
        output_path: target_zip.clone(),
        overwrite: true,
        include_database: true,
        skip_missing_files: false,
    };

    let res = export_library(&conn, &config, &opts);
    assert!(res.is_err(), "Missing file mid-export must cause error");
    assert!(!target_zip.exists(), "Target zip must not exist on failure");

    for entry in fs::read_dir(tmp.path()).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().to_string();
        assert!(
            !name.starts_with(".tmp_export_") && !name.starts_with(".tmp_backup_"),
            "Leftover temp file found: {name}"
        );
    }
}

#[test]
fn test_rt67_io_failure_clean_rollback() {
    let tmp = tempdir().unwrap();
    let (config, mut conn) = setup_doctor_test_env(tmp.path());
    let pdf_path = tmp.path().join("paper.pdf");
    create_valid_pdf_with_text(&pdf_path, "Clean rollback on IO failure");
    import_paper(&mut conn, &config, &pdf_path, None).unwrap();

    let blocking_dir = tmp.path().join("target_dir");
    fs::create_dir(&blocking_dir).unwrap();

    let opts = ExportOptions {
        output_path: blocking_dir.clone(),
        overwrite: false,
        include_database: true,
        skip_missing_files: false,
    };

    let res = export_library(&conn, &config, &opts);
    assert!(
        res.is_err(),
        "Exporting over existing dir without overwrite fails"
    );
    assert!(
        blocking_dir.is_dir(),
        "Original directory must remain intact"
    );
}

#[test]
fn test_rt68_filename_collision_uuid_disambiguation() {
    let tmp = tempdir().unwrap();
    let (config, mut conn) = setup_doctor_test_env(tmp.path());

    let dir1 = tmp.path().join("dir1");
    let dir2 = tmp.path().join("dir2");
    fs::create_dir(&dir1).unwrap();
    fs::create_dir(&dir2).unwrap();

    let p1 = dir1.join("paper.pdf");
    let p2 = dir2.join("paper.pdf");
    create_valid_pdf_with_text(&p1, "Content One");
    create_valid_pdf_with_text(&p2, "Content Two (different hash)");

    let paper1 = import_paper(&mut conn, &config, &p1, None).unwrap();
    let paper2 = import_paper(&mut conn, &config, &p2, None).unwrap();
    assert_ne!(paper1.id, paper2.id);
    assert_ne!(paper1.content_hash, paper2.content_hash);

    let zip_path = tmp.path().join("collision.zip");
    let opts = ExportOptions {
        output_path: zip_path.clone(),
        overwrite: true,
        include_database: true,
        skip_missing_files: false,
    };
    let res = export_library(&conn, &config, &opts).unwrap();
    assert_eq!(res.paper_count, 2);

    let file = File::open(&zip_path).unwrap();
    let archive = ZipArchive::new(file).unwrap();
    assert_eq!(archive.len(), 4); // 2 papers + manifest + db
}

#[test]
fn test_rt69_unicode_titles_and_filenames() {
    let tmp = tempdir().unwrap();
    let (config, mut conn) = setup_doctor_test_env(tmp.path());

    let unicode_filename = tmp.path().join("тест_論文_🚀.pdf");
    create_valid_pdf_with_text(&unicode_filename, "Unicode test content");

    let paper = import_paper(&mut conn, &config, &unicode_filename, None).unwrap();

    let unicode_title = "Квантовые вычисления 🌌 & 深度学习 (2026)";
    let updated = update_metadata(
        &mut conn,
        paper.id,
        UpdatePaperMetadata {
            title: unicode_title.to_string(),
            authors: Some("Иванов И.И. & 张伟".into()),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    assert_eq!(updated.title.as_deref(), Some(unicode_title));

    let report = run_doctor(&conn, &config, false);
    assert_eq!(report.verdict(), DoctorVerdict::Ok);

    let zip_path = tmp.path().join("unicode_export.zip");
    let opts = ExportOptions {
        output_path: zip_path,
        overwrite: true,
        include_database: true,
        skip_missing_files: false,
    };
    let res = export_library(&conn, &config, &opts);
    assert!(res.is_ok(), "Unicode export must succeed without error");
}
