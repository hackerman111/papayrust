use papyrus_core::db::{
    apply_migrations, current_schema_version, open_database, open_in_memory, Collection,
    CollectionRepo, Paper, PaperRepo, RepoError, TocEntry, TocRepo, TocSource,
};
use tempfile::tempdir;
use uuid::Uuid;

fn sample_paper(name: &str) -> Paper {
    Paper {
        id: Uuid::now_v7(),
        file_path: format!("/path/to/{name}.pdf"),
        content_hash: format!("hash_{name}_{}", Uuid::now_v7()),
        title: Some(format!("Title of {name}")),
        authors: Some("Alice, Bob".to_string()),
        year: Some(2024),
        journal: Some("Journal of Testing".to_string()),
        doi: Some(format!("10.1234/{name}")),
        abstract_text: Some(format!("Abstract for {name}")),
        text_path: Some(format!("/path/to/extracted/{name}.txt")),
        annotated_pdf_path: None,
        toc_embedded_at: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    }
}

/// RT-04: SQL migrations apply in order and idempotently upon reopening the DB.
#[test]
fn test_rt_04_migration_order_and_idempotency() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("migration_test.db");

    // 1. Initial open applies migrations to version 1
    {
        let conn = open_database(&db_path).expect("open database first time");
        let version = current_schema_version(&conn).expect("get schema version");
        assert_eq!(version, 1, "Initial schema version must be 1");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .expect("query schema_migrations count");
        assert_eq!(count, 1);
    }

    // 2. Reopening database does not fail and version remains 1
    {
        let mut conn = open_database(&db_path).expect("reopen database");
        let version = current_schema_version(&conn).expect("get schema version");
        assert_eq!(version, 1, "Schema version after reopen must still be 1");

        // 3. Explicit re-application is idempotent
        apply_migrations(&mut conn).expect("re-applying migrations must succeed");
        let version_after = current_schema_version(&conn).expect("get schema version");
        assert_eq!(version_after, 1);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .expect("query schema_migrations count");
        assert_eq!(count, 1, "Migration count in table must remain 1");
    }
}

/// RT-05: Full CRUD for Paper.
#[test]
fn test_rt_05_paper_crud() {
    let conn = open_in_memory().expect("open in memory db");
    let mut paper = sample_paper("paper_crud");

    // Create
    PaperRepo::insert(&conn, &paper).expect("insert paper");

    // Read by ID
    let found = PaperRepo::get_by_id(&conn, paper.id)
        .expect("get_by_id")
        .expect("paper should exist");
    assert_eq!(found, paper);

    // Read by content_hash
    let found_hash = PaperRepo::get_by_content_hash(&conn, &paper.content_hash)
        .expect("get_by_content_hash")
        .expect("paper should exist");
    assert_eq!(found_hash, paper);

    // Read by file_path
    let found_path = PaperRepo::get_by_file_path(&conn, &paper.file_path)
        .expect("get_by_file_path")
        .expect("paper should exist");
    assert_eq!(found_path, paper);

    // Update
    paper.title = Some("Updated Title".to_string());
    paper.year = Some(2025);
    paper.updated_at = "2024-02-01T00:00:00Z".to_string();
    PaperRepo::update(&conn, &paper).expect("update paper");

    let updated = PaperRepo::get_by_id(&conn, paper.id)
        .expect("get_by_id")
        .expect("paper should exist");
    assert_eq!(updated.title, Some("Updated Title".to_string()));
    assert_eq!(updated.year, Some(2025));
    assert_eq!(updated.updated_at, "2024-02-01T00:00:00Z");

    // List
    let list = PaperRepo::list(&conn).expect("list papers");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, paper.id);

    // Delete
    let deleted = PaperRepo::delete(&conn, paper.id).expect("delete paper");
    assert!(deleted, "delete should return true for existing paper");

    let after_delete = PaperRepo::get_by_id(&conn, paper.id).expect("get_by_id after delete");
    assert!(
        after_delete.is_none(),
        "paper should not exist after delete"
    );

    let delete_again = PaperRepo::delete(&conn, paper.id).expect("delete nonexistent paper");
    assert!(
        !delete_again,
        "delete should return false for already deleted paper"
    );

    // Update nonexistent paper returns NotFound
    let update_err = PaperRepo::update(&conn, &paper);
    assert!(matches!(update_err, Err(RepoError::NotFound { .. })));
}

/// RT-06: CRUD for Collection, rename, delete does not delete papers.
#[test]
fn test_rt_06_collection_crud_and_safety() {
    let conn = open_in_memory().expect("open in memory db");

    let paper = sample_paper("safe_paper");
    PaperRepo::insert(&conn, &paper).expect("insert paper");

    let collection_id = Uuid::now_v7();
    let collection = Collection {
        id: collection_id,
        name: "Machine Learning".to_string(),
        parent_id: None,
    };

    // Insert collection
    CollectionRepo::insert(&conn, &collection).expect("insert collection");

    // Get by ID & name
    let found_col = CollectionRepo::get_by_id(&conn, collection_id)
        .expect("get collection by id")
        .expect("collection exists");
    assert_eq!(found_col.name, "Machine Learning");

    let found_by_name = CollectionRepo::get_by_name(&conn, "Machine Learning")
        .expect("get collection by name")
        .expect("collection exists");
    assert_eq!(found_by_name.id, collection_id);

    // Rename collection
    let renamed =
        CollectionRepo::rename(&conn, collection_id, "AI & ML").expect("rename collection");
    assert!(renamed);

    let updated_col = CollectionRepo::get_by_id(&conn, collection_id)
        .expect("get updated collection")
        .expect("exists");
    assert_eq!(updated_col.name, "AI & ML");

    // Add paper to collection
    CollectionRepo::add_paper(&conn, paper.id, collection_id).expect("add paper to collection");
    let col_papers = CollectionRepo::get_papers(&conn, collection_id).expect("get papers");
    assert_eq!(col_papers.len(), 1);
    assert_eq!(col_papers[0].id, paper.id);

    // Delete collection
    let deleted_col = CollectionRepo::delete(&conn, collection_id).expect("delete collection");
    assert!(deleted_col);

    let col_after =
        CollectionRepo::get_by_id(&conn, collection_id).expect("get deleted collection");
    assert!(col_after.is_none());

    // CRITICAL: Paper itself must NOT be deleted!
    let paper_still_exists =
        PaperRepo::get_by_id(&conn, paper.id).expect("query paper after collection delete");
    assert!(
        paper_still_exists.is_some(),
        "Paper must NOT be deleted when its collection is deleted"
    );
}

/// RT-07: Paper in multiple collections via paper_collections; remove_paper does not affect paper or other collections.
#[test]
fn test_rt_07_paper_in_multiple_collections() {
    let conn = open_in_memory().expect("open in memory db");

    let paper = sample_paper("multi_col");
    PaperRepo::insert(&conn, &paper).expect("insert paper");

    let c1 = Collection {
        id: Uuid::now_v7(),
        name: "Collection 1".to_string(),
        parent_id: None,
    };
    let c2 = Collection {
        id: Uuid::now_v7(),
        name: "Collection 2".to_string(),
        parent_id: None,
    };
    let c3 = Collection {
        id: Uuid::now_v7(),
        name: "Collection 3".to_string(),
        parent_id: None,
    };

    CollectionRepo::insert(&conn, &c1).expect("insert c1");
    CollectionRepo::insert(&conn, &c2).expect("insert c2");
    CollectionRepo::insert(&conn, &c3).expect("insert c3");

    // Add paper to all 3 collections
    CollectionRepo::add_paper(&conn, paper.id, c1.id).expect("add to c1");
    CollectionRepo::add_paper(&conn, paper.id, c2.id).expect("add to c2");
    CollectionRepo::add_paper(&conn, paper.id, c3.id).expect("add to c3");

    let cols = CollectionRepo::get_collections_for_paper(&conn, paper.id)
        .expect("get collections for paper");
    assert_eq!(cols.len(), 3);

    // Remove paper from c2 only
    let removed = CollectionRepo::remove_paper(&conn, paper.id, c2.id).expect("remove from c2");
    assert!(removed);

    // c2 should no longer contain paper
    let c2_papers = CollectionRepo::get_papers(&conn, c2.id).expect("get c2 papers");
    assert!(c2_papers.is_empty());

    // c1 and c3 must still contain paper
    let c1_papers = CollectionRepo::get_papers(&conn, c1.id).expect("get c1 papers");
    assert_eq!(c1_papers.len(), 1);
    assert_eq!(c1_papers[0].id, paper.id);

    let c3_papers = CollectionRepo::get_papers(&conn, c3.id).expect("get c3 papers");
    assert_eq!(c3_papers.len(), 1);
    assert_eq!(c3_papers[0].id, paper.id);

    // Paper itself must still exist
    assert!(PaperRepo::get_by_id(&conn, paper.id)
        .expect("get paper")
        .is_some());
}

/// RT-08: PaperRepo::delete cascades to paper_collections and toc_entries via SQLite FOREIGN KEY ... ON DELETE CASCADE.
#[test]
fn test_rt_08_cascading_deletes() {
    let conn = open_in_memory().expect("open in memory db");

    let paper = sample_paper("cascade_paper");
    PaperRepo::insert(&conn, &paper).expect("insert paper");

    let col = Collection {
        id: Uuid::now_v7(),
        name: "Cascade Col".to_string(),
        parent_id: None,
    };
    CollectionRepo::insert(&conn, &col).expect("insert col");
    CollectionRepo::add_paper(&conn, paper.id, col.id).expect("add to col");

    let root_toc_id = Uuid::now_v7();
    let child_toc_id = Uuid::now_v7();

    let toc_entries = vec![
        TocEntry {
            id: root_toc_id,
            paper_id: paper.id,
            parent_id: None,
            title: "Chapter 1".to_string(),
            page_number: 1,
            order_index: 0,
            source: TocSource::Auto,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        },
        TocEntry {
            id: child_toc_id,
            paper_id: paper.id,
            parent_id: Some(root_toc_id),
            title: "Section 1.1".to_string(),
            page_number: 2,
            order_index: 0,
            source: TocSource::Manual,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        },
    ];

    TocRepo::insert_batch(&conn, &toc_entries).expect("insert batch toc entries");

    let existing_toc = TocRepo::get_by_paper(&conn, paper.id).expect("get toc");
    assert_eq!(existing_toc.len(), 2);

    // Delete the paper
    let deleted = PaperRepo::delete(&conn, paper.id).expect("delete paper");
    assert!(deleted);

    // 1. paper_collections association must be automatically deleted by SQLite cascade
    let col_papers = CollectionRepo::get_papers(&conn, col.id).expect("get col papers");
    assert!(
        col_papers.is_empty(),
        "Association must be cascaded on paper delete"
    );

    // 2. toc_entries must be automatically deleted by SQLite cascade
    let toc_after = TocRepo::get_by_paper(&conn, paper.id).expect("get toc after delete");
    assert!(
        toc_after.is_empty(),
        "TOC entries must be cascaded on paper delete"
    );

    // 3. Collection itself must still exist
    assert!(CollectionRepo::get_by_id(&conn, col.id)
        .expect("get col")
        .is_some());

    // 4. Test TOC subtree cascade (deleting parent TOC entry deletes children)
    let paper2 = sample_paper("toc_tree_paper");
    PaperRepo::insert(&conn, &paper2).expect("insert paper2");

    let root_id = Uuid::now_v7();
    let child1_id = Uuid::now_v7();
    let child2_id = Uuid::now_v7();

    TocRepo::insert(
        &conn,
        &TocEntry {
            id: root_id,
            paper_id: paper2.id,
            parent_id: None,
            title: "Root".to_string(),
            page_number: 1,
            order_index: 0,
            source: TocSource::Auto,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        },
    )
    .expect("insert root");

    TocRepo::insert(
        &conn,
        &TocEntry {
            id: child1_id,
            paper_id: paper2.id,
            parent_id: Some(root_id),
            title: "Child 1".to_string(),
            page_number: 2,
            order_index: 0,
            source: TocSource::Manual,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        },
    )
    .expect("insert child 1");

    TocRepo::insert(
        &conn,
        &TocEntry {
            id: child2_id,
            paper_id: paper2.id,
            parent_id: Some(child1_id),
            title: "Child 2".to_string(),
            page_number: 3,
            order_index: 0,
            source: TocSource::Imported,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        },
    )
    .expect("insert child 2");

    assert_eq!(
        TocRepo::get_by_paper(&conn, paper2.id)
            .expect("get toc")
            .len(),
        3
    );

    // Deleting root should cascade to child1 and child2
    let del_root = TocRepo::delete(&conn, root_id).expect("delete root toc");
    assert!(del_root);

    let toc_remaining = TocRepo::get_by_paper(&conn, paper2.id).expect("get toc");
    assert!(
        toc_remaining.is_empty(),
        "All descendant TOC entries should be cascade-deleted"
    );
}

/// RT-08a: Duplicate insert on file_path or content_hash rejected by SQLite UNIQUE constraint.
#[test]
fn test_rt_08a_unique_constraint_enforcement() {
    let conn = open_in_memory().expect("open in memory db");

    let p1 = Paper {
        id: Uuid::now_v7(),
        file_path: "/papers/unique_test.pdf".to_string(),
        content_hash: "sha256_unique_hash_12345".to_string(),
        title: Some("Paper 1".to_string()),
        authors: None,
        year: None,
        journal: None,
        doi: None,
        abstract_text: None,
        text_path: None,
        annotated_pdf_path: None,
        toc_embedded_at: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };

    PaperRepo::insert(&conn, &p1).expect("insert p1");

    // 1. Duplicate file_path must be rejected by SQLite UNIQUE constraint
    let mut p_dup_path = p1.clone();
    p_dup_path.id = Uuid::now_v7();
    p_dup_path.content_hash = "different_hash_67890".to_string();

    let err_path = PaperRepo::insert(&conn, &p_dup_path);
    assert!(
        err_path.is_err(),
        "Duplicate file_path must trigger UNIQUE violation"
    );
    match err_path.unwrap_err() {
        RepoError::UniqueViolation { field, .. } => {
            assert!(
                field.contains("file_path"),
                "Unique violation must name file_path, got '{field}'"
            );
        }
        other => panic!("Expected UniqueViolation, got {:?}", other),
    }

    // 2. Duplicate content_hash must be rejected by SQLite UNIQUE constraint
    let mut p_dup_hash = p1.clone();
    p_dup_hash.id = Uuid::now_v7();
    p_dup_hash.file_path = "/papers/different_path.pdf".to_string();

    let err_hash = PaperRepo::insert(&conn, &p_dup_hash);
    assert!(
        err_hash.is_err(),
        "Duplicate content_hash must trigger UNIQUE violation"
    );
    match err_hash.unwrap_err() {
        RepoError::UniqueViolation { field, .. } => {
            assert!(
                field.contains("content_hash"),
                "Unique violation must name content_hash, got '{field}'"
            );
        }
        other => panic!("Expected UniqueViolation, got {:?}", other),
    }

    // 3. Duplicate collection name must be rejected
    let col1 = Collection {
        id: Uuid::now_v7(),
        name: "Duplicate Name".to_string(),
        parent_id: None,
    };
    CollectionRepo::insert(&conn, &col1).expect("insert col1");

    let col2 = Collection {
        id: Uuid::now_v7(),
        name: "Duplicate Name".to_string(),
        parent_id: None,
    };
    let err_col = CollectionRepo::insert(&conn, &col2);
    assert!(
        err_col.is_err(),
        "Duplicate collection name must trigger UNIQUE violation"
    );
    assert!(matches!(
        err_col.unwrap_err(),
        RepoError::UniqueViolation { .. }
    ));
}

/// Tests TocRepo::get_by_id, TocRepo::update, and TocRepo::delete_by_paper.
#[test]
fn test_toc_repo_additional_operations() {
    let conn = open_in_memory().expect("open in memory db");
    let paper = sample_paper("toc_extra_ops");
    PaperRepo::insert(&conn, &paper).expect("insert paper");

    let entry1 = TocEntry {
        id: Uuid::now_v7(),
        paper_id: paper.id,
        parent_id: None,
        title: "Introduction".to_string(),
        page_number: 1,
        order_index: 0,
        source: TocSource::Auto,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };
    let entry2 = TocEntry {
        id: Uuid::now_v7(),
        paper_id: paper.id,
        parent_id: Some(entry1.id),
        title: "Background".to_string(),
        page_number: 3,
        order_index: 0,
        source: TocSource::Manual,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };

    TocRepo::insert(&conn, &entry1).expect("insert entry1");
    TocRepo::insert(&conn, &entry2).expect("insert entry2");

    // 1. TocRepo::get_by_id
    let fetched1 = TocRepo::get_by_id(&conn, entry1.id)
        .expect("get_by_id entry1")
        .expect("entry1 exists");
    assert_eq!(fetched1.title, "Introduction");
    assert_eq!(fetched1.page_number, 1);
    assert_eq!(fetched1.source, TocSource::Auto);

    let fetched_none = TocRepo::get_by_id(&conn, Uuid::now_v7()).expect("get_by_id nonexistent");
    assert!(fetched_none.is_none());

    // 2. TocRepo::update
    let mut updated_entry1 = fetched1;
    updated_entry1.title = "Introduction (Revised)".to_string();
    updated_entry1.page_number = 2;
    updated_entry1.order_index = 1;
    updated_entry1.source = TocSource::Imported;
    updated_entry1.updated_at = "2024-02-01T00:00:00Z".to_string();

    TocRepo::update(&conn, &updated_entry1).expect("update entry1");

    let re_fetched = TocRepo::get_by_id(&conn, entry1.id)
        .expect("get_by_id after update")
        .expect("entry1 exists");
    assert_eq!(re_fetched.title, "Introduction (Revised)");
    assert_eq!(re_fetched.page_number, 2);
    assert_eq!(re_fetched.order_index, 1);
    assert_eq!(re_fetched.source, TocSource::Imported);
    assert_eq!(re_fetched.updated_at, "2024-02-01T00:00:00Z");

    // Updating a non-existent entry returns NotFound
    let non_existent = TocEntry {
        id: Uuid::now_v7(),
        paper_id: paper.id,
        parent_id: None,
        title: "Ghost".to_string(),
        page_number: 99,
        order_index: 99,
        source: TocSource::Manual,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };
    let update_err = TocRepo::update(&conn, &non_existent);
    assert!(matches!(update_err, Err(RepoError::NotFound { .. })));

    // 3. TocRepo::delete_by_paper
    let deleted_count = TocRepo::delete_by_paper(&conn, paper.id).expect("delete_by_paper");
    assert!(deleted_count > 0, "Should delete entries for paper");

    let remaining = TocRepo::get_by_paper(&conn, paper.id).expect("get_by_paper after delete");
    assert!(
        remaining.is_empty(),
        "All entries for paper must be deleted"
    );

    let deleted_again = TocRepo::delete_by_paper(&conn, paper.id).expect("delete_by_paper again");
    assert_eq!(deleted_again, 0, "No entries left to delete");

    // Also verify multi-root delete count
    let paper2 = sample_paper("toc_two_roots");
    PaperRepo::insert(&conn, &paper2).expect("insert paper2");
    let e_a = TocEntry {
        id: Uuid::now_v7(),
        paper_id: paper2.id,
        parent_id: None,
        title: "A".to_string(),
        page_number: 1,
        order_index: 0,
        source: TocSource::Auto,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };
    let e_b = TocEntry {
        id: Uuid::now_v7(),
        paper_id: paper2.id,
        parent_id: None,
        title: "B".to_string(),
        page_number: 2,
        order_index: 1,
        source: TocSource::Auto,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };
    TocRepo::insert(&conn, &e_a).expect("insert e_a");
    TocRepo::insert(&conn, &e_b).expect("insert e_b");
    assert_eq!(
        TocRepo::delete_by_paper(&conn, paper2.id).expect("delete roots"),
        2,
        "Directly deletes 2 root entries"
    );
}

/// Tests CollectionRepo::list ordering.
#[test]
fn test_collection_repo_list() {
    let conn = open_in_memory().expect("open in memory db");

    let col_c = Collection {
        id: Uuid::now_v7(),
        name: "Computer Science".to_string(),
        parent_id: None,
    };
    let col_a = Collection {
        id: Uuid::now_v7(),
        name: "Artificial Intelligence".to_string(),
        parent_id: None,
    };
    let col_b = Collection {
        id: Uuid::now_v7(),
        name: "Bioinformatics".to_string(),
        parent_id: None,
    };

    CollectionRepo::insert(&conn, &col_c).expect("insert C");
    CollectionRepo::insert(&conn, &col_a).expect("insert A");
    CollectionRepo::insert(&conn, &col_b).expect("insert B");

    let list = CollectionRepo::list(&conn).expect("list collections");
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].name, "Artificial Intelligence");
    assert_eq!(list[1].name, "Bioinformatics");
    assert_eq!(list[2].name, "Computer Science");
}

/// Tests invalid foreign key operations return RepoError::ForeignKeyViolation.
#[test]
fn test_foreign_key_violations() {
    let conn = open_in_memory().expect("open in memory db");

    let non_existent_paper_id = Uuid::now_v7();
    let non_existent_col_id = Uuid::now_v7();

    // 1. paper_collections with non-existent paper_id
    let col = Collection {
        id: Uuid::now_v7(),
        name: "Valid Col".to_string(),
        parent_id: None,
    };
    CollectionRepo::insert(&conn, &col).expect("insert valid collection");

    let err_pc = CollectionRepo::add_paper(&conn, non_existent_paper_id, col.id);
    assert!(
        err_pc.is_err(),
        "Adding non-existent paper to collection must violate foreign key"
    );
    match err_pc.unwrap_err() {
        RepoError::ForeignKeyViolation { message } => {
            assert!(
                message.contains("FOREIGN KEY") || message.is_empty(),
                "Expected foreign key message, got: {message}"
            );
        }
        other => panic!("Expected ForeignKeyViolation, got {:?}", other),
    }

    // 2. paper_collections with non-existent collection_id
    let paper = sample_paper("fk_paper");
    PaperRepo::insert(&conn, &paper).expect("insert paper");

    let err_pc2 = CollectionRepo::add_paper(&conn, paper.id, non_existent_col_id);
    assert!(matches!(
        err_pc2.unwrap_err(),
        RepoError::ForeignKeyViolation { .. }
    ));

    // 3. toc_entries with non-existent paper_id
    let invalid_toc = TocEntry {
        id: Uuid::now_v7(),
        paper_id: non_existent_paper_id,
        parent_id: None,
        title: "Orphaned TOC".to_string(),
        page_number: 1,
        order_index: 0,
        source: TocSource::Auto,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };
    let err_toc = TocRepo::insert(&conn, &invalid_toc);
    assert!(matches!(
        err_toc.unwrap_err(),
        RepoError::ForeignKeyViolation { .. }
    ));

    // 4. collections with non-existent parent_id
    let invalid_child_col = Collection {
        id: Uuid::now_v7(),
        name: "Invalid Child".to_string(),
        parent_id: Some(non_existent_col_id),
    };
    let err_col_parent = CollectionRepo::insert(&conn, &invalid_child_col);
    assert!(matches!(
        err_col_parent.unwrap_err(),
        RepoError::ForeignKeyViolation { .. }
    ));
}

/// Tests ON DELETE SET NULL for nested collection parent_id.
#[test]
fn test_collection_parent_delete_set_null() {
    let conn = open_in_memory().expect("open in memory db");

    let parent_id = Uuid::now_v7();
    let parent_col = Collection {
        id: parent_id,
        name: "Parent Category".to_string(),
        parent_id: None,
    };
    CollectionRepo::insert(&conn, &parent_col).expect("insert parent collection");

    let child_id = Uuid::now_v7();
    let child_col = Collection {
        id: child_id,
        name: "Child Subcategory".to_string(),
        parent_id: Some(parent_id),
    };
    CollectionRepo::insert(&conn, &child_col).expect("insert child collection");

    // Verify child has parent_id
    let fetched_child = CollectionRepo::get_by_id(&conn, child_id)
        .expect("get child")
        .expect("child exists");
    assert_eq!(fetched_child.parent_id, Some(parent_id));

    // Delete parent
    let deleted = CollectionRepo::delete(&conn, parent_id).expect("delete parent");
    assert!(deleted);

    // Parent is gone
    assert!(CollectionRepo::get_by_id(&conn, parent_id)
        .expect("get parent")
        .is_none());

    // Child still exists and parent_id became None via SQLite ON DELETE SET NULL
    let child_after = CollectionRepo::get_by_id(&conn, child_id)
        .expect("get child after parent deletion")
        .expect("child must still exist");
    assert_eq!(
        child_after.parent_id, None,
        "Child parent_id must become None after parent is deleted (ON DELETE SET NULL)"
    );
    assert_eq!(child_after.name, "Child Subcategory");
}
