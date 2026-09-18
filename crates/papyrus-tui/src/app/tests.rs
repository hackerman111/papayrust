use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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

#[test]
fn test_subcollection_creation_and_tree_hierarchy() {
    let conn = papyrus_core::db::open_in_memory().unwrap();
    let mut app = App::from_db_conn(conn).unwrap();

    // 1. Create root collection "Computer Science"
    app.dispatch(Action::CreateCollectionModalOpen);
    for c in "Computer Science".chars() {
        app.dispatch(Action::CreateCollectionModalInput(c));
    }
    app.dispatch(Action::CreateCollectionModalConfirm);

    let cs_idx = app
        .collections
        .iter()
        .position(|c| c.name == "Computer Science")
        .unwrap();
    assert_eq!(app.collections[cs_idx].depth, 0);
    let cs_id = app.collections[cs_idx].id.unwrap();

    // 2. Select CS and create subcollection "AI"
    app.selected_collection = cs_idx;
    app.dispatch(Action::CreateSubcollectionModalOpen);
    assert_eq!(app.create_collection_parent_id, Some(cs_id));
    assert!(app.is_creating_collection);

    for c in "Artificial Intelligence".chars() {
        app.dispatch(Action::CreateCollectionModalInput(c));
    }
    app.dispatch(Action::CreateCollectionModalConfirm);

    let ai_idx = app
        .collections
        .iter()
        .position(|c| c.name == "Artificial Intelligence")
        .unwrap();
    assert_eq!(app.collections[ai_idx].depth, 1);
    assert_eq!(app.collections[ai_idx].parent_id, Some(cs_id));

    // Verify ordering: CS comes right before its child AI
    assert_eq!(ai_idx, cs_idx + 1);
}

#[test]
fn test_collection_rename_lifecycle_and_persistence() {
    let conn = papyrus_core::db::open_in_memory().unwrap();
    let mut app = App::from_db_conn(conn).unwrap();

    // Cannot rename "All Papers"
    app.selected_collection = 0;
    app.dispatch(Action::RenameCollectionModalOpen);
    assert!(!app.is_renaming_collection);
    assert_eq!(
        app.status_message.as_deref(),
        Some("Cannot rename 'All Papers' virtual collection")
    );

    // Create a collection
    app.dispatch(Action::CreateCollectionModalOpen);
    for c in "Drafts".chars() {
        app.dispatch(Action::CreateCollectionModalInput(c));
    }
    app.dispatch(Action::CreateCollectionModalConfirm);

    let drafts_idx = app
        .collections
        .iter()
        .position(|c| c.name == "Drafts")
        .unwrap();
    app.selected_collection = drafts_idx;

    // Open rename modal
    app.dispatch(Action::RenameCollectionModalOpen);
    assert!(app.is_renaming_collection);
    assert_eq!(app.rename_collection_buffer, "Drafts");

    // Change to "Archived Papers"
    app.rename_collection_buffer.clear();
    for c in "Archived Papers".chars() {
        app.dispatch(Action::RenameCollectionModalInput(c));
    }
    app.dispatch(Action::RenameCollectionModalConfirm);

    assert!(!app.is_renaming_collection);
    assert!(app.collections.iter().any(|c| c.name == "Archived Papers"));
    assert!(!app.collections.iter().any(|c| c.name == "Drafts"));

    // Verify in SQLite
    if let Some(ref conn) = app.db_conn {
        let cols = papyrus_core::db::CollectionRepo::list(conn).unwrap();
        assert!(cols.iter().any(|c| c.name == "Archived Papers"));
    }
}

#[test]
fn test_collection_export_to_zip() {
    use tempfile::tempdir;

    let conn = papyrus_core::db::open_in_memory().unwrap();
    let mut app = App::from_db_conn(conn).unwrap();

    let p = dummy_paper("Export Paper", "Researcher", 2023);
    // Create a dummy file so export doesn't fail on missing file
    let dir = tempdir().unwrap();
    let dummy_pdf = dir.path().join("export_dummy.pdf");
    std::fs::write(&dummy_pdf, b"%PDF-1.4 dummy content").unwrap();

    let mut p_with_real_path = p.clone();
    p_with_real_path.file_path = dummy_pdf.to_str().unwrap().to_string();

    if let Some(ref conn) = app.db_conn {
        papyrus_core::db::PaperRepo::insert(conn, &p_with_real_path).unwrap();
    }
    app.reload_from_db().unwrap();

    let zip_dest = dir.path().join("my_export.zip");

    // Open export modal
    app.dispatch(Action::ExportCollectionModalOpen);
    assert!(app.is_exporting_collection);

    for c in zip_dest.to_str().unwrap().chars() {
        app.dispatch(Action::ExportCollectionModalInput(c));
    }
    app.dispatch(Action::ExportCollectionModalConfirm);

    assert!(!app.is_exporting_collection);
    assert!(zip_dest.exists());
    assert!(app
        .status_message
        .as_deref()
        .unwrap_or("")
        .starts_with("Exported 1 papers to"));
}

#[test]
fn test_import_metadata_from_json_lifecycle() {
    use tempfile::tempdir;

    let conn = papyrus_core::db::open_in_memory().unwrap();
    let mut app = App::from_db_conn(conn).unwrap();

    let p = dummy_paper("Unprocessed Paper", "Unknown", 2019);
    if let Some(ref conn) = app.db_conn {
        papyrus_core::db::PaperRepo::insert(conn, &p).unwrap();
    }
    app.reload_from_db().unwrap();
    assert_eq!(app.papers.len(), 1);

    let dir = tempdir().unwrap();
    let json_file = dir.path().join("metadata.json");
    std::fs::write(
        &json_file,
        r#"{
            "title": "Quantum Supremacy Using a Programmable Superconducting Processor",
            "authors": "Arute et al.",
            "year": 2019,
            "journal": "Nature",
            "tags": ["quantum", "google"]
        }"#,
    )
    .unwrap();

    // Select paper and import metadata
    app.selected_paper = 0;
    app.dispatch(Action::ImportMetadataModalOpen);
    assert!(app.is_importing_metadata);

    for c in json_file.to_str().unwrap().chars() {
        app.dispatch(Action::ImportMetadataModalInput(c));
    }
    app.dispatch(Action::ImportMetadataModalConfirm);

    assert!(!app.is_importing_metadata);
    assert_eq!(
        app.papers[0].title.as_deref(),
        Some("Quantum Supremacy Using a Programmable Superconducting Processor")
    );
    assert_eq!(app.papers[0].authors.as_deref(), Some("Arute et al."));
    assert_eq!(app.papers[0].journal.as_deref(), Some("Nature"));

    let tags = app.tags_by_paper.get(&app.papers[0].id).unwrap();
    assert_eq!(tags, &vec!["google".to_string(), "quantum".to_string()]);
}

#[test]
fn test_path_autocomplete_in_modals() {
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let target_file = dir.path().join("unique_target_paper.pdf");
    std::fs::write(&target_file, b"test").unwrap();

    let mut app = App::new();

    // 1. In AddPaperModal
    app.dispatch(Action::AddPaperModalOpen);
    let prefix = format!("{}/unique_tar", dir.path().display());
    app.add_paper_path_buffer = prefix;
    app.dispatch(Action::AddPaperModalAutocomplete);
    assert_eq!(
        app.add_paper_path_buffer,
        format!("{}/unique_target_paper.pdf", dir.path().display())
    );

    // 2. In ExportCollectionModal
    app.dispatch(Action::ExportCollectionModalOpen);
    let prefix_export = format!("{}/unique_tar", dir.path().display());
    app.export_path_buffer = prefix_export;
    app.dispatch(Action::ExportCollectionModalAutocomplete);
    assert_eq!(
        app.export_path_buffer,
        format!("{}/unique_target_paper.pdf", dir.path().display())
    );

    // 3. In ImportMetadataModal
    let json_file = dir.path().join("unique_meta.json");
    std::fs::write(&json_file, b"{}").unwrap();

    app.dispatch(Action::ImportMetadataModalOpen);
    let prefix_meta = format!("{}/unique_met", dir.path().display());
    app.import_metadata_buffer = prefix_meta;
    app.dispatch(Action::ImportMetadataModalAutocomplete);
    assert_eq!(
        app.import_metadata_buffer,
        format!("{}/unique_meta.json", dir.path().display())
    );
}

#[test]
fn test_db_reload_preserves_selection_by_uuid() {
    use papyrus_core::db::{Collection, CollectionRepo, PaperRepo};

    let conn = papyrus_core::db::open_in_memory().unwrap();
    let col_a = Collection {
        id: Uuid::now_v7(),
        name: "Col A".to_string(),
        parent_id: None,
    };
    let col_b = Collection {
        id: Uuid::now_v7(),
        name: "Col B".to_string(),
        parent_id: None,
    };
    CollectionRepo::insert(&conn, &col_a).unwrap();
    CollectionRepo::insert(&conn, &col_b).unwrap();

    let p1 = dummy_paper("Paper B1", "Author 1", 2021);
    let p2 = dummy_paper("Paper B2", "Author 2", 2022);
    PaperRepo::insert(&conn, &p1).unwrap();
    PaperRepo::insert(&conn, &p2).unwrap();

    CollectionRepo::add_paper(&conn, p1.id, col_b.id).unwrap();
    CollectionRepo::add_paper(&conn, p2.id, col_b.id).unwrap();

    let mut app = App::from_db_conn(conn).unwrap();
    // collections: [All Papers, Col A, Col B]
    let col_b_idx = app
        .collections
        .iter()
        .position(|c| c.id == Some(col_b.id))
        .unwrap();
    assert_eq!(col_b_idx, 2);

    app.selected_collection = col_b_idx;
    app.sync_current_selection();

    // In Col B, select p2 (index 1)
    app.selected_paper = 1;
    app.update_selection_from_indices();
    assert_eq!(app.selection.paper_id, Some(p2.id));

    // Now insert a new collection in the DB that shifts alphabetical ordering: "Col 0"
    if let Some(ref conn) = app.db_conn {
        let col_0 = Collection {
            id: Uuid::now_v7(),
            name: "Col 0".to_string(),
            parent_id: None,
        };
        CollectionRepo::insert(conn, &col_0).unwrap();
    }

    // Reload from DB
    app.reload_from_db().unwrap();

    // Col B should have shifted to index 3: [All Papers, Col 0, Col A, Col B]
    assert_eq!(app.collections[app.selected_collection].id, Some(col_b.id));
    assert_eq!(app.selected_collection, 3);
    assert_eq!(app.selection.collection, CollectionKey::Real(col_b.id));

    // Paper B2 should still be selected by UUID
    assert_eq!(app.selection.paper_id, Some(p2.id));
    assert_eq!(app.papers[app.selected_paper].id, p2.id);
}

#[test]
fn test_position_memory_roundtrip() {
    let col1_id = Uuid::now_v7();
    let col2_id = Uuid::now_v7();

    let p1 = dummy_paper("P1", "A1", 2020);
    let p2 = dummy_paper("P2", "A2", 2021);
    let p3 = dummy_paper("P3", "A3", 2022);
    let p4 = dummy_paper("P4", "A4", 2023);

    let p2_id = p2.id;
    let p4_id = p4.id;

    let col1 = CollectionItem::new(Some(col1_id), "Col 1", 2);
    let col2 = CollectionItem::new(Some(col2_id), "Col 2", 2);

    let mut papers_by_col = HashMap::new();
    papers_by_col.insert(Some(col1_id), vec![p1, p2]);
    papers_by_col.insert(Some(col2_id), vec![p3, p4]);

    let mut app = App::with_data(vec![col1, col2], papers_by_col, HashMap::new());
    app.active_panel = ActivePanel::Collections;

    // Initially at col 0, papers: [P1, P2]
    assert_eq!(app.selected_collection, 0);
    // Switch to Papers panel and move down to P2
    app.active_panel = ActivePanel::Papers;
    app.dispatch(Action::MoveDown);
    assert_eq!(app.selected_paper, 1);
    assert_eq!(app.selection.paper_id, Some(p2_id));

    // Switch to Collections panel and move down to Col 2
    app.active_panel = ActivePanel::Collections;
    app.dispatch(Action::MoveDown);
    assert_eq!(app.selected_collection, 1);
    assert_eq!(app.selected_paper, 0); // Col 2 has no memory yet -> defaults to P3 (0)

    // Select P4 in Col 2
    app.active_panel = ActivePanel::Papers;
    app.dispatch(Action::MoveDown);
    assert_eq!(app.selected_paper, 1);
    assert_eq!(app.selection.paper_id, Some(p4_id));

    // Switch to Collections panel and move back up to Col 1
    app.active_panel = ActivePanel::Collections;
    app.dispatch(Action::MoveUp);
    assert_eq!(app.selected_collection, 0);
    // Memory should have restored P2 (index 1)!
    assert_eq!(app.selected_paper, 1);
    assert_eq!(app.selection.paper_id, Some(p2_id));

    // Move back down to Col 2
    app.dispatch(Action::MoveDown);
    assert_eq!(app.selected_collection, 1);
    // Memory should have restored P4 (index 1)!
    assert_eq!(app.selected_paper, 1);
    assert_eq!(app.selection.paper_id, Some(p4_id));
}

#[test]
fn test_position_memory_by_paper_toc() {
    let col_id = Uuid::now_v7();
    let p1 = dummy_paper("P1", "A1", 2020);
    let p2 = dummy_paper("P2", "A2", 2021);

    let p1_id = p1.id;
    let p2_id = p2.id;

    let t1_1 = dummy_toc(p1_id, "TOC 1.1", 1);
    let t1_2 = dummy_toc(p1_id, "TOC 1.2", 5);
    let t1_3 = dummy_toc(p1_id, "TOC 1.3", 10);
    let t1_3_id = t1_3.id;

    let t2_1 = dummy_toc(p2_id, "TOC 2.1", 1);
    let t2_2 = dummy_toc(p2_id, "TOC 2.2", 20);

    let col = CollectionItem::new(Some(col_id), "Col 1", 2);

    let mut papers_by_col = HashMap::new();
    papers_by_col.insert(Some(col_id), vec![p1, p2]);

    let mut tocs = HashMap::new();
    tocs.insert(p1_id, vec![t1_1, t1_2, t1_3]);
    tocs.insert(p2_id, vec![t2_1, t2_2]);

    let mut app = App::with_data(vec![col], papers_by_col, tocs);

    // Currently at P1 (index 0)
    assert_eq!(app.selected_paper, 0);
    // Move to Details panel and select TOC 1.3 (index 2)
    app.active_panel = ActivePanel::Details;
    app.dispatch(Action::MoveDown); // index 1
    app.dispatch(Action::MoveDown); // index 2
    assert_eq!(app.selected_toc, 2);
    assert_eq!(app.selection.toc_id, Some(t1_3_id));

    // Move back to Papers panel and move down to P2
    app.active_panel = ActivePanel::Papers;
    app.dispatch(Action::MoveDown);
    assert_eq!(app.selected_paper, 1);
    assert_eq!(app.selected_toc, 0); // P2 defaults to TOC 2.1 (index 0)

    // Move back to P1
    app.dispatch(Action::MoveUp);
    assert_eq!(app.selected_paper, 0);
    // Memory should have restored TOC 1.3 (index 2)!
    assert_eq!(app.selected_toc, 2);
    assert_eq!(app.selection.toc_id, Some(t1_3_id));
}

#[test]
fn test_delete_fallback_selection() {
    let col_id = Uuid::now_v7();
    let p1 = dummy_paper("P1", "A1", 2020);
    let p2 = dummy_paper("P2", "A2", 2021);
    let p3 = dummy_paper("P3", "A3", 2022);

    let p1_id = p1.id;
    let p2_id = p2.id;
    let p3_id = p3.id;

    let col = CollectionItem::new(Some(col_id), "Col 1", 3);
    let mut papers_by_col = HashMap::new();
    papers_by_col.insert(Some(col_id), vec![p1, p2, p3]);

    let mut app = App::with_data(vec![col], papers_by_col, HashMap::new());
    app.active_panel = ActivePanel::Papers;

    // 1. Delete last element (P3 at index 2)
    app.selected_paper = 2;
    app.update_selection_from_indices();
    assert_eq!(app.selection.paper_id, Some(p3_id));

    app.dispatch(Action::DeleteConfirmOpen);
    app.dispatch(Action::DeleteConfirmExecute);

    // Papers list is now [P1, P2]. Selected paper should clamp to index 1 (P2), not panic
    assert_eq!(app.papers.len(), 2);
    assert_eq!(app.selected_paper, 1);
    assert_eq!(app.selection.paper_id, Some(p2_id));

    // 2. Delete middle element (P2 at index 1)
    app.dispatch(Action::DeleteConfirmOpen);
    app.dispatch(Action::DeleteConfirmExecute);

    // Papers list is now [P1]. Selected paper should clamp to index 0 (P1)
    assert_eq!(app.papers.len(), 1);
    assert_eq!(app.selected_paper, 0);
    assert_eq!(app.selection.paper_id, Some(p1_id));

    // 3. Delete only element (P1 at index 0)
    app.dispatch(Action::DeleteConfirmOpen);
    app.dispatch(Action::DeleteConfirmExecute);

    // Papers list is now empty. Selected paper is 0, selection.paper_id is None, no panic
    assert_eq!(app.papers.len(), 0);
    assert_eq!(app.selected_paper, 0);
    assert_eq!(app.selection.paper_id, None);
    assert_eq!(app.toc_preview.len(), 0);
    assert_eq!(app.selected_toc, 0);
    assert_eq!(app.selection.toc_id, None);
}

#[test]
fn test_h_l_clamping_and_tab_backtab_cycling() {
    let mut app = App::new();
    assert_eq!(app.active_panel, ActivePanel::Collections);

    // 'h' at Collections is clamped at Collections
    app.dispatch(Action::PanelLeft);
    assert_eq!(app.active_panel, ActivePanel::Collections);

    // 'l' moves to Papers
    app.dispatch(Action::PanelRight);
    assert_eq!(app.active_panel, ActivePanel::Papers);

    // 'l' moves to Details
    app.dispatch(Action::PanelRight);
    assert_eq!(app.active_panel, ActivePanel::Details);

    // 'l' at Details is clamped at Details
    app.dispatch(Action::PanelRight);
    assert_eq!(app.active_panel, ActivePanel::Details);

    // 'h' moves back to Papers
    app.dispatch(Action::PanelLeft);
    assert_eq!(app.active_panel, ActivePanel::Papers);

    // 'h' moves back to Collections
    app.dispatch(Action::PanelLeft);
    assert_eq!(app.active_panel, ActivePanel::Collections);

    // Tab cycles: Collections -> Papers -> Details -> Collections
    app.dispatch(Action::NextPanel);
    assert_eq!(app.active_panel, ActivePanel::Papers);
    app.dispatch(Action::NextPanel);
    assert_eq!(app.active_panel, ActivePanel::Details);
    app.dispatch(Action::NextPanel);
    assert_eq!(app.active_panel, ActivePanel::Collections);

    // BackTab cycles in reverse: Collections -> Details -> Papers -> Collections
    app.dispatch(Action::PreviousPanel);
    assert_eq!(app.active_panel, ActivePanel::Details);
    app.dispatch(Action::PreviousPanel);
    assert_eq!(app.active_panel, ActivePanel::Papers);
    app.dispatch(Action::PreviousPanel);
    assert_eq!(app.active_panel, ActivePanel::Collections);
}

#[test]
fn test_enter_in_collections_focuses_papers() {
    let mut app = App::new();
    app.active_panel = ActivePanel::Collections;
    app.layout_mode = LayoutMode::MultiPanel;

    // In MultiPanel mode, Enter focuses Papers
    let enter = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    );
    let action = crate::event::map_key_event_for_app(enter, &app);
    assert_eq!(action, Some(Action::FocusPapers));

    app.dispatch(action.unwrap());
    assert_eq!(app.active_panel, ActivePanel::Papers);
    assert_eq!(app.layout_mode, LayoutMode::MultiPanel);

    // In SinglePanel mode, Enter focuses Papers and visible panel will be Papers
    app.active_panel = ActivePanel::Collections;
    app.layout_mode = LayoutMode::SinglePanel;

    let action = crate::event::map_key_event_for_app(enter, &app);
    assert_eq!(action, Some(Action::FocusPapers));

    app.dispatch(action.unwrap());
    assert_eq!(app.active_panel, ActivePanel::Papers);
    assert_eq!(app.layout_mode, LayoutMode::SinglePanel);
}

#[test]
fn test_all_motion_variants() {
    let col = CollectionItem::new(None, "All Papers", 5);
    let p0 = dummy_paper("P0", "A0", 2020);
    let p1 = dummy_paper("P1", "A1", 2021);
    let p2 = dummy_paper("P2", "A2", 2022);
    let p3 = dummy_paper("P3", "A3", 2023);
    let p4 = dummy_paper("P4", "A4", 2024);

    let mut map = HashMap::new();
    map.insert(None, vec![p0, p1, p2, p3, p4]);
    let mut app = App::with_data(vec![col], map, HashMap::new());
    app.active_panel = ActivePanel::Papers;
    assert_eq!(app.selected_paper, 0);

    // Relative(2) -> index 2
    app.dispatch(Action::Motion(Motion::Relative(2)));
    assert_eq!(app.selected_paper, 2);

    // Relative(-1) -> index 1
    app.dispatch(Action::Motion(Motion::Relative(-1)));
    assert_eq!(app.selected_paper, 1);

    // First -> index 0
    app.dispatch(Action::Motion(Motion::First));
    assert_eq!(app.selected_paper, 0);

    // Last -> index 4
    app.dispatch(Action::Motion(Motion::Last));
    assert_eq!(app.selected_paper, 4);

    // Absolute(3) (1-based index 3 -> 0-based index 2)
    app.dispatch(Action::Motion(Motion::Absolute(3)));
    assert_eq!(app.selected_paper, 2);

    // Absolute(0) -> 0
    app.dispatch(Action::Motion(Motion::Absolute(0)));
    assert_eq!(app.selected_paper, 0);

    // Absolute(999) -> clamped to 4
    app.dispatch(Action::Motion(Motion::Absolute(999)));
    assert_eq!(app.selected_paper, 4);

    // HalfPageUp -> clamped to 0
    app.dispatch(Action::Motion(Motion::HalfPageUp));
    assert_eq!(app.selected_paper, 0);

    // HalfPageDown -> clamped to 4
    app.dispatch(Action::Motion(Motion::HalfPageDown));
    assert_eq!(app.selected_paper, 4);
}

#[test]
fn test_vim_motions_gg_g_ctrl_d_u() {
    let col = CollectionItem::new(None, "All Papers", 25);
    let papers: Vec<Paper> = (0..25)
        .map(|i| dummy_paper(&format!("Paper {i}"), "Author", 2000 + i))
        .collect();
    let mut map = HashMap::new();
    map.insert(None, papers);

    let mut app = App::with_data(vec![col], map, HashMap::new());
    app.active_panel = ActivePanel::Papers;
    assert_eq!(app.selected_paper, 0);

    // Press 'G' -> moves to last item (index 24)
    let g_upper = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('G'),
        crossterm::event::KeyModifiers::NONE,
    );
    let act = crate::event::map_key_event_for_app(g_upper, &app);
    assert_eq!(act, Some(Action::Motion(Motion::Last)));
    app.dispatch(act.unwrap());
    assert_eq!(app.selected_paper, 24);

    // Press 'g' once -> sets pending chord 'g'
    let g_lower = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('g'),
        crossterm::event::KeyModifiers::NONE,
    );
    let act = crate::event::map_key_event_for_app(g_lower, &app);
    assert_eq!(act, Some(Action::PendingChord('g')));
    app.dispatch(act.unwrap());
    assert_eq!(app.pending_chord, Some('g'));

    // Press 'g' second time -> moves to first item (index 0)
    let act = crate::event::map_key_event_for_app(g_lower, &app);
    assert_eq!(act, Some(Action::Motion(Motion::First)));
    app.dispatch(act.unwrap());
    assert_eq!(app.selected_paper, 0);
    assert_eq!(app.pending_chord, None);

    // Ctrl-d -> half page down (step 10) -> index 10
    let ctrl_d = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('d'),
        crossterm::event::KeyModifiers::CONTROL,
    );
    let act = crate::event::map_key_event_for_app(ctrl_d, &app);
    assert_eq!(act, Some(Action::Motion(Motion::HalfPageDown)));
    app.dispatch(act.unwrap());
    assert_eq!(app.selected_paper, 10);

    // Ctrl-u -> half page up (step 10) -> index 0
    let ctrl_u = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('u'),
        crossterm::event::KeyModifiers::CONTROL,
    );
    let act = crate::event::map_key_event_for_app(ctrl_u, &app);
    assert_eq!(act, Some(Action::Motion(Motion::HalfPageUp)));
    app.dispatch(act.unwrap());
    assert_eq!(app.selected_paper, 0);
}

#[test]
fn test_count_prefixes_and_reset() {
    let col = CollectionItem::new(None, "All Papers", 30);
    let papers: Vec<Paper> = (0..30)
        .map(|i| dummy_paper(&format!("Paper {i}"), "Author", 2000 + i))
        .collect();
    let mut map = HashMap::new();
    map.insert(None, papers);

    let mut app = App::with_data(vec![col], map, HashMap::new());
    app.active_panel = ActivePanel::Papers;
    assert_eq!(app.selected_paper, 0);

    // '5' then 'j' -> moves down 5
    let key_5 = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('5'),
        crossterm::event::KeyModifiers::NONE,
    );
    let act = crate::event::map_key_event_for_app(key_5, &app);
    assert_eq!(act, Some(Action::CountDigit(5)));
    app.dispatch(act.unwrap());
    assert_eq!(app.pending_count, Some(5));

    let key_j = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('j'),
        crossterm::event::KeyModifiers::NONE,
    );
    let act = crate::event::map_key_event_for_app(key_j, &app);
    assert_eq!(act, Some(Action::MoveDown));
    app.dispatch(act.unwrap());
    assert_eq!(app.selected_paper, 5);
    assert_eq!(app.pending_count, None);

    // '1', '0', 'k' -> moves up 10 (clamped to 0)
    let key_1 = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('1'),
        crossterm::event::KeyModifiers::NONE,
    );
    app.dispatch(crate::event::map_key_event_for_app(key_1, &app).unwrap());
    let key_0 = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('0'),
        crossterm::event::KeyModifiers::NONE,
    );
    app.dispatch(crate::event::map_key_event_for_app(key_0, &app).unwrap());
    assert_eq!(app.pending_count, Some(10));

    let key_k = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('k'),
        crossterm::event::KeyModifiers::NONE,
    );
    app.dispatch(crate::event::map_key_event_for_app(key_k, &app).unwrap());
    assert_eq!(app.selected_paper, 0);
    assert_eq!(app.pending_count, None);

    // '2', '0', 'G' -> moves to 1-based line 20 (0-based index 19)
    let key_2 = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('2'),
        crossterm::event::KeyModifiers::NONE,
    );
    app.dispatch(crate::event::map_key_event_for_app(key_2, &app).unwrap());
    app.dispatch(crate::event::map_key_event_for_app(key_0, &app).unwrap());
    assert_eq!(app.pending_count, Some(20));

    let key_g_upper = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('G'),
        crossterm::event::KeyModifiers::NONE,
    );
    let act = crate::event::map_key_event_for_app(key_g_upper, &app);
    assert_eq!(act, Some(Action::Motion(Motion::Absolute(20))));
    app.dispatch(act.unwrap());
    assert_eq!(app.selected_paper, 19);
    assert_eq!(app.pending_count, None);

    // Count reset upon Esc
    app.dispatch(Action::CountDigit(7));
    assert_eq!(app.pending_count, Some(7));
    let esc = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    );
    let act = crate::event::map_key_event_for_app(esc, &app);
    assert_eq!(act, Some(Action::ResetNavigationState));
    app.dispatch(act.unwrap());
    assert_eq!(app.pending_count, None);

    // Chord reset upon Esc
    app.dispatch(Action::PendingChord('g'));
    assert_eq!(app.pending_chord, Some('g'));
    let act = crate::event::map_key_event_for_app(esc, &app);
    assert_eq!(act, Some(Action::ResetNavigationState));
    app.dispatch(act.unwrap());
    assert_eq!(app.pending_chord, None);
}

#[test]
fn test_layout_mode_toggle_and_esc_return() {
    let mut app = App::new();
    assert_eq!(app.layout_mode, LayoutMode::MultiPanel);

    // Ctrl-w toggles to SinglePanel
    let ctrl_w = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('w'),
        crossterm::event::KeyModifiers::CONTROL,
    );
    let act = crate::event::map_key_event_for_app(ctrl_w, &app);
    assert_eq!(act, Some(Action::ToggleLayoutMode));
    app.dispatch(act.unwrap());
    assert_eq!(app.layout_mode, LayoutMode::SinglePanel);

    // Ctrl-w toggles back to MultiPanel
    app.dispatch(Action::ToggleLayoutMode);
    assert_eq!(app.layout_mode, LayoutMode::MultiPanel);

    // Esc returns from SinglePanel when search query is empty and no modals open
    app.layout_mode = LayoutMode::SinglePanel;
    let esc = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    );
    let act = crate::event::map_key_event_for_app(esc, &app);
    assert_eq!(act, Some(Action::ToggleLayoutMode));
    app.dispatch(act.unwrap());
    assert_eq!(app.layout_mode, LayoutMode::MultiPanel);
}

#[test]
fn test_paper_sorting_cycles_and_uuid_selection_preservation() {
    let col = CollectionItem::new(None, "All Papers", 4);
    let mut p1 = dummy_paper("Alpha", "Author C", 2020);
    p1.created_at = "2024-01-01T00:00:00Z".to_string();
    let mut p2 = dummy_paper("Beta", "Author A", 2022);
    p2.created_at = "2024-01-03T00:00:00Z".to_string();
    let mut p3 = dummy_paper("Gamma", "Author D", 2019);
    p3.created_at = "2024-01-02T00:00:00Z".to_string();
    let mut p4 = dummy_paper("Delta", "Author B", 2021);
    p4.created_at = "2024-01-04T00:00:00Z".to_string();

    let p2_id = p2.id;

    let mut map = HashMap::new();
    map.insert(None, vec![p1, p2, p3, p4]);

    let mut app = App::with_data(vec![col], map, HashMap::new());
    app.active_panel = ActivePanel::Papers;

    // Set initial sort to Added Desc
    app.sort_field = PaperSortField::Added;
    app.sort_direction = SortDirection::Desc;
    app.sort_current_papers();

    // Select Beta (p2_id)
    let p2_idx = app.papers.iter().position(|p| p.id == p2_id).unwrap();
    app.selected_paper = p2_idx;
    app.update_selection_from_indices();
    assert_eq!(app.selection.paper_id, Some(p2_id));

    // Cycle 1: Added (Desc) -> Year (Desc)
    let s_key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('S'),
        crossterm::event::KeyModifiers::NONE,
    );
    let act = crate::event::map_key_event_for_app(s_key, &app);
    assert_eq!(act, Some(Action::CyclePaperSort));
    app.dispatch(act.unwrap());

    assert_eq!(app.sort_field, PaperSortField::Year);
    assert_eq!(app.sort_direction, SortDirection::Desc);
    assert_eq!(app.sort_field.badge(app.sort_direction), "Year↓");
    // In Year Desc: 2022 (Beta), 2021 (Delta), 2020 (Alpha), 2019 (Gamma)
    assert_eq!(app.papers[0].id, p2_id);
    assert_eq!(app.selected_paper, 0);
    assert_eq!(app.selection.paper_id, Some(p2_id));

    // Cycle 2: Year (Desc) -> Title (Asc)
    app.dispatch(Action::CyclePaperSort);
    assert_eq!(app.sort_field, PaperSortField::Title);
    assert_eq!(app.sort_direction, SortDirection::Asc);
    assert_eq!(app.sort_field.badge(app.sort_direction), "Title↑");
    // In Title Asc: Alpha (0), Beta (1), Delta (2), Gamma (3)
    assert_eq!(app.papers[1].id, p2_id);
    assert_eq!(app.selected_paper, 1);
    assert_eq!(app.selection.paper_id, Some(p2_id));

    // Cycle 3: Title (Asc) -> Author (Asc)
    app.dispatch(Action::CyclePaperSort);
    assert_eq!(app.sort_field, PaperSortField::Author);
    assert_eq!(app.sort_direction, SortDirection::Asc);
    assert_eq!(app.sort_field.badge(app.sort_direction), "Author↑");
    // In Author Asc: Author A (Beta, idx 0), Author B (Delta, idx 1), Author C (Alpha, idx 2), Author D (Gamma, idx 3)
    assert_eq!(app.papers[0].id, p2_id);
    assert_eq!(app.selected_paper, 0);
    assert_eq!(app.selection.paper_id, Some(p2_id));

    // Cycle 4: Author (Asc) -> Added (Desc)
    app.dispatch(Action::CyclePaperSort);
    assert_eq!(app.sort_field, PaperSortField::Added);
    assert_eq!(app.sort_direction, SortDirection::Desc);
    assert_eq!(app.sort_field.badge(app.sort_direction), "Added↓");
    // Selection UUID still preserved
    assert_eq!(app.selection.paper_id, Some(p2_id));
    assert_eq!(app.papers[app.selected_paper].id, p2_id);
}

#[test]
fn test_breadcrumb_generation_and_indicators() {
    let id_root = Uuid::new_v4();
    let id_child = Uuid::new_v4();
    let id_grandchild = Uuid::new_v4();

    let root_item = CollectionItem::with_hierarchy(Some(id_root), "Computer Science", 10, 0, None);
    let child_item =
        CollectionItem::with_hierarchy(Some(id_child), "Machine Learning", 6, 1, Some(id_root));
    let grandchild_item =
        CollectionItem::with_hierarchy(Some(id_grandchild), "Deep Learning", 3, 2, Some(id_child));

    let mut app = App::new();
    app.collections = vec![root_item, child_item, grandchild_item];
    app.update_breadcrumbs();

    assert_eq!(
        app.collection_breadcrumbs
            .get(&CollectionKey::Real(id_grandchild))
            .map(String::as_str),
        Some("Computer Science / Machine Learning / Deep Learning")
    );
    assert_eq!(
        app.collection_breadcrumbs
            .get(&CollectionKey::Real(id_child))
            .map(String::as_str),
        Some("Computer Science / Machine Learning")
    );
    assert_eq!(
        app.collection_breadcrumbs
            .get(&CollectionKey::Real(id_root))
            .map(String::as_str),
        Some("Computer Science")
    );

    // Test render indicators
    let backend = ratatui::backend::TestBackend::new(120, 30);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|f| crate::ui::render(&app, f)).unwrap();

    let buffer = terminal.backend().buffer();
    let mut rendered = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            rendered.push_str(buffer[(x, y)].symbol());
        }
        rendered.push('\n');
    }
    assert!(rendered.contains("Collections · 1/3"));
}

#[test]
fn test_esc_in_single_panel_search_cancels_search_cleanly() {
    let mut app = App::new();
    app.layout_mode = LayoutMode::SinglePanel;
    app.is_searching = true;
    app.search_query.clear();

    // Esc while searching in SinglePanel with empty query
    let esc = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    );
    let act = crate::event::map_key_event_for_app(esc, &app);
    assert_eq!(act, Some(Action::SearchCancel));

    app.dispatch(act.unwrap());
    assert!(
        !app.is_searching,
        "is_searching must be false after SearchCancel"
    );
    assert_eq!(
        app.layout_mode,
        LayoutMode::SinglePanel,
        "layout_mode should remain SinglePanel on search cancel"
    );

    // Pressing Esc again while in normal mode SinglePanel toggles layout to MultiPanel
    let act2 = crate::event::map_key_event_for_app(esc, &app);
    assert_eq!(act2, Some(Action::ToggleLayoutMode));
    app.dispatch(act2.unwrap());
    assert_eq!(app.layout_mode, LayoutMode::MultiPanel);
}

#[test]
fn test_navigating_collections_applies_active_sort() {
    let col1_id = Uuid::new_v4();
    let col2_id = Uuid::new_v4();
    let col1 = CollectionItem::new(Some(col1_id), "Col 1", 2);
    let col2 = CollectionItem::new(Some(col2_id), "Col 2", 3);

    let p1 = dummy_paper("Col1 Paper 2020", "Author A", 2020);
    let p2 = dummy_paper("Col1 Paper 2022", "Author B", 2022);

    let p3 = dummy_paper("Col2 Paper 2018", "Author C", 2018);
    let p4 = dummy_paper("Col2 Paper 2023", "Author D", 2023);
    let p5 = dummy_paper("Col2 Paper 2021", "Author E", 2021);

    let mut map = HashMap::new();
    map.insert(Some(col1_id), vec![p1, p2]);
    map.insert(Some(col2_id), vec![p3, p4, p5]);

    let mut app = App::with_data(vec![col1, col2], map, HashMap::new());
    app.active_panel = ActivePanel::Collections;
    app.sort_field = PaperSortField::Year;
    app.sort_direction = SortDirection::Desc;

    // Initially at collection 0
    assert_eq!(app.selected_collection, 0);

    // Navigate down to collection 1
    app.dispatch(Action::MoveDown);
    assert_eq!(app.selected_collection, 1);

    // Verify papers in collection 2 are sorted according to Year Desc: 2023, 2021, 2018
    assert_eq!(app.papers.len(), 3);
    assert_eq!(app.papers[0].year, Some(2023));
    assert_eq!(app.papers[1].year, Some(2021));
    assert_eq!(app.papers[2].year, Some(2018));
}

#[test]
fn test_cycle_sort_preserves_search_filter() {
    let col = CollectionItem::new(None, "All Papers", 4);
    let p1 = dummy_paper("Alpha ML", "Author C", 2020);
    let p2 = dummy_paper("Beta AI", "Author A", 2022);
    let p3 = dummy_paper("Gamma ML", "Author D", 2019);
    let p4 = dummy_paper("Delta Systems", "Author B", 2021);

    let mut map = HashMap::new();
    map.insert(None, vec![p1, p2, p3, p4]);

    let mut app = App::with_data(vec![col], map, HashMap::new());
    app.active_panel = ActivePanel::Papers;

    // Search for "ML"
    app.dispatch(Action::Search);
    for c in "ml".chars() {
        app.dispatch(Action::SearchInput(c));
    }
    app.dispatch(Action::SearchConfirm);

    assert_eq!(app.papers.len(), 2);
    assert_eq!(app.search_query, "ml");

    // Press S to cycle sort (Added Desc -> Year Desc)
    app.dispatch(Action::CyclePaperSort);
    assert_eq!(app.sort_field, PaperSortField::Year);
    assert_eq!(app.sort_direction, SortDirection::Desc);

    // The search filter must NOT be discarded! Still only 2 papers matching "ml"
    assert_eq!(app.papers.len(), 2);
    // Year Desc: Alpha (2020) then Gamma (2019)
    assert_eq!(app.papers[0].title.as_deref(), Some("Alpha ML"));
    assert_eq!(app.papers[1].title.as_deref(), Some("Gamma ML"));
}

#[test]
fn test_count_prefix_with_gg_chord() {
    let col = CollectionItem::new(None, "All Papers", 10);
    let papers: Vec<Paper> = (0..10)
        .map(|i| dummy_paper(&format!("Paper {i}"), "Author", 2000 + i))
        .collect();
    let mut map = HashMap::new();
    map.insert(None, papers);

    let mut app = App::with_data(vec![col], map, HashMap::new());
    app.active_panel = ActivePanel::Papers;
    assert_eq!(app.selected_paper, 0);

    // Type '5'
    let key_5 = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('5'),
        crossterm::event::KeyModifiers::NONE,
    );
    app.dispatch(crate::event::map_key_event_for_app(key_5, &app).unwrap());
    assert_eq!(app.pending_count, Some(5));

    // Type first 'g'
    let key_g = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('g'),
        crossterm::event::KeyModifiers::NONE,
    );
    let act1 = crate::event::map_key_event_for_app(key_g, &app);
    assert_eq!(act1, Some(Action::PendingChord('g')));
    app.dispatch(act1.unwrap());
    assert_eq!(app.pending_chord, Some('g'));
    assert_eq!(app.pending_count, Some(5));

    // Type second 'g' -> 5gg jumps to 5th item (1-based 5 => index 4)
    let act2 = crate::event::map_key_event_for_app(key_g, &app);
    assert_eq!(act2, Some(Action::Motion(Motion::Absolute(5))));
    app.dispatch(act2.unwrap());
    assert_eq!(app.selected_paper, 4);
    assert_eq!(app.pending_chord, None);
    assert_eq!(app.pending_count, None);
}

#[test]
fn test_visual_mode_toggle_and_range_expansion() {
    let col = CollectionItem::new(None, "All Papers", 5);
    let papers: Vec<Paper> = (0..5)
        .map(|i| dummy_paper(&format!("Paper {i}"), "Author", 2020 + i))
        .collect();
    let mut map = HashMap::new();
    map.insert(None, papers.clone());

    let mut app = App::with_data(vec![col], map, HashMap::new());
    app.active_panel = ActivePanel::Papers;
    app.selected_paper = 1;

    // Press 'V' to toggle visual mode
    let key_v = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('V'),
        crossterm::event::KeyModifiers::NONE,
    );
    let act_v = crate::event::map_key_event_for_app(key_v, &app);
    assert_eq!(act_v, Some(Action::VisualModeToggle));
    app.dispatch(act_v.unwrap());

    assert!(app.visual_mode);
    assert_eq!(app.visual_anchor, Some(1));
    assert_eq!(app.visual_selected_uuids.len(), 1);
    assert!(app.visual_selected_uuids.contains(&papers[1].id));
    assert_eq!(app.visual_selected_papers(), vec![papers[1].id]);

    // Move down 2 times using 'j'
    let key_j = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('j'),
        crossterm::event::KeyModifiers::NONE,
    );
    app.dispatch(crate::event::map_key_event_for_app(key_j, &app).unwrap());
    assert_eq!(app.selected_paper, 2);
    assert_eq!(app.visual_selected_uuids.len(), 2);

    app.dispatch(crate::event::map_key_event_for_app(key_j, &app).unwrap());
    assert_eq!(app.selected_paper, 3);
    assert_eq!(app.visual_selected_uuids.len(), 3);
    assert_eq!(
        app.visual_selected_papers(),
        vec![papers[1].id, papers[2].id, papers[3].id]
    );

    // Now move up above the anchor to paper 0
    let key_k = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('k'),
        crossterm::event::KeyModifiers::NONE,
    );
    app.dispatch(crate::event::map_key_event_for_app(key_k, &app).unwrap()); // to 2
    app.dispatch(crate::event::map_key_event_for_app(key_k, &app).unwrap()); // to 1
    app.dispatch(crate::event::map_key_event_for_app(key_k, &app).unwrap()); // to 0
    assert_eq!(app.selected_paper, 0);
    // Range 0..=1 should now be included
    assert!(app.visual_selected_uuids.contains(&papers[0].id));
    assert!(app.visual_selected_uuids.contains(&papers[1].id));
    assert_eq!(
        app.visual_selected_papers(),
        vec![papers[0].id, papers[1].id, papers[2].id, papers[3].id]
            .into_iter()
            .filter(|id| app.visual_selected_uuids.contains(id))
            .collect::<Vec<_>>()
    );

    // Toggle off by pressing 'V' again
    let act_v2 = crate::event::map_key_event_for_app(key_v, &app);
    assert_eq!(act_v2, Some(Action::VisualModeToggle));
    app.dispatch(act_v2.unwrap());
    assert!(!app.visual_mode);
    assert_eq!(app.visual_anchor, None);
    assert!(app.visual_selected_uuids.is_empty());
}

#[test]
fn test_visual_mode_space_toggle() {
    let col = CollectionItem::new(None, "All Papers", 4);
    let papers: Vec<Paper> = (0..4)
        .map(|i| dummy_paper(&format!("Paper {i}"), "Author", 2020 + i))
        .collect();
    let mut map = HashMap::new();
    map.insert(None, papers.clone());

    let mut app = App::with_data(vec![col], map, HashMap::new());
    app.active_panel = ActivePanel::Papers;
    app.selected_paper = 0;

    app.enter_visual_mode();
    assert!(app.visual_mode);

    // Move to paper 1
    app.apply_motion(Motion::Relative(1));
    assert_eq!(app.selected_paper, 1);
    assert_eq!(app.visual_selected_uuids.len(), 2);
    assert!(app.visual_selected_uuids.contains(&papers[1].id));

    // Press Space to toggle selection of paper 1
    let key_space = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char(' '),
        crossterm::event::KeyModifiers::NONE,
    );
    let act_space = crate::event::map_key_event_for_app(key_space, &app);
    assert_eq!(act_space, Some(Action::VisualModeToggleItem));
    app.dispatch(act_space.unwrap());

    // Paper 1 should now be deselected
    assert!(!app.visual_selected_uuids.contains(&papers[1].id));
    assert!(app.visual_selected_uuids.contains(&papers[0].id));
    assert_eq!(app.visual_selected_papers(), vec![papers[0].id]);

    // Press Space again to reselect paper 1
    app.dispatch(Action::VisualModeToggleItem);
    assert!(app.visual_selected_uuids.contains(&papers[1].id));
    assert_eq!(
        app.visual_selected_papers(),
        vec![papers[0].id, papers[1].id]
    );
}

#[test]
fn test_visual_mode_esc_cancel() {
    let col = CollectionItem::new(None, "All Papers", 3);
    let papers: Vec<Paper> = (0..3)
        .map(|i| dummy_paper(&format!("Paper {i}"), "Author", 2020 + i))
        .collect();
    let mut map = HashMap::new();
    map.insert(None, papers.clone());

    let mut app = App::with_data(vec![col], map, HashMap::new());
    app.active_panel = ActivePanel::Papers;
    app.selected_paper = 0;

    app.enter_visual_mode();
    app.apply_motion(Motion::Relative(1));
    assert!(app.visual_mode);
    assert_eq!(app.visual_selected_uuids.len(), 2);

    // Press Esc to cancel visual mode
    let key_esc = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
    );
    let act_esc = crate::event::map_key_event_for_app(key_esc, &app);
    assert_eq!(act_esc, Some(Action::VisualModeCancel));
    app.dispatch(act_esc.unwrap());

    assert!(!app.visual_mode);
    assert_eq!(app.visual_anchor, None);
    assert!(app.visual_selected_uuids.is_empty());
    // In normal mode, visual_selected_papers returns current paper
    assert_eq!(app.visual_selected_papers(), vec![papers[1].id]);
}

#[test]
fn test_visual_mode_panel_switch_clears() {
    let col = CollectionItem::new(None, "All Papers", 3);
    let papers: Vec<Paper> = (0..3)
        .map(|i| dummy_paper(&format!("Paper {i}"), "Author", 2020 + i))
        .collect();
    let mut map = HashMap::new();
    map.insert(None, papers.clone());

    let mut app = App::with_data(vec![col], map, HashMap::new());
    app.active_panel = ActivePanel::Papers;
    app.selected_paper = 0;

    app.enter_visual_mode();
    assert!(app.visual_mode);
    assert!(!app.visual_selected_uuids.is_empty());

    // Switch panel left (to Collections)
    app.dispatch(Action::PanelLeft);
    assert_eq!(app.active_panel, ActivePanel::Collections);
    assert!(!app.visual_mode);
    assert_eq!(app.visual_anchor, None);
    assert!(app.visual_selected_uuids.is_empty());

    // Switch back to Papers and re-enter
    app.dispatch(Action::PanelRight);
    assert_eq!(app.active_panel, ActivePanel::Papers);
    app.enter_visual_mode();
    assert!(app.visual_mode);

    // Switch panel right (to Details)
    app.dispatch(Action::PanelRight);
    assert_eq!(app.active_panel, ActivePanel::Details);
    assert!(!app.visual_mode);
    assert_eq!(app.visual_anchor, None);
    assert!(app.visual_selected_uuids.is_empty());
}

#[test]
fn test_visual_mode_enter_conditions_and_empty() {
    let mut empty_app = App::new();
    assert_eq!(empty_app.active_panel, ActivePanel::Collections);

    // In Collections panel: enter_visual_mode should not activate
    empty_app.enter_visual_mode();
    assert!(!empty_app.visual_mode);

    // Empty papers list in Papers panel: should not activate
    empty_app.active_panel = ActivePanel::Papers;
    empty_app.enter_visual_mode();
    assert!(!empty_app.visual_mode);
    assert!(empty_app.visual_selected_papers().is_empty());
}

#[test]
fn test_quick_open_jump_to_paper() {
    let mut app = App::new();
    let p1 = dummy_paper("Attention Is All You Need", "Vaswani", 2017);
    let p2 = dummy_paper("BERT: Pre-training", "Devlin", 2018);
    let p3 = dummy_paper("Deep Residual Learning", "He", 2015);

    let all_col = CollectionItem::new(None, "All Papers", 3);
    app.add_collection(all_col, vec![p1.clone(), p2.clone(), p3.clone()]);

    assert_eq!(app.selected_paper, 0);

    // Open Quick Open
    app.dispatch(Action::QuickOpenModalOpen);
    assert!(app.active_picker.is_some());
    assert_eq!(app.picker_context, Some(PickerContext::QuickOpen));

    // Type "bert" to filter
    for c in ['b', 'e', 'r', 't'] {
        app.dispatch(Action::PickerInput(c));
    }

    let picker = app.active_picker.as_ref().unwrap();
    assert_eq!(picker.visible_indices.len(), 1);
    assert_eq!(picker.selected_item().unwrap().id, p2.id);

    // Confirm selection
    app.dispatch(Action::PickerConfirm);
    assert!(app.active_picker.is_none());
    assert_eq!(app.picker_context, None);
    assert_eq!(app.active_panel, ActivePanel::Papers);
    assert_eq!(app.current_paper().unwrap().id, p2.id);
    assert_eq!(app.selected_paper, 1);
}

#[test]
fn test_quick_open_jump_to_collection() {
    let mut app = App::new();
    let col1 = CollectionItem::new(None, "All Papers", 2);
    let c2_id = Uuid::now_v7();
    let col2 = CollectionItem::new(Some(c2_id), "Machine Learning", 2);

    let p1 = dummy_paper("Paper 1", "Author", 2020);
    let p2 = dummy_paper("Paper 2", "Author", 2021);

    app.add_collection(col1, vec![p1.clone(), p2.clone()]);
    app.add_collection(col2, vec![p1.clone(), p2.clone()]);

    assert_eq!(app.selected_collection, 0);

    // Open Quick Open
    app.open_quick_open();
    assert!(app.active_picker.is_some());

    // Filter "Machine"
    for c in ['m', 'a', 'c', 'h'] {
        app.dispatch(Action::PickerInput(c));
    }

    // Confirm selection
    app.dispatch(Action::PickerConfirm);
    assert!(app.active_picker.is_none());
    assert_eq!(app.active_panel, ActivePanel::Papers);
    assert_eq!(app.selected_collection, 1);
    assert_eq!(app.current_collection().unwrap().name, "Machine Learning");
}

#[test]
fn test_collection_membership_picker_single_paper() {
    let conn = papyrus_core::db::open_in_memory().unwrap();
    let c1 = papyrus_core::db::Collection {
        id: Uuid::now_v7(),
        name: "Collection 1".to_string(),
        parent_id: None,
    };
    let c2 = papyrus_core::db::Collection {
        id: Uuid::now_v7(),
        name: "Collection 2".to_string(),
        parent_id: None,
    };
    papyrus_core::db::CollectionRepo::insert(&conn, &c1).unwrap();
    papyrus_core::db::CollectionRepo::insert(&conn, &c2).unwrap();

    let p1 = dummy_paper("Paper One", "Author", 2022);
    papyrus_core::db::PaperRepo::insert(&conn, &p1).unwrap();
    papyrus_core::db::CollectionRepo::add_paper(&conn, p1.id, c1.id).unwrap();

    let mut app = App::from_db_conn(conn).unwrap();
    app.active_panel = ActivePanel::Papers;
    app.selected_paper = 0;

    // Open collection membership modal
    app.dispatch(Action::CollectionMembershipModalOpen);
    assert!(app.active_picker.is_some());
    assert_eq!(
        app.picker_context,
        Some(PickerContext::CollectionMembership {
            target_papers: vec![p1.id]
        })
    );

    let picker = app.active_picker.as_ref().unwrap();
    assert!(picker.checked.contains(&c1.id));
    assert!(!picker.checked.contains(&c2.id));

    // Toggle items: uncheck c1, check c2
    // Move to c1 and toggle, move to c2 and toggle
    let c1_pos = picker
        .visible_indices
        .iter()
        .position(|&idx| picker.items[idx].id == c1.id)
        .unwrap();
    let c2_pos = picker
        .visible_indices
        .iter()
        .position(|&idx| picker.items[idx].id == c2.id)
        .unwrap();

    // Select c1 and toggle
    app.active_picker.as_mut().unwrap().selected = c1_pos;
    app.dispatch(Action::PickerToggleItem);

    // Select c2 and toggle
    app.active_picker.as_mut().unwrap().selected = c2_pos;
    app.dispatch(Action::PickerToggleItem);

    // Confirm changes
    app.dispatch(Action::PickerConfirm);
    assert!(app.active_picker.is_none());

    // Verify in db
    let db_conn = app.db_conn.as_ref().unwrap();
    let p1_cols =
        papyrus_core::db::CollectionRepo::get_collections_for_paper(db_conn, p1.id).unwrap();
    let p1_col_ids: Vec<Uuid> = p1_cols.into_iter().map(|c| c.id).collect();
    assert!(!p1_col_ids.contains(&c1.id));
    assert!(p1_col_ids.contains(&c2.id));
}

#[test]
fn test_collection_membership_picker_batch_papers() {
    let conn = papyrus_core::db::open_in_memory().unwrap();
    let c1 = papyrus_core::db::Collection {
        id: Uuid::now_v7(),
        name: "Col A".to_string(),
        parent_id: None,
    };
    let c2 = papyrus_core::db::Collection {
        id: Uuid::now_v7(),
        name: "Col B".to_string(),
        parent_id: None,
    };
    papyrus_core::db::CollectionRepo::insert(&conn, &c1).unwrap();
    papyrus_core::db::CollectionRepo::insert(&conn, &c2).unwrap();

    let p1 = dummy_paper("Paper A", "Author", 2022);
    let p2 = dummy_paper("Paper B", "Author", 2023);
    papyrus_core::db::PaperRepo::insert(&conn, &p1).unwrap();
    papyrus_core::db::PaperRepo::insert(&conn, &p2).unwrap();

    papyrus_core::db::CollectionRepo::add_paper(&conn, p1.id, c1.id).unwrap();
    papyrus_core::db::CollectionRepo::add_paper(&conn, p2.id, c1.id).unwrap();

    let mut app = App::from_db_conn(conn).unwrap();
    app.active_panel = ActivePanel::Papers;
    app.selected_paper = 0;

    // Visual mode select both papers
    app.enter_visual_mode();
    app.apply_motion(Motion::Relative(1));
    assert_eq!(app.visual_selected_papers().len(), 2);

    // Open collection membership modal
    app.dispatch(Action::CollectionMembershipModalOpen);
    let picker = app.active_picker.as_ref().unwrap();
    assert!(picker.checked.contains(&c1.id));
    assert!(!picker.checked.contains(&c2.id));

    // Check c2
    let c2_pos = picker
        .visible_indices
        .iter()
        .position(|&idx| picker.items[idx].id == c2.id)
        .unwrap();
    app.active_picker.as_mut().unwrap().selected = c2_pos;
    app.dispatch(Action::PickerToggleItem);

    // Confirm
    app.dispatch(Action::PickerConfirm);
    assert!(app.active_picker.is_none());
    assert!(!app.visual_mode);

    // Verify both papers now have c1 and c2
    let db_conn = app.db_conn.as_ref().unwrap();
    for pid in [p1.id, p2.id] {
        let cols =
            papyrus_core::db::CollectionRepo::get_collections_for_paper(db_conn, pid).unwrap();
        let col_ids: Vec<Uuid> = cols.into_iter().map(|c| c.id).collect();
        assert!(col_ids.contains(&c1.id));
        assert!(col_ids.contains(&c2.id));
    }
}

#[test]
fn test_tag_picker_single_and_batch() {
    let conn = papyrus_core::db::open_in_memory().unwrap();
    let p1 = dummy_paper("Paper T1", "Author", 2022);
    let p2 = dummy_paper("Paper T2", "Author", 2023);
    papyrus_core::db::PaperRepo::insert(&conn, &p1).unwrap();
    papyrus_core::db::PaperRepo::insert(&conn, &p2).unwrap();

    papyrus_core::db::TagRepo::add_tag(&conn, p1.id, "initial").unwrap();

    let mut app = App::from_db_conn(conn).unwrap();
    app.active_panel = ActivePanel::Papers;
    app.selected_paper = 0;

    // Open tag picker for single paper
    app.dispatch(Action::TagModalOpen);
    assert!(app.active_picker.is_some());
    assert_eq!(
        app.picker_context,
        Some(PickerContext::TagManagement {
            target_papers: vec![p1.id]
        })
    );

    // Add a new tag via query buffer
    for c in ['n', 'e', 'w', 't', 'a', 'g'] {
        app.dispatch(Action::PickerInput(c));
    }
    app.dispatch(Action::PickerConfirm);
    assert!(app.active_picker.is_none());

    // Verify p1 tags in DB
    let db_conn = app.db_conn.as_ref().unwrap();
    let p1_tags = papyrus_core::db::TagRepo::get_tags_for_paper(db_conn, p1.id).unwrap();
    assert!(p1_tags.contains(&"initial".to_string()));
    assert!(p1_tags.contains(&"newtag".to_string()));

    // Now batch tag P1 and P2 in visual mode
    app.enter_visual_mode();
    app.apply_motion(Motion::Relative(1));
    assert_eq!(app.visual_selected_papers().len(), 2);

    app.dispatch(Action::TagModalOpen);
    for c in ['c', 'o', 'm', 'm', 'o', 'n'] {
        app.dispatch(Action::PickerInput(c));
    }
    app.dispatch(Action::PickerConfirm);

    let db_conn = app.db_conn.as_ref().unwrap();
    for pid in [p1.id, p2.id] {
        let tags = papyrus_core::db::TagRepo::get_tags_for_paper(db_conn, pid).unwrap();
        assert!(tags.contains(&"common".to_string()));
    }
}

#[test]
fn test_batch_delete_in_visual_mode() {
    let conn = papyrus_core::db::open_in_memory().unwrap();
    let p1 = dummy_paper("Paper D1", "Author", 2022);
    let p2 = dummy_paper("Paper D2", "Author", 2023);
    let p3 = dummy_paper("Paper D3", "Author", 2024);
    papyrus_core::db::PaperRepo::insert(&conn, &p1).unwrap();
    papyrus_core::db::PaperRepo::insert(&conn, &p2).unwrap();
    papyrus_core::db::PaperRepo::insert(&conn, &p3).unwrap();

    let mut app = App::from_db_conn(conn).unwrap();
    app.active_panel = ActivePanel::Papers;
    app.selected_paper = 0;

    // Enter visual mode and select P1 and P2
    app.enter_visual_mode();
    app.apply_motion(Motion::Relative(1));
    assert_eq!(app.visual_selected_papers(), vec![p1.id, p2.id]);

    // Batch delete
    app.dispatch(Action::BatchDeleteConfirm);
    assert!(!app.visual_mode);

    // Verify in db and app
    let db_conn = app.db_conn.as_ref().unwrap();
    assert!(papyrus_core::db::PaperRepo::get_by_id(db_conn, p1.id)
        .unwrap()
        .is_none());
    assert!(papyrus_core::db::PaperRepo::get_by_id(db_conn, p2.id)
        .unwrap()
        .is_none());
    assert!(papyrus_core::db::PaperRepo::get_by_id(db_conn, p3.id)
        .unwrap()
        .is_some());
    assert_eq!(app.papers.len(), 1);
    assert_eq!(app.papers[0].id, p3.id);
}

#[test]
fn test_render_visual_mode_and_generic_picker() {
    let col = CollectionItem::new(None, "All Papers", 2);
    let p1 = dummy_paper("Attention Is All You Need", "Vaswani et al.", 2017);
    let p2 = dummy_paper("BERT", "Devlin et al.", 2018);
    let mut map = HashMap::new();
    map.insert(None, vec![p1.clone(), p2.clone()]);

    let mut app = App::with_data(vec![col], map, HashMap::new());
    app.active_panel = ActivePanel::Papers;
    app.selected_paper = 0;

    // 1. Render in visual mode
    app.enter_visual_mode();
    let backend = ratatui::backend::TestBackend::new(120, 30);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|f| crate::ui::render(&app, f)).unwrap();

    let buffer = terminal.backend().buffer();
    let mut rendered = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            rendered.push_str(buffer[(x, y)].symbol());
        }
        rendered.push('\n');
    }
    assert!(rendered.contains("[x] Attention Is All You"));
    assert!(rendered.contains("[ ] BERT"));
    assert!(rendered.contains("-- VISUAL (1 selected) --"));

    // 2. Render with active picker
    app.open_quick_open();
    assert!(app.active_picker.is_some());

    terminal.draw(|f| crate::ui::render(&app, f)).unwrap();
    let buffer = terminal.backend().buffer();
    let mut rendered_picker = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            rendered_picker.push_str(buffer[(x, y)].symbol());
        }
        rendered_picker.push('\n');
    }
    assert!(rendered_picker.contains("Quick Open (Ctrl-p)"));
    assert!(rendered_picker.contains("Search Query"));
}

#[test]
fn test_remove_paper_from_collection_preserves_db_record() {
    let conn = papyrus_core::db::open_in_memory().unwrap();
    let col = papyrus_core::db::Collection {
        id: Uuid::now_v7(),
        name: "Machine Learning".to_string(),
        parent_id: None,
    };
    papyrus_core::db::CollectionRepo::insert(&conn, &col).unwrap();

    let p1 = dummy_paper("Attention Is All You Need", "Vaswani et al.", 2017);
    let p2 = dummy_paper("BERT", "Devlin et al.", 2018);
    papyrus_core::db::PaperRepo::insert(&conn, &p1).unwrap();
    papyrus_core::db::PaperRepo::insert(&conn, &p2).unwrap();
    papyrus_core::db::CollectionRepo::add_paper(&conn, p1.id, col.id).unwrap();
    papyrus_core::db::CollectionRepo::add_paper(&conn, p2.id, col.id).unwrap();

    let mut app = App::from_db_conn(conn).unwrap();
    let ml_idx = app
        .collections
        .iter()
        .position(|c| c.id == Some(col.id))
        .unwrap();

    // Select "Machine Learning" collection
    app.selected_collection = ml_idx;
    app.sync_current_selection();
    assert_eq!(app.papers.len(), 2);

    app.active_panel = ActivePanel::Papers;
    let p1_pos = app.papers.iter().position(|p| p.id == p1.id).unwrap();
    app.selected_paper = p1_pos;
    app.update_selection_from_indices();

    // Press 'd' -> Action::RemoveFromCollection
    app.dispatch(Action::RemoveFromCollection);

    // Collection should now only have 1 paper (p2)
    assert_eq!(app.papers.len(), 1);
    assert_eq!(app.papers[0].id, p2.id);

    // BUT in the database, p1 is NOT deleted!
    let db_conn = app.db_conn.as_ref().unwrap();
    assert!(papyrus_core::db::PaperRepo::get_by_id(db_conn, p1.id)
        .unwrap()
        .is_some());
    assert!(papyrus_core::db::PaperRepo::get_by_id(db_conn, p2.id)
        .unwrap()
        .is_some());

    // In "All Papers", p1 is still present!
    app.selected_collection = 0; // All Papers
    app.sync_current_selection();
    assert_eq!(app.papers.len(), 2);
}

#[test]
fn test_remove_paper_from_all_papers_shows_hint() {
    let conn = papyrus_core::db::open_in_memory().unwrap();
    let p1 = dummy_paper("Attention Is All You Need", "Vaswani et al.", 2017);
    papyrus_core::db::PaperRepo::insert(&conn, &p1).unwrap();

    let mut app = App::from_db_conn(conn).unwrap();
    app.selected_collection = 0; // All Papers (id is None)
    app.sync_current_selection();
    app.active_panel = ActivePanel::Papers;
    app.selected_paper = 0;

    // Press 'd' -> Action::RemoveFromCollection in All Papers
    app.dispatch(Action::RemoveFromCollection);

    // Paper must NOT be removed or deleted
    assert_eq!(app.papers.len(), 1);
    let db_conn = app.db_conn.as_ref().unwrap();
    assert!(papyrus_core::db::PaperRepo::get_by_id(db_conn, p1.id)
        .unwrap()
        .is_some());

    // Status message tells user to use 'D' for database deletion
    assert_eq!(
        app.status_message.as_deref(),
        Some("Cannot remove from 'All Papers'; press 'D' to delete from database")
    );
}

#[test]
fn test_batch_remove_papers_from_collection() {
    let conn = papyrus_core::db::open_in_memory().unwrap();
    let col = papyrus_core::db::Collection {
        id: Uuid::now_v7(),
        name: "Deep Learning".to_string(),
        parent_id: None,
    };
    papyrus_core::db::CollectionRepo::insert(&conn, &col).unwrap();

    let p1 = dummy_paper("Paper 1", "Author 1", 2021);
    let p2 = dummy_paper("Paper 2", "Author 2", 2022);
    let p3 = dummy_paper("Paper 3", "Author 3", 2023);
    papyrus_core::db::PaperRepo::insert(&conn, &p1).unwrap();
    papyrus_core::db::PaperRepo::insert(&conn, &p2).unwrap();
    papyrus_core::db::PaperRepo::insert(&conn, &p3).unwrap();
    papyrus_core::db::CollectionRepo::add_paper(&conn, p1.id, col.id).unwrap();
    papyrus_core::db::CollectionRepo::add_paper(&conn, p2.id, col.id).unwrap();
    papyrus_core::db::CollectionRepo::add_paper(&conn, p3.id, col.id).unwrap();

    let mut app = App::from_db_conn(conn).unwrap();
    let dl_idx = app
        .collections
        .iter()
        .position(|c| c.id == Some(col.id))
        .unwrap();

    app.selected_collection = dl_idx;
    app.sync_current_selection();
    assert_eq!(app.papers.len(), 3);

    app.active_panel = ActivePanel::Papers;
    app.selected_paper = 0;

    // Enter visual mode and select first two papers
    app.enter_visual_mode();
    app.apply_motion(Motion::Relative(1));
    assert_eq!(app.visual_selected_papers().len(), 2);

    // Press 'd' in visual mode -> Action::BatchRemoveFromCollection
    app.dispatch(Action::BatchRemoveFromCollection);
    assert!(!app.visual_mode);

    // Active collection now has only 1 paper (the 3rd)
    assert_eq!(app.papers.len(), 1);
    assert_eq!(app.papers[0].id, p3.id);

    // In database, all 3 papers still exist
    let db_conn = app.db_conn.as_ref().unwrap();
    assert!(papyrus_core::db::PaperRepo::get_by_id(db_conn, p1.id)
        .unwrap()
        .is_some());
    assert!(papyrus_core::db::PaperRepo::get_by_id(db_conn, p2.id)
        .unwrap()
        .is_some());
    assert!(papyrus_core::db::PaperRepo::get_by_id(db_conn, p3.id)
        .unwrap()
        .is_some());

    // In All Papers, all 3 papers are present
    app.selected_collection = 0;
    app.sync_current_selection();
    assert_eq!(app.papers.len(), 3);
}

#[test]
fn test_permanent_delete_paper_with_capital_d() {
    let conn = papyrus_core::db::open_in_memory().unwrap();
    let col = papyrus_core::db::Collection {
        id: Uuid::now_v7(),
        name: "Test Col".to_string(),
        parent_id: None,
    };
    papyrus_core::db::CollectionRepo::insert(&conn, &col).unwrap();

    let p1 = dummy_paper("Paper To Erase", "Author", 2020);
    papyrus_core::db::PaperRepo::insert(&conn, &p1).unwrap();
    papyrus_core::db::CollectionRepo::add_paper(&conn, p1.id, col.id).unwrap();

    let mut app = App::from_db_conn(conn).unwrap();
    let col_idx = app
        .collections
        .iter()
        .position(|c| c.id == Some(col.id))
        .unwrap();

    app.selected_collection = col_idx;
    app.sync_current_selection();
    app.active_panel = ActivePanel::Papers;
    app.selected_paper = 0;

    // Key 'D' is mapped to Action::DeleteConfirmOpen
    let key_event = KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE);
    let action = crate::event::map_key_event_for_app(key_event, &app);
    assert_eq!(action, Some(Action::DeleteConfirmOpen));

    app.dispatch(Action::DeleteConfirmOpen);
    assert!(app.is_confirming_delete);

    // Confirm deletion
    app.dispatch(Action::DeleteConfirmExecute);
    assert!(!app.is_confirming_delete);

    // Paper is deleted from collection AND permanently deleted from DB
    assert_eq!(app.papers.len(), 0);
    let db_conn = app.db_conn.as_ref().unwrap();
    assert!(papyrus_core::db::PaperRepo::get_by_id(db_conn, p1.id)
        .unwrap()
        .is_none());

    // In All Papers, paper is also gone
    app.selected_collection = 0;
    app.sync_current_selection();
    assert_eq!(app.papers.len(), 0);
}

#[test]
fn test_centered_scroll_rendering_papers_table() {
    let col = CollectionItem::new(None, "All Papers", 30);
    let mut papers = Vec::new();
    for i in 0..30 {
        papers.push(dummy_paper(
            &format!("Paper Number {:02}", i),
            "Author",
            2020 + (i as i64 % 5),
        ));
    }
    let mut map = HashMap::new();
    map.insert(None, papers);

    let mut app = App::with_data(vec![col], map, HashMap::new());
    app.active_panel = ActivePanel::Papers;
    // Select paper 15 (middle of list)
    app.selected_paper = 15;
    app.update_selection_from_indices();

    let backend = ratatui::backend::TestBackend::new(120, 20);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|f| crate::ui::render(&app, f)).unwrap();

    let buffer = terminal.backend().buffer();
    let mut rendered = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            rendered.push_str(buffer[(x, y)].symbol());
        }
        rendered.push('\n');
    }

    // Paper 15 should be visible with active cursor prefix "> "
    assert!(rendered.contains("> Paper Number 15"));
    // Paper 0 should NOT be visible because table scrolled down to center item 15!
    assert!(!rendered.contains("Paper Number 00"));
}

#[test]
fn test_centered_scroll_rendering_generic_picker() {
    let mut items = Vec::new();
    for i in 0..30 {
        items.push(crate::app::PickerItem {
            id: Uuid::now_v7(),
            title: format!("Picker Item {:02}", i),
            subtitle: None,
            category: Some("Test".to_string()),
        });
    }
    let mut picker = crate::app::GenericPicker::new("Search Items", items, false);
    picker.selected = 15;

    let app = App::new();
    let backend = ratatui::backend::TestBackend::new(120, 20);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|f| crate::ui::modals::render_generic_picker(&app, f, &picker))
        .unwrap();

    let buffer = terminal.backend().buffer();
    let mut rendered = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            rendered.push_str(buffer[(x, y)].symbol());
        }
        rendered.push('\n');
    }

    // Picker item 15 should be visible with active cursor prefix "> "
    assert!(rendered.contains("> Picker Item 15"));
    // Top items should NOT be visible because picker is centered on 15
    assert!(!rendered.contains("Picker Item 00"));
}
