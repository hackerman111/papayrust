use super::*;
use std::sync::Arc;
use uuid::Uuid;

use papyrus_core::search::SearchIndex;
use papyrus_core::Action;

fn dummy_paper(title: &str, authors: &str, year: i64) -> Paper {
    Paper {
        id: Uuid::now_v7(),
        file_path: format!("/path/{title}.pdf"),
        content_hash: format!("hash_{title}"),
        title: Some(title.to_string()),
        authors: Some(authors.to_string()),
        year: Some(year),
        journal: None,
        doi: None,
        abstract_text: Some(format!("Abstract of {title}")),
        text_path: None,
        annotated_pdf_path: None,
        toc_embedded_at: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    }
}

fn dummy_toc(paper_id: Uuid, title: &str, page_number: u32) -> TocEntry {
    TocEntry {
        id: Uuid::now_v7(),
        paper_id,
        parent_id: None,
        title: title.to_string(),
        page_number,
        order_index: 0,
        source: papyrus_core::db::TocSource::Manual,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    }
}

#[test]
fn test_initial_focus_is_papers() {
    let app = App::new();
    assert_eq!(app.active_panel, ActivePanel::Collections);

    let conn = papyrus_core::db::open_in_memory().unwrap();
    let app_db = App::from_db(&conn).unwrap();
    assert_eq!(app_db.active_panel, ActivePanel::Papers);
}

#[test]
fn test_app_search_state_and_filter() {
    let mut app = App::new();
    let p1 = dummy_paper("Attention Is All You Need", "Vaswani", 2017);
    let p2 = dummy_paper("BERT: Pre-training", "Devlin", 2018);
    let p3 = dummy_paper("Deep Residual Learning", "He", 2015);

    app.add_collection(
        CollectionItem::new(None, "All Papers", 3),
        vec![p1.clone(), p2.clone(), p3.clone()],
    );

    assert_eq!(app.papers.len(), 3);
    assert!(!app.is_searching);
    assert!(app.search_query.is_empty());

    // Toggle search mode
    app.dispatch(Action::Search);
    assert!(app.is_searching);

    // Type 'b' -> 'e' -> 'r' -> 't'
    for c in ['b', 'e', 'r', 't'] {
        app.dispatch(Action::SearchInput(c));
    }
    assert_eq!(app.search_query, "bert");
    assert_eq!(app.papers.len(), 1);
    assert_eq!(app.papers[0].title.as_deref(), Some("BERT: Pre-training"));

    // Backspace removes 't' -> query "ber", still matches BERT
    app.dispatch(Action::SearchBackspace);
    assert_eq!(app.search_query, "ber");
    assert_eq!(app.papers.len(), 1);

    // Confirm search keeps filtered list but exits typing mode
    app.dispatch(Action::SearchConfirm);
    assert!(!app.is_searching);
    assert_eq!(app.papers.len(), 1);

    // Cancel / Esc restores full list and clears query
    app.dispatch(Action::SearchCancel);
    assert!(!app.is_searching);
    assert!(app.search_query.is_empty());
    assert_eq!(app.papers.len(), 3);
}

#[test]
fn test_app_search_with_search_index() {
    let mut app = App::new();
    let search_index = Arc::new(SearchIndex::create_in_ram().expect("create index"));

    let p1 = dummy_paper("Quantum Computing", "Nielsen", 2010);
    let p2 = dummy_paper("Deep Learning", "Goodfellow", 2016);

    search_index
        .index_paper(&p1, Some("superposition and entanglement qubit"))
        .expect("index p1");
    search_index
        .index_paper(&p2, Some("backpropagation neural network gradient"))
        .expect("index p2");

    app.set_search_index(search_index);
    app.add_collection(
        CollectionItem::new(None, "All Papers", 2),
        vec![p1.clone(), p2.clone()],
    );

    // Search body keyword "qubit"
    app.set_search_query("qubit");
    assert_eq!(app.papers.len(), 1);
    assert_eq!(app.papers[0].id, p1.id);

    // Search body keyword "gradient"
    app.set_search_query("gradient");
    assert_eq!(app.papers.len(), 1);
    assert_eq!(app.papers[0].id, p2.id);

    // Clear search restores all
    app.clear_status();
    app.cancel_search();
    assert_eq!(app.papers.len(), 2);
}

#[test]
fn test_app_metadata_edit_modal_lifecycle() {
    let mut app = App::new();
    let p = dummy_paper("Attention Is All You Need", "Vaswani", 2017);
    app.add_collection(CollectionItem::new(None, "All Papers", 1), vec![p]);

    assert!(!app.is_editing_metadata);
    assert_eq!(app.editing_field_index, 0);

    // Dispatch EditMetadata
    app.dispatch(Action::EditMetadata);
    assert!(app.is_editing_metadata);
    assert_eq!(app.editing_field_index, 0);
    assert_eq!(app.edit_buffers[0], "Attention Is All You Need");
    assert_eq!(app.edit_buffers[1], "Vaswani");
    assert_eq!(app.edit_buffers[2], "2017");
    assert_eq!(app.edit_buffers[3], "");
    assert_eq!(app.edit_buffers[4], "");
    assert_eq!(app.edit_buffers[5], "Abstract of Attention Is All You Need");

    // Next field navigation (0 -> 1 -> 2 -> 3 -> 4 -> 5 -> 0)
    for i in 1..=5 {
        app.dispatch(Action::EditMetadataNextField);
        assert_eq!(app.editing_field_index, i);
    }
    app.dispatch(Action::EditMetadataNextField);
    assert_eq!(app.editing_field_index, 0);

    // Prev field navigation (0 -> 5 -> 4 -> ...)
    app.dispatch(Action::EditMetadataPrevField);
    assert_eq!(app.editing_field_index, 5);
    app.dispatch(Action::EditMetadataPrevField);
    assert_eq!(app.editing_field_index, 4);

    // Switch back to Title (0)
    app.editing_field_index = 0;
    app.dispatch(Action::EditMetadataInput('!'));
    assert_eq!(app.edit_buffers[0], "Attention Is All You Need!");
    app.dispatch(Action::EditMetadataBackspace);
    assert_eq!(app.edit_buffers[0], "Attention Is All You Need");

    // Cancel editing restores modal state without modifying paper
    app.dispatch(Action::EditMetadataCancel);
    assert!(!app.is_editing_metadata);
    assert_eq!(app.editing_field_index, 0);
    assert!(app.edit_buffers[0].is_empty());
    assert_eq!(
        app.papers[0].title.as_deref(),
        Some("Attention Is All You Need")
    );
}

#[test]
fn test_app_metadata_save_success_and_validation() {
    let mut app = App::new();
    let p = dummy_paper("Original Title", "Original Author", 2020);
    app.add_collection(CollectionItem::new(None, "All Papers", 1), vec![p]);

    // Open modal
    app.dispatch(Action::EditMetadata);
    assert!(app.is_editing_metadata);

    // 1. Validation error: empty title
    app.edit_buffers[0] = "   ".to_string();
    app.dispatch(Action::EditMetadataSave);
    assert!(app.is_editing_metadata, "Modal must remain open on error");
    assert_eq!(app.status_message.as_deref(), Some("Title cannot be empty"));

    // 2. Validation error: invalid year
    app.edit_buffers[0] = "Valid Title".to_string();
    app.edit_buffers[2] = "9999".to_string();
    app.dispatch(Action::EditMetadataSave);
    assert!(app.is_editing_metadata, "Modal must remain open on error");
    assert_eq!(app.status_message.as_deref(), Some("Invalid year"));

    app.edit_buffers[2] = "not_a_number".to_string();
    app.dispatch(Action::EditMetadataSave);
    assert!(app.is_editing_metadata, "Modal must remain open on error");
    assert_eq!(app.status_message.as_deref(), Some("Invalid year"));

    // 3. Valid save
    app.edit_buffers[0] = "Attention 2.0".to_string();
    app.edit_buffers[1] = "Vaswani et al.".to_string();
    app.edit_buffers[2] = "2024".to_string();
    app.edit_buffers[3] = "NeurIPS".to_string();
    app.dispatch(Action::EditMetadataSave);

    assert!(!app.is_editing_metadata, "Modal must close on success");
    assert_eq!(app.papers[0].title.as_deref(), Some("Attention 2.0"));
    assert_eq!(app.papers[0].authors.as_deref(), Some("Vaswani et al."));
    assert_eq!(app.papers[0].year, Some(2024));
    assert_eq!(app.papers[0].journal.as_deref(), Some("NeurIPS"));
    assert!(app
        .status_message
        .as_deref()
        .unwrap_or("")
        .contains("Updated metadata for Attention 2.0"));
}

#[test]
fn test_app_toc_modal_lifecycle() {
    let mut app = App::new();
    let p = dummy_paper("Attention", "Vaswani", 2017);
    app.add_collection(CollectionItem::new(None, "All Papers", 1), vec![p]);

    assert!(!app.is_editing_toc());

    // Start Add TOC entry
    app.dispatch(Action::TocAddEntry { parent_id: None });
    assert!(app.is_editing_toc());

    let state = app.toc_edit_state.as_ref().unwrap();
    assert_eq!(state.entry_id, None);
    assert_eq!(state.active_field, 0);

    // Type title
    app.dispatch(Action::TocEditModalInput('I'));
    app.dispatch(Action::TocEditModalInput('n'));
    app.dispatch(Action::TocEditModalInput('t'));
    app.dispatch(Action::TocEditModalInput('r'));
    app.dispatch(Action::TocEditModalInput('o'));

    // Switch to page field
    app.dispatch(Action::TocEditModalNextField);
    let state = app.toc_edit_state.as_ref().unwrap();
    assert_eq!(state.active_field, 1);

    // Change page to 5
    app.dispatch(Action::TocEditModalBackspace);
    app.dispatch(Action::TocEditModalInput('5'));

    // Save
    app.dispatch(Action::TocEditModalSave);
    assert!(!app.is_editing_toc());
    assert_eq!(app.toc_preview.len(), 1);
    assert_eq!(app.toc_preview[0].title, "Intro");
    assert_eq!(app.toc_preview[0].page_number, 5);

    // Start editing existing TOC entry
    let id = app.toc_preview[0].id;
    app.dispatch(Action::TocEditEntry { id });
    assert!(app.is_editing_toc());
    let state = app.toc_edit_state.as_ref().unwrap();
    assert_eq!(state.entry_id, Some(id));
    assert_eq!(state.title_buffer, "Intro");
    assert_eq!(state.page_buffer, "5");

    // Cancel editing
    app.dispatch(Action::TocEditModalCancel);
    assert!(!app.is_editing_toc());
}

#[test]
fn test_app_add_paper_modal_lifecycle() {
    let mut app = App::new();
    assert!(!app.is_adding_paper);
    assert!(app.add_paper_path_buffer.is_empty());

    // Open add paper modal
    app.dispatch(Action::AddPaperModalOpen);
    assert!(app.is_adding_paper);

    // Type path
    for c in "paper.pdf".chars() {
        app.dispatch(Action::AddPaperModalInput(c));
    }
    assert_eq!(app.add_paper_path_buffer, "paper.pdf");

    // Backspace
    app.dispatch(Action::AddPaperModalBackspace);
    assert_eq!(app.add_paper_path_buffer, "paper.pd");

    // Cancel
    app.dispatch(Action::AddPaperModalCancel);
    assert!(!app.is_adding_paper);
    assert!(app.add_paper_path_buffer.is_empty());
}

#[test]
fn test_app_create_collection_modal_lifecycle() {
    let mut app = App::new();
    assert!(!app.is_creating_collection);
    assert!(app.collection_name_buffer.is_empty());

    // Open modal
    app.dispatch(Action::CreateCollectionModalOpen);
    assert!(app.is_creating_collection);

    // Type collection name
    for c in "Machine Learning".chars() {
        app.dispatch(Action::CreateCollectionModalInput(c));
    }
    assert_eq!(app.collection_name_buffer, "Machine Learning");

    // Backspace
    app.dispatch(Action::CreateCollectionModalBackspace);
    assert_eq!(app.collection_name_buffer, "Machine Learnin");

    // Cancel
    app.dispatch(Action::CreateCollectionModalCancel);
    assert!(!app.is_creating_collection);
    assert!(app.collection_name_buffer.is_empty());
}

#[test]
fn test_app_create_collection_with_db() {
    let conn = papyrus_core::db::open_in_memory().unwrap();
    let mut app = App::from_db_conn(conn).unwrap();

    let initial_count = app.collections.len();

    // Open modal and type new collection name
    app.dispatch(Action::CreateCollectionModalOpen);
    for c in "Deep Learning".chars() {
        app.dispatch(Action::CreateCollectionModalInput(c));
    }
    app.dispatch(Action::CreateCollectionModalConfirm);

    assert!(!app.is_creating_collection);
    assert_eq!(app.collections.len(), initial_count + 1);
    assert!(app.collections.iter().any(|c| c.name == "Deep Learning"));
    assert_eq!(
        app.collections[app.selected_collection].name,
        "Deep Learning"
    );
    assert_eq!(
        app.status_message.as_deref(),
        Some("Created collection 'Deep Learning'")
    );
    assert!(app.needs_clear);
}

#[test]
fn test_app_folder_recursive_pdf_import() {
    use lopdf::{dictionary, Document, Object, Stream};
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let root_path = dir.path();
    let sub_path = root_path.join("subdir");
    std::fs::create_dir_all(&sub_path).unwrap();

    let make_pdf = |p: &std::path::Path, text: &str| {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let content_id = doc.add_object(Stream::new(
            dictionary!(),
            format!("BT /F1 12 Tf 100 700 Td ({text}) Tj ET").into_bytes(),
        ));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Resources" => dictionary! {
                "Font" => dictionary! {
                    "F1" => font_id,
                },
            },
        });
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc.save(p).unwrap();
    };

    make_pdf(&root_path.join("paper1.pdf"), "Paper 1 content");
    make_pdf(&sub_path.join("paper2.pdf"), "Paper 2 content");
    // Non-PDF file to verify filtering
    std::fs::write(root_path.join("notes.txt"), "some notes").unwrap();

    let conn = papyrus_core::db::open_in_memory().unwrap();
    let mut app = App::from_db_conn(conn).unwrap();

    app.dispatch(Action::AddPaperModalOpen);
    for c in root_path.to_str().unwrap().chars() {
        app.dispatch(Action::AddPaperModalInput(c));
    }
    app.dispatch(Action::AddPaperModalConfirm);

    assert!(!app.is_adding_paper);
    assert_eq!(app.papers.len(), 2);
    assert!(app
        .status_message
        .as_ref()
        .unwrap()
        .starts_with("Imported 2 papers from"));
    assert!(app.needs_clear);

    // Re-importing same folder should report 2 duplicates skipped
    app.dispatch(Action::AddPaperModalOpen);
    for c in root_path.to_str().unwrap().chars() {
        app.dispatch(Action::AddPaperModalInput(c));
    }
    app.dispatch(Action::AddPaperModalConfirm);

    assert_eq!(app.papers.len(), 2);
    assert!(app
        .status_message
        .as_ref()
        .unwrap()
        .contains("2 duplicates skipped"));
}

#[test]
fn test_key_events_for_collections_panel() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    let action = crate::event::map_key_event_with_context(key, ActivePanel::Collections, None);
    assert_eq!(action, Some(Action::CreateCollectionModalOpen));

    let key_paper = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    let action_paper =
        crate::event::map_key_event_with_context(key_paper, ActivePanel::Papers, None);
    assert_eq!(action_paper, Some(Action::AddPaperModalOpen));
}

#[test]
fn test_fullscreen_toc_flow() {
    let p = dummy_paper("Attention Is All You Need", "Vaswani", 2017);
    let toc1 = dummy_toc(p.id, "1. Introduction", 1);
    let toc2 = dummy_toc(p.id, "2. Background", 3);
    let toc3 = dummy_toc(p.id, "3. Model Architecture", 5);

    let mut tocs_map = std::collections::HashMap::new();
    tocs_map.insert(p.id, vec![toc1, toc2, toc3]);

    let mut papers_map = std::collections::HashMap::new();
    papers_map.insert(None, vec![p]);

    let collections = vec![CollectionItem::new(None, "All Papers", 1)];
    let mut app = App::with_data(collections, papers_map, tocs_map);

    assert!(!app.is_viewing_fullscreen_toc);

    // Open fullscreen TOC
    app.dispatch(Action::OpenFullscreenToc);
    assert!(app.is_viewing_fullscreen_toc);
    assert_eq!(app.selected_toc, 0);

    // Navigate down
    app.dispatch(Action::MoveDown);
    assert_eq!(app.selected_toc, 1);

    // Close fullscreen TOC
    app.dispatch(Action::CloseFullscreenToc);
    assert!(!app.is_viewing_fullscreen_toc);
}

#[test]
fn test_paper_tags_lifecycle_and_db() {
    let conn = papyrus_core::db::open_in_memory().unwrap();
    let mut app = App::from_db_conn(conn).unwrap();

    let p = dummy_paper("Deep Learning Book", "Goodfellow", 2016);
    if let Some(ref conn) = app.db_conn {
        papyrus_core::db::PaperRepo::insert(conn, &p).unwrap();
    }
    app.reload_from_db().unwrap();
    assert_eq!(app.papers.len(), 1);

    // Open tags modal
    app.dispatch(Action::EditTagsModalOpen);
    assert!(app.is_editing_tags);

    // Type tags: "ai, math, deep-learning"
    for c in "ai, math, deep-learning".chars() {
        app.dispatch(Action::EditTagsModalInput(c));
    }
    assert_eq!(app.tags_input_buffer, "ai, math, deep-learning");

    // Confirm
    app.dispatch(Action::EditTagsModalConfirm);
    assert!(!app.is_editing_tags);

    // Verify stored in memory
    let tags = app.tags_by_paper.get(&p.id).unwrap();
    assert_eq!(
        tags,
        &vec![
            "ai".to_string(),
            "math".to_string(),
            "deep-learning".to_string()
        ]
    );

    // Verify stored in SQLite
    if let Some(ref conn) = app.db_conn {
        let db_tags = papyrus_core::db::TagRepo::get_tags_for_paper(conn, p.id).unwrap();
        assert_eq!(db_tags, vec!["ai", "deep-learning", "math"]);
    }
}

#[test]
fn test_search_by_title_and_tags_and_collection_cycling() {
    let p1 = dummy_paper("Attention Is All You Need", "Vaswani", 2017);
    let p2 = dummy_paper("Deep Residual Learning", "He", 2016);

    let col1_id = uuid::Uuid::now_v7();
    let col2_id = uuid::Uuid::now_v7();

    let collections = vec![
        CollectionItem::new(None, "All Papers", 2),
        CollectionItem::new(Some(col1_id), "NLP", 1),
        CollectionItem::new(Some(col2_id), "Vision", 1),
    ];

    let mut papers_map = std::collections::HashMap::new();
    papers_map.insert(None, vec![p1.clone(), p2.clone()]);
    papers_map.insert(Some(col1_id), vec![p1.clone()]);
    papers_map.insert(Some(col2_id), vec![p2.clone()]);

    let mut app = App::with_data(collections, papers_map, std::collections::HashMap::new());

    // Set tags
    app.tags_by_paper
        .insert(p1.id, vec!["transformer".to_string(), "nlp".to_string()]);
    app.tags_by_paper
        .insert(p2.id, vec!["resnet".to_string(), "cnn".to_string()]);

    // 1. Search by title
    app.start_search();
    app.set_search_query("Residual");
    assert_eq!(app.papers.len(), 1);
    assert_eq!(app.papers[0].id, p2.id);

    // 2. Search by tag substring
    app.set_search_query("transformer");
    assert_eq!(app.papers.len(), 1);
    assert_eq!(app.papers[0].id, p1.id);

    // 3. Search with #tag prefix
    app.set_search_query("#resnet");
    assert_eq!(app.papers.len(), 1);
    assert_eq!(app.papers[0].id, p2.id);

    // 4. Cycle collection via Tab
    app.set_search_query(""); // show all in current collection
    assert_eq!(app.papers.len(), 2);

    app.dispatch(Action::SearchCycleCollection);
    assert_eq!(app.search_collection_index, 1); // "NLP"
    assert_eq!(app.papers.len(), 1);
    assert_eq!(app.papers[0].id, p1.id);

    app.dispatch(Action::SearchCycleCollection);
    assert_eq!(app.search_collection_index, 2); // "Vision"
    assert_eq!(app.papers.len(), 1);
    assert_eq!(app.papers[0].id, p2.id);
}

#[test]
fn test_deletion_of_collection_and_paper() {
    let conn = papyrus_core::db::open_in_memory().unwrap();
    let mut app = App::from_db_conn(conn).unwrap();

    let col = papyrus_core::db::Collection {
        id: uuid::Uuid::now_v7(),
        name: "Temporary".to_string(),
        parent_id: None,
    };
    if let Some(ref conn) = app.db_conn {
        papyrus_core::db::CollectionRepo::insert(conn, &col).unwrap();
    }
    app.reload_from_db().unwrap();

    // Select custom collection (index 1)
    app.selected_collection = 1;
    app.active_panel = ActivePanel::Collections;

    // Trigger delete confirm
    app.dispatch(Action::DeleteConfirmOpen);
    assert!(app.is_confirming_delete);
    assert!(app.delete_target_description.contains("Temporary"));

    // Execute delete
    app.dispatch(Action::DeleteConfirmExecute);
    assert!(!app.is_confirming_delete);

    // Verify collection is gone
    assert!(app.collections.iter().all(|c| c.name != "Temporary"));

    // Now test paper deletion
    let p = dummy_paper("Paper To Delete", "Author", 2020);
    if let Some(ref conn) = app.db_conn {
        papyrus_core::db::PaperRepo::insert(conn, &p).unwrap();
    }
    app.reload_from_db().unwrap();
    assert_eq!(app.papers.len(), 1);

    app.active_panel = ActivePanel::Papers;
    app.selected_paper = 0;

    app.dispatch(Action::DeleteConfirmOpen);
    assert!(app.is_confirming_delete);
    assert!(app.delete_target_description.contains("Paper To Delete"));

    app.dispatch(Action::DeleteConfirmExecute);
    assert!(!app.is_confirming_delete);
    assert_eq!(app.papers.len(), 0);
}

#[test]
fn test_help_modal_toggle() {
    let mut app = App::new();
    assert!(!app.is_showing_help);

    app.dispatch(Action::HelpModalToggle);
    assert!(app.is_showing_help);

    app.dispatch(Action::HelpModalToggle);
    assert!(!app.is_showing_help);
}
