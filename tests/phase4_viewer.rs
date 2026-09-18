use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use papyrus_core::config::Config;
use papyrus_core::db::{Paper, TocEntry, TocSource};
use papyrus_core::opener::{
    build_open_command, open_paper, resolve_paper_path, MockCommandRunner, OpenerError,
};
use papyrus_core::Action;
use papyrus_tui::{
    map_key_event, map_key_event_for_app, map_key_event_with_context, ActivePanel, App,
    CollectionItem,
};
use tempfile::{tempdir, NamedTempFile};
use uuid::Uuid;

fn create_sample_paper(file_path: &str, title: &str) -> Paper {
    Paper {
        id: Uuid::now_v7(),
        file_path: file_path.to_string(),
        content_hash: format!("hash_{}", Uuid::now_v7()),
        title: Some(title.to_string()),
        authors: Some("Author Name".to_string()),
        year: Some(2024),
        journal: Some("Journal of Systems".to_string()),
        doi: Some("10.1234/test".to_string()),
        abstract_text: Some("Sample abstract".to_string()),
        text_path: None,
        annotated_pdf_path: None,
        toc_embedded_at: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    }
}

fn create_sample_toc(paper_id: Uuid, title: &str, page_number: u32, order_index: i32) -> TocEntry {
    TocEntry {
        id: Uuid::now_v7(),
        paper_id,
        parent_id: None,
        title: title.to_string(),
        page_number,
        order_index,
        source: TocSource::Auto,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    }
}

/// RT-17: Open paper with default template (`zathura --page={page} {path}` or `{path}`),
/// verifies command structure and verifies that annotated copy is preferred when available.
#[test]
fn test_rt_17_open_paper_command_structure_and_annotated_preference() {
    let original_file = NamedTempFile::new().expect("create original temp file");
    let original_path = original_file
        .path()
        .to_str()
        .expect("valid path")
        .to_string();

    let mut paper = create_sample_paper(&original_path, "Attention Is All You Need");
    let config = Config::default();
    let runner = MockCommandRunner::new();

    // 1. Open with default configuration (page = None)
    open_paper(&paper, None, &config, &runner).expect("open paper should succeed");
    assert_eq!(runner.command_count(), 1);

    let (program, args) = runner.last_command().expect("recorded command");
    assert_eq!(program, "zathura");
    assert_eq!(args, vec![original_path.clone()]);

    // 2. Open with custom path-only template: `{path}`
    let mut custom_config = config.clone();
    custom_config.pdf.viewer = "{path}".to_string();
    runner.clear();

    open_paper(&paper, None, &custom_config, &runner).expect("open paper with path-only template");
    assert_eq!(runner.command_count(), 1);
    let (program, args) = runner.last_command().expect("recorded command");
    assert_eq!(program, original_path);
    assert!(args.is_empty());

    // 3. Prefer annotated copy when available and file exists on disk
    let annotated_file = NamedTempFile::new().expect("create annotated temp file");
    let annotated_path = annotated_file
        .path()
        .to_str()
        .expect("valid path")
        .to_string();

    paper.annotated_pdf_path = Some(annotated_path.clone());
    let mut annotated_pref_config = Config::default();
    annotated_pref_config.toc.prefer_annotated_copy = true;
    runner.clear();

    open_paper(&paper, None, &annotated_pref_config, &runner)
        .expect("open paper with preferred annotated copy");
    assert_eq!(runner.command_count(), 1);
    let (program, args) = runner.last_command().expect("recorded command");
    assert_eq!(program, "zathura");
    assert_eq!(args, vec![annotated_path.clone()]);

    // 4. If prefer_annotated_copy is false, use original paper.file_path even if annotated copy exists
    annotated_pref_config.toc.prefer_annotated_copy = false;
    runner.clear();

    open_paper(&paper, None, &annotated_pref_config, &runner)
        .expect("open paper when prefer_annotated_copy is false");
    let (program, args) = runner.last_command().expect("recorded command");
    assert_eq!(program, "zathura");
    assert_eq!(args, vec![original_path.clone()]);

    // 5. If annotated file does not exist on disk, fall back to original paper.file_path
    paper.annotated_pdf_path = Some("/tmp/non_existent_annotated_copy_12345.pdf".to_string());
    annotated_pref_config.toc.prefer_annotated_copy = true;
    runner.clear();

    open_paper(&paper, None, &annotated_pref_config, &runner)
        .expect("open paper with missing annotated file fallback");
    let (program, args) = runner.last_command().expect("recorded command");
    assert_eq!(program, "zathura");
    assert_eq!(args, vec![original_path]);
}

/// RT-18: Open paper at specific page, verifies `{page}` substitution and safe argument parsing.
#[test]
fn test_rt_18_open_paper_at_specific_page_substitution() {
    let temp_file = NamedTempFile::new().expect("create temp file");
    let file_path = temp_file.path().to_str().expect("valid path").to_string();

    let paper = create_sample_paper(&file_path, "Deep Residual Learning");
    let config = Config::default();
    let runner = MockCommandRunner::new();

    // 1. Default template with page substitution (e.g. page 42)
    open_paper(&paper, Some(42), &config, &runner).expect("open at page 42");
    assert_eq!(runner.command_count(), 1);
    let (program, args) = runner.last_command().expect("command");
    assert_eq!(program, "zathura");
    assert_eq!(args, vec!["--page=42", file_path.as_str()]);

    // 2. Open at page 1
    runner.clear();
    open_paper(&paper, Some(1), &config, &runner).expect("open at page 1");
    let (program, args) = runner.last_command().expect("command");
    assert_eq!(program, "zathura");
    assert_eq!(args, vec!["--page=1", file_path.as_str()]);

    // 3. Custom template with separate flag and argument: "viewer -p {page} '{path}'"
    let mut custom_config = config.clone();
    custom_config.pdf.page_open_template = "mupdf -p {page} '{path}'".to_string();
    runner.clear();

    open_paper(&paper, Some(15), &custom_config, &runner).expect("open with custom template");
    let (program, args) = runner.last_command().expect("command");
    assert_eq!(program, "mupdf");
    assert_eq!(args, vec!["-p", "15", file_path.as_str()]);

    // 4. File path containing spaces and special characters must not be split into multiple arguments
    let dir = tempdir().expect("temp dir");
    let path_with_spaces = dir.path().join("Paper With Spaces & Special [2024].pdf");
    File::create(&path_with_spaces)
        .expect("create file with spaces")
        .write_all(b"%PDF-1.4 dummy")
        .expect("write dummy bytes");

    let space_paper = create_sample_paper(
        path_with_spaces.to_str().expect("valid path"),
        "Paper With Spaces",
    );
    runner.clear();

    open_paper(&space_paper, Some(7), &config, &runner).expect("open file with spaces");
    let (program, args) = runner.last_command().expect("command");
    assert_eq!(program, "zathura");
    assert_eq!(
        args,
        vec!["--page=7", path_with_spaces.to_str().expect("path str")]
    );
}

/// RT-19: Error handling when file does not exist, when page is invalid (e.g. 0), or when runner fails.
#[test]
fn test_rt_19_error_handling() {
    let runner = MockCommandRunner::new();
    let config = Config::default();

    // 1. File does not exist on disk
    let missing_path = PathBuf::from("/non/existent/papyrus_test_file_9999.pdf");
    let missing_paper =
        create_sample_paper(missing_path.to_str().expect("path string"), "Ghost Paper");

    let err = open_paper(&missing_paper, None, &config, &runner).unwrap_err();
    assert_eq!(err, OpenerError::FileNotFound(missing_path.clone()));

    let resolve_err = resolve_paper_path(&missing_paper, &config).unwrap_err();
    assert_eq!(resolve_err, OpenerError::FileNotFound(missing_path));

    // 2. Invalid page number 0
    let temp_file = NamedTempFile::new().expect("temp file");
    let existing_paper =
        create_sample_paper(temp_file.path().to_str().expect("path str"), "Valid Paper");

    let page_err = open_paper(&existing_paper, Some(0), &config, &runner).unwrap_err();
    assert_eq!(page_err, OpenerError::InvalidPage(0));

    let cmd_err =
        build_open_command(&config.pdf.page_open_template, temp_file.path(), Some(0)).unwrap_err();
    assert_eq!(cmd_err, OpenerError::InvalidPage(0));

    // 3. Command runner failure simulation
    runner.set_fail_with("Permission denied (os error 13)");
    let launch_err = open_paper(&existing_paper, Some(1), &config, &runner).unwrap_err();
    assert_eq!(
        launch_err,
        OpenerError::LaunchFailed("Permission denied (os error 13)".to_string())
    );

    // 4. Invalid template syntax (empty or unclosed quote)
    let empty_err = build_open_command("   ", temp_file.path(), None).unwrap_err();
    assert!(matches!(empty_err, OpenerError::InvalidTemplate(_)));

    let quote_err =
        build_open_command("zathura --opt 'unclosed", temp_file.path(), None).unwrap_err();
    assert!(matches!(quote_err, OpenerError::InvalidTemplate(_)));
}

/// Test TUI dispatching Action::Open and Action::OpenAtPage with MockCommandRunner.
#[test]
fn test_tui_dispatch_open_and_open_at_page() {
    let temp_file = NamedTempFile::new().expect("temp file");
    let file_path = temp_file.path().to_str().expect("path str").to_string();

    let paper1 = create_sample_paper(&file_path, "Attention Mechanism");
    let paper_id = paper1.id;
    let col_id = Uuid::now_v7();

    let mock_runner = MockCommandRunner::new();

    let mut app = App::new().with_runner(mock_runner.clone());
    app.add_collection(
        CollectionItem::new(Some(col_id), "Deep Learning", 1),
        vec![paper1],
    );
    app.set_paper_tocs(
        paper_id,
        vec![
            create_sample_toc(paper_id, "1. Introduction", 1, 0),
            create_sample_toc(paper_id, "2. Background", 5, 1),
            create_sample_toc(paper_id, "3. Architecture", 12, 2),
        ],
    );

    // Switch focus to Papers panel
    app.active_panel = ActivePanel::Papers;

    // Verify key mapping in Papers panel:
    let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(map_key_event_for_app(enter_key, &app), Some(Action::Open));

    let o_key = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE);
    assert_eq!(map_key_event_for_app(o_key, &app), Some(Action::Open));

    // Dispatch Action::Open
    app.dispatch(Action::Open);
    assert_eq!(mock_runner.command_count(), 1);

    let (prog, args) = mock_runner.last_command().expect("command executed");
    assert_eq!(prog, "zathura");
    assert_eq!(args, vec![file_path.clone()]);
    assert!(app
        .status_message
        .as_ref()
        .expect("status message set")
        .contains("Opened Attention Mechanism"));

    // Switch to Details panel and select TOC entry at index 2 (page 12)
    app.active_panel = ActivePanel::Details;
    app.selected_toc = 2;

    // Verify key mapping in Details panel:
    let enter_toc = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(enter_toc, &app),
        Some(Action::OpenAtPage(12))
    );

    // Dispatch Action::OpenAtPage(12)
    app.dispatch(Action::OpenAtPage(12));
    assert_eq!(mock_runner.command_count(), 2);

    let (prog, args) = mock_runner.last_command().expect("command executed");
    assert_eq!(prog, "zathura");
    assert_eq!(args, vec!["--page=12", file_path.as_str()]);
    assert!(app
        .status_message
        .as_ref()
        .expect("status message set")
        .contains("Opened Attention Mechanism at page 12"));

    // Test error handling in TUI when opening a non-existent file
    let missing_paper =
        create_sample_paper("/tmp/missing_file_for_tui_test_8888.pdf", "Missing Paper");
    let mut err_app = App::new().with_runner(mock_runner.clone());
    err_app.add_collection(
        CollectionItem::new(Some(Uuid::now_v7()), "Missing", 1),
        vec![missing_paper],
    );
    err_app.active_panel = ActivePanel::Papers;

    let pre_count = mock_runner.command_count();
    err_app.dispatch(Action::Open);
    assert_eq!(
        mock_runner.command_count(),
        pre_count,
        "No command should be executed for missing file"
    );
    assert!(err_app
        .status_message
        .as_ref()
        .expect("error status set")
        .contains("Failed to open: file not found"));

    // Test dispatching Open when no paper is selected
    let mut empty_app = App::new();
    empty_app.dispatch(Action::Open);
    assert_eq!(
        empty_app.status_message.as_deref(),
        Some("No paper selected to open")
    );
}

/// Test fallback and backward-compatibility of map_key_event.
#[test]
fn test_map_key_event_backward_compatibility() {
    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(map_key_event(enter), Some(Action::Open));

    let o_key = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE);
    assert_eq!(map_key_event(o_key), Some(Action::Open));

    let enter_col = map_key_event_with_context(enter, ActivePanel::Collections, None);
    assert_eq!(enter_col, None);

    let enter_details_no_toc = map_key_event_with_context(enter, ActivePanel::Details, None);
    assert_eq!(enter_details_no_toc, None);

    let enter_details_with_page = map_key_event_with_context(enter, ActivePanel::Details, Some(99));
    assert_eq!(enter_details_with_page, Some(Action::OpenAtPage(99)));
}
