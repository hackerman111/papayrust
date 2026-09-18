use super::*;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use uuid::Uuid;

use crate::config::Config;
use crate::db::Paper;

fn dummy_paper(path: &str, annotated: Option<&str>) -> Paper {
    Paper {
        id: Uuid::now_v7(),
        file_path: path.to_string(),
        content_hash: "dummyhash".to_string(),
        title: Some("Sample Title".to_string()),
        authors: Some("Author Name".to_string()),
        year: Some(2024),
        journal: None,
        doi: None,
        abstract_text: None,
        text_path: None,
        annotated_pdf_path: annotated.map(ToString::to_string),
        toc_embedded_at: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    }
}

#[test]
fn test_build_open_command_default_template_with_page() {
    let (prog, args) = build_open_command(
        "zathura --page={page} {path}",
        Path::new("/papers/test.pdf"),
        Some(5),
    )
    .expect("build command");

    assert_eq!(prog, "zathura");
    assert_eq!(args, vec!["--page=5", "/papers/test.pdf"]);
}

#[test]
fn test_build_open_command_default_template_without_page() {
    let (prog, args) = build_open_command(
        "zathura --page={page} {path}",
        Path::new("/papers/test.pdf"),
        None,
    )
    .expect("build command");

    assert_eq!(prog, "zathura");
    assert_eq!(args, vec!["/papers/test.pdf"]);
}

#[test]
fn test_build_open_command_space_flag_without_page() {
    let (prog, args) = build_open_command(
        "viewer -p {page} {path}",
        Path::new("/papers/test.pdf"),
        None,
    )
    .expect("build command");

    assert_eq!(prog, "viewer");
    assert_eq!(args, vec!["/papers/test.pdf"]);
}

#[test]
fn test_build_open_command_path_only() {
    let (prog, args) =
        build_open_command("{path}", Path::new("/papers/test.pdf"), None).expect("build command");

    assert_eq!(prog, "/papers/test.pdf");
    assert!(args.is_empty());
}

#[test]
fn test_build_open_command_appends_path_when_missing() {
    let (prog, args) =
        build_open_command("zathura", Path::new("/papers/test.pdf"), None).expect("build command");

    assert_eq!(prog, "zathura");
    assert_eq!(args, vec!["/papers/test.pdf"]);
}

#[test]
fn test_build_open_command_invalid_page_zero() {
    let err = build_open_command(
        "zathura --page={page} {path}",
        Path::new("/papers/test.pdf"),
        Some(0),
    )
    .unwrap_err();

    assert_eq!(err, OpenerError::InvalidPage(0));
}

#[test]
fn test_build_open_command_invalid_template() {
    let err_empty = build_open_command("   ", Path::new("/papers/test.pdf"), None).unwrap_err();
    assert!(matches!(err_empty, OpenerError::InvalidTemplate(_)));

    let err_unclosed =
        build_open_command("zathura 'unclosed", Path::new("/papers/test.pdf"), None).unwrap_err();
    assert!(matches!(err_unclosed, OpenerError::InvalidTemplate(_)));
}

#[test]
fn test_resolve_paper_path_prefer_annotated() {
    let original_file = NamedTempFile::new().unwrap();
    let annotated_file = NamedTempFile::new().unwrap();

    let paper = dummy_paper(
        original_file.path().to_str().unwrap(),
        Some(annotated_file.path().to_str().unwrap()),
    );

    let mut config = Config::default();
    config.toc.prefer_annotated_copy = true;

    let resolved = resolve_paper_path(&paper, &config).unwrap();
    assert_eq!(resolved, annotated_file.path());

    config.toc.prefer_annotated_copy = false;
    let resolved_orig = resolve_paper_path(&paper, &config).unwrap();
    assert_eq!(resolved_orig, original_file.path());
}

#[test]
fn test_resolve_paper_path_annotated_missing_fallback_original() {
    let original_file = NamedTempFile::new().unwrap();
    let paper = dummy_paper(
        original_file.path().to_str().unwrap(),
        Some("/path/does/not/exist.pdf"),
    );

    let mut config = Config::default();
    config.toc.prefer_annotated_copy = true;

    let resolved = resolve_paper_path(&paper, &config).unwrap();
    assert_eq!(resolved, original_file.path());
}

#[test]
fn test_resolve_paper_path_file_not_found() {
    let paper = dummy_paper("/path/does/not/exist.pdf", None);
    let config = Config::default();

    let err = resolve_paper_path(&paper, &config).unwrap_err();
    assert_eq!(
        err,
        OpenerError::FileNotFound(PathBuf::from("/path/does/not/exist.pdf"))
    );
}

#[test]
fn test_open_paper_with_mock_runner() {
    let original_file = NamedTempFile::new().unwrap();
    let paper = dummy_paper(original_file.path().to_str().unwrap(), None);
    let config = Config::default();
    let runner = MockCommandRunner::new();

    open_paper(&paper, None, &config, &runner).expect("open paper");
    assert_eq!(runner.command_count(), 1);

    let (prog, args) = runner.last_command().unwrap();
    assert_eq!(prog, "zathura");
    assert_eq!(args, vec![original_file.path().to_str().unwrap()]);

    open_paper(&paper, Some(3), &config, &runner).expect("open paper at page 3");
    assert_eq!(runner.command_count(), 2);
    let (prog2, args2) = runner.last_command().unwrap();
    assert_eq!(prog2, "zathura");
    assert_eq!(
        args2,
        vec!["--page=3", original_file.path().to_str().unwrap()]
    );
}
