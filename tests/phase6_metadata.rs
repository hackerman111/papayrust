use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use papyrus_core::db::{apply_migrations, open_in_memory, Paper, PaperRepo};
use papyrus_core::metadata_editor::{update_metadata, MetadataError, UpdatePaperMetadata};
use papyrus_core::search::SearchIndex;
use papyrus_core::Action;
use papyrus_tui::{map_key_event, map_key_event_for_app, render, App, CollectionItem};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use rusqlite::Connection;
use std::sync::Arc;
use uuid::Uuid;

fn setup_db() -> Connection {
    let mut conn = open_in_memory().expect("open in-memory db");
    apply_migrations(&mut conn).expect("apply schema migrations");
    conn
}

fn create_test_paper(
    id: Uuid,
    title: &str,
    authors: &str,
    year: i64,
    journal: &str,
    doi: &str,
    abstract_text: &str,
) -> Paper {
    Paper {
        id,
        file_path: format!("/papers/{id}.pdf"),
        content_hash: format!("hash_{id}"),
        title: Some(title.to_string()),
        authors: Some(authors.to_string()),
        year: Some(year),
        journal: Some(journal.to_string()),
        doi: Some(doi.to_string()),
        abstract_text: Some(abstract_text.to_string()),
        text_path: None,
        annotated_pdf_path: Some(format!("/papers/{id}_annotated.pdf")),
        toc_embedded_at: Some("2020-01-01T00:00:00Z".to_string()),
        created_at: "2020-01-01T00:00:00Z".to_string(),
        updated_at: "2020-01-01T00:00:00Z".to_string(),
    }
}

/// RT-25: Metadata editing in DB updates all fields, sets new updated_at,
/// and updates Tantivy search index so paper is discoverable by new metadata and no longer matches old metadata.
#[test]
fn test_rt_25_metadata_editing_db_and_search_index() {
    let mut conn = setup_db();
    let search_index = SearchIndex::create_in_ram().expect("create in-ram index");

    let paper_id = Uuid::now_v7();
    let paper = create_test_paper(
        paper_id,
        "Old Title: Symbolic Artificial Intelligence",
        "John McCarthy",
        1960,
        "Communications of the ACM",
        "10.1145/367177.367199",
        "LISP and symbolic computation logic for early reasoning systems.",
    );

    PaperRepo::insert(&conn, &paper).expect("insert original paper");
    search_index
        .index_paper(
            &paper,
            Some("recursive functions of symbolic expressions and their computation by machine"),
        )
        .expect("index paper");

    // Verify initial search finds old terms and does not find new terms
    let initial_symbolic = search_index.search("Symbolic", 10).expect("search");
    assert_eq!(initial_symbolic.len(), 1);
    assert_eq!(initial_symbolic[0].id, paper_id);

    let initial_mccarthy = search_index.search("McCarthy", 10).expect("search");
    assert_eq!(initial_mccarthy.len(), 1);

    let initial_transformers = search_index.search("Transformers", 10).expect("search");
    assert!(initial_transformers.is_empty());

    // Perform metadata update
    let update = UpdatePaperMetadata {
        title: "Modern Deep Learning with Transformers".to_string(),
        authors: Some("Ashish Vaswani and Noam Shazeer".to_string()),
        year: Some(2017),
        journal: Some("NeurIPS Proceedings".to_string()),
        doi: Some("10.5555/3295222.3295349".to_string()),
        abstract_text: Some(
            "The dominant sequence transduction models are based on complex recurrent or convolutional neural networks."
                .to_string(),
        ),
    };

    let updated_paper = update_metadata(&mut conn, paper_id, update, Some(&search_index))
        .expect("update_metadata must succeed");

    // 1. Verify returned paper struct
    assert_eq!(
        updated_paper.title.as_deref(),
        Some("Modern Deep Learning with Transformers")
    );
    assert_eq!(
        updated_paper.authors.as_deref(),
        Some("Ashish Vaswani and Noam Shazeer")
    );
    assert_eq!(updated_paper.year, Some(2017));
    assert_eq!(
        updated_paper.journal.as_deref(),
        Some("NeurIPS Proceedings")
    );
    assert_eq!(
        updated_paper.doi.as_deref(),
        Some("10.5555/3295222.3295349")
    );
    assert_eq!(
        updated_paper.abstract_text.as_deref(),
        Some("The dominant sequence transduction models are based on complex recurrent or convolutional neural networks.")
    );
    assert_ne!(
        updated_paper.updated_at, "2020-01-01T00:00:00Z",
        "updated_at timestamp must be refreshed"
    );
    assert_eq!(updated_paper.created_at, "2020-01-01T00:00:00Z");
    assert_eq!(updated_paper.file_path, format!("/papers/{paper_id}.pdf"));
    assert_eq!(updated_paper.content_hash, format!("hash_{paper_id}"));

    // 2. Verify persisted paper in SQLite database
    let db_paper = PaperRepo::get_by_id(&conn, paper_id)
        .expect("query paper by id")
        .expect("paper must exist in db");
    assert_eq!(
        db_paper.title.as_deref(),
        Some("Modern Deep Learning with Transformers")
    );
    assert_eq!(
        db_paper.authors.as_deref(),
        Some("Ashish Vaswani and Noam Shazeer")
    );
    assert_eq!(db_paper.year, Some(2017));
    assert_eq!(db_paper.journal.as_deref(), Some("NeurIPS Proceedings"));
    assert_eq!(db_paper.doi.as_deref(), Some("10.5555/3295222.3295349"));
    assert_eq!(
        db_paper.abstract_text.as_deref(),
        Some("The dominant sequence transduction models are based on complex recurrent or convolutional neural networks.")
    );
    assert_ne!(db_paper.updated_at, "2020-01-01T00:00:00Z");
    assert_eq!(db_paper.created_at, "2020-01-01T00:00:00Z");
    assert_eq!(db_paper.file_path, format!("/papers/{paper_id}.pdf"));
    assert_eq!(db_paper.content_hash, format!("hash_{paper_id}"));
    assert_eq!(
        db_paper.annotated_pdf_path.as_deref(),
        Some(format!("/papers/{paper_id}_annotated.pdf").as_str())
    );
    assert_eq!(
        db_paper.toc_embedded_at.as_deref(),
        Some("2020-01-01T00:00:00Z")
    );

    // 3. Verify Tantivy full-text index was updated
    // New metadata is discoverable
    let new_title_matches = search_index.search("Transformers", 10).expect("search");
    assert_eq!(new_title_matches.len(), 1);
    assert_eq!(new_title_matches[0].id, paper_id);

    let new_author_matches = search_index.search("Vaswani", 10).expect("search");
    assert_eq!(new_author_matches.len(), 1);
    assert_eq!(new_author_matches[0].id, paper_id);

    let new_abstract_matches = search_index.search("transduction", 10).expect("search");
    assert_eq!(new_abstract_matches.len(), 1);
    assert_eq!(new_abstract_matches[0].id, paper_id);

    // Old metadata no longer matches
    let old_title_matches = search_index.search("Symbolic", 10).expect("search");
    assert!(
        old_title_matches.is_empty(),
        "Old title term 'Symbolic' must no longer match"
    );

    let old_author_matches = search_index.search("McCarthy", 10).expect("search");
    assert!(
        old_author_matches.is_empty(),
        "Old author term 'McCarthy' must no longer match"
    );
}

/// RT-26: Validation rules (empty title rejected, invalid year rejected,
/// unchanged fields preserved, TUI edit modal dispatching and state updates verified).
#[test]
fn test_rt_26_validation_rules_and_tui_modal() {
    let mut conn = setup_db();
    let paper_id = Uuid::now_v7();
    let paper = create_test_paper(
        paper_id,
        "Foundations of Cryptography",
        "Oded Goldreich",
        2001,
        "Cambridge University Press",
        "10.1017/CBO9780511546891",
        "Basic tools of modern cryptography including pseudorandomness.",
    );
    PaperRepo::insert(&conn, &paper).expect("insert paper");

    // --- Validation Rules ---
    // 1. Empty title rejected
    let empty_title_update = UpdatePaperMetadata {
        title: "".to_string(),
        ..Default::default()
    };
    let err = update_metadata(&mut conn, paper_id, empty_title_update, None).unwrap_err();
    assert!(matches!(err, MetadataError::Validation(ref msg) if msg == "Title cannot be empty"));

    // Whitespace-only title rejected
    let ws_title_update = UpdatePaperMetadata {
        title: "   \n\t  ".to_string(),
        ..Default::default()
    };
    let err = update_metadata(&mut conn, paper_id, ws_title_update, None).unwrap_err();
    assert!(matches!(err, MetadataError::Validation(ref msg) if msg == "Title cannot be empty"));

    // 2. Invalid year rejected (< 1000)
    let year_too_small = UpdatePaperMetadata {
        title: "Valid Title".to_string(),
        year: Some(999),
        ..Default::default()
    };
    let err = update_metadata(&mut conn, paper_id, year_too_small, None).unwrap_err();
    assert!(matches!(err, MetadataError::Validation(ref msg) if msg == "Invalid year"));

    // Invalid year rejected (> 3000)
    let year_too_large = UpdatePaperMetadata {
        title: "Valid Title".to_string(),
        year: Some(3001),
        ..Default::default()
    };
    let err = update_metadata(&mut conn, paper_id, year_too_large, None).unwrap_err();
    assert!(matches!(err, MetadataError::Validation(ref msg) if msg == "Invalid year"));

    // Valid year boundaries (1000 and 3000) and None accepted
    for valid_year in [Some(1000), Some(2024), Some(3000), None] {
        let valid_update = UpdatePaperMetadata {
            title: "Cryptographic Protocols".to_string(),
            year: valid_year,
            ..Default::default()
        };
        let res = update_metadata(&mut conn, paper_id, valid_update, None);
        assert!(res.is_ok(), "Year {:?} should be valid", valid_year);
    }

    // Nonexistent paper ID returns NotFound
    let missing_id = Uuid::now_v7();
    let missing_update = UpdatePaperMetadata {
        title: "Nonexistent".to_string(),
        ..Default::default()
    };
    let err = update_metadata(&mut conn, missing_id, missing_update, None).unwrap_err();
    assert!(matches!(err, MetadataError::NotFound(id) if id == missing_id));

    // 3. Verify unchanged fields are preserved
    let after_validation_paper = PaperRepo::get_by_id(&conn, paper_id)
        .expect("get paper")
        .expect("paper exists");
    assert_eq!(
        after_validation_paper.file_path,
        format!("/papers/{paper_id}.pdf")
    );
    assert_eq!(
        after_validation_paper.content_hash,
        format!("hash_{paper_id}")
    );
    assert_eq!(
        after_validation_paper.annotated_pdf_path.as_deref(),
        Some(format!("/papers/{paper_id}_annotated.pdf").as_str())
    );
    assert_eq!(
        after_validation_paper.toc_embedded_at.as_deref(),
        Some("2020-01-01T00:00:00Z")
    );
    assert_eq!(after_validation_paper.created_at, "2020-01-01T00:00:00Z");

    // --- TUI Edit Modal Dispatching and State Updates ---
    let search_index = Arc::new(SearchIndex::create_in_ram().expect("ram index"));
    let mut app = App::new()
        .with_db_conn(conn)
        .with_search_index(Arc::clone(&search_index));

    app.add_collection(
        CollectionItem::new(None, "All Papers", 1),
        vec![after_validation_paper.clone()],
    );

    // Initial state: not editing
    assert!(!app.is_editing_metadata);
    assert_eq!(app.editing_field_index, 0);

    // 'e' key triggers Action::EditMetadata
    let e_key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
    assert_eq!(map_key_event(e_key), Some(Action::EditMetadata));
    assert_eq!(
        map_key_event_for_app(e_key, &app),
        Some(Action::EditMetadata)
    );

    // Dispatch Action::EditMetadata
    app.dispatch(Action::EditMetadata);
    assert!(app.is_editing_metadata);
    assert_eq!(app.editing_field_index, 0); // Focus on Title
    assert_eq!(app.edit_buffers[0], "Cryptographic Protocols");
    assert_eq!(app.edit_buffers[1], "");
    assert_eq!(app.edit_buffers[2], ""); // Year was set to None in last iteration
    assert_eq!(app.edit_buffers[3], "");
    assert_eq!(app.edit_buffers[4], "");
    assert_eq!(app.edit_buffers[5], "");

    // Key event mapping inside edit modal
    // Tab -> Next field
    let tab_key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(tab_key, &app),
        Some(Action::EditMetadataNextField)
    );
    app.dispatch(Action::EditMetadataNextField);
    assert_eq!(app.editing_field_index, 1); // Authors

    // Down -> Next field
    let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(down_key, &app),
        Some(Action::EditMetadataNextField)
    );
    app.dispatch(Action::EditMetadataNextField);
    assert_eq!(app.editing_field_index, 2); // Year

    // Shift+Tab -> Prev field
    let shift_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT);
    assert_eq!(
        map_key_event_for_app(shift_tab, &app),
        Some(Action::EditMetadataPrevField)
    );
    app.dispatch(Action::EditMetadataPrevField);
    assert_eq!(app.editing_field_index, 1); // Authors

    // Up -> Prev field
    let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(up_key, &app),
        Some(Action::EditMetadataPrevField)
    );
    app.dispatch(Action::EditMetadataPrevField);
    assert_eq!(app.editing_field_index, 0); // Title

    // Test buffer text editing (append and backspace)
    let char_k = KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(char_k, &app),
        Some(Action::EditMetadataInput('!'))
    );
    app.dispatch(Action::EditMetadataInput('!'));
    assert_eq!(app.edit_buffers[0], "Cryptographic Protocols!");

    let bs_key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(bs_key, &app),
        Some(Action::EditMetadataBackspace)
    );
    app.dispatch(Action::EditMetadataBackspace);
    assert_eq!(app.edit_buffers[0], "Cryptographic Protocols");

    // Test Esc cancellation
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(esc_key, &app),
        Some(Action::EditMetadataCancel)
    );
    app.dispatch(Action::EditMetadataCancel);
    assert!(!app.is_editing_metadata);
    assert_eq!(
        app.papers[0].title.as_deref(),
        Some("Cryptographic Protocols")
    );

    // Re-open modal and test TUI validation rejection (empty title)
    app.dispatch(Action::EditMetadata);
    assert!(app.is_editing_metadata);
    app.edit_buffers[0] = "".to_string();

    let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        map_key_event_for_app(enter_key, &app),
        Some(Action::EditMetadataSave)
    );
    app.dispatch(Action::EditMetadataSave);
    assert!(
        app.is_editing_metadata,
        "Modal must stay open on empty title"
    );
    assert_eq!(app.status_message.as_deref(), Some("Title cannot be empty"));

    // Test TUI validation rejection (invalid year)
    app.edit_buffers[0] = "Valid Title".to_string();
    app.edit_buffers[2] = "4500".to_string();
    app.dispatch(Action::EditMetadataSave);
    assert!(
        app.is_editing_metadata,
        "Modal must stay open on invalid year"
    );
    assert_eq!(app.status_message.as_deref(), Some("Invalid year"));

    // Test successful save via TUI
    app.edit_buffers[0] = "Zero Knowledge Proofs".to_string();
    app.edit_buffers[1] = "Goldwasser, Micali, Rackoff".to_string();
    app.edit_buffers[2] = "1985".to_string();
    app.edit_buffers[3] = "SIAM Journal on Computing".to_string();
    app.edit_buffers[4] = "10.1137/0218012".to_string();
    app.edit_buffers[5] = "Interactive proof systems with zero knowledge.".to_string();

    // Ctrl+s saves
    let ctrl_s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
    assert_eq!(
        map_key_event_for_app(ctrl_s, &app),
        Some(Action::EditMetadataSave)
    );
    app.dispatch(Action::EditMetadataSave);

    assert!(!app.is_editing_metadata, "Modal must close on valid save");
    assert_eq!(
        app.papers[0].title.as_deref(),
        Some("Zero Knowledge Proofs")
    );
    assert_eq!(
        app.papers[0].authors.as_deref(),
        Some("Goldwasser, Micali, Rackoff")
    );
    assert_eq!(app.papers[0].year, Some(1985));
    assert_eq!(
        app.papers[0].journal.as_deref(),
        Some("SIAM Journal on Computing")
    );
    assert_eq!(app.papers[0].doi.as_deref(), Some("10.1137/0218012"));
    assert_eq!(
        app.papers[0].abstract_text.as_deref(),
        Some("Interactive proof systems with zero knowledge.")
    );
    assert!(app
        .status_message
        .as_deref()
        .unwrap_or("")
        .contains("Updated metadata for Zero Knowledge Proofs"));

    // Verify DB was updated
    let db_conn = app.db_conn().expect("conn");
    let persisted = PaperRepo::get_by_id(db_conn, paper_id)
        .expect("query paper")
        .expect("paper exists");
    assert_eq!(persisted.title.as_deref(), Some("Zero Knowledge Proofs"));
    assert_eq!(persisted.year, Some(1985));

    // Verify Tantivy index was updated via TUI save
    let zk_search = search_index.search("Knowledge", 10).expect("search");
    assert_eq!(zk_search.len(), 1);
    assert_eq!(zk_search[0].id, paper_id);

    // Verify UI rendering in both modal and normal state
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).expect("create terminal");

    // Normal rendering
    terminal.draw(|f| render(&app, f)).expect("render normal");

    // Modal rendering
    app.dispatch(Action::EditMetadata);
    assert!(app.is_editing_metadata);
    terminal.draw(|f| render(&app, f)).expect("render modal");

    // Compact modal rendering on small terminal
    let compact_backend = TestBackend::new(80, 16);
    let mut compact_terminal = Terminal::new(compact_backend).expect("create compact terminal");
    compact_terminal
        .draw(|f| render(&app, f))
        .expect("render compact modal");
}
