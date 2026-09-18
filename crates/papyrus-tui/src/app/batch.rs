use papyrus_core::db::{CollectionRepo, PaperRepo, RepoError, TagRepo};
use papyrus_core::SearchIndex;
use rusqlite::Connection;
use uuid::Uuid;

/// Batch associates/dissociates a list of papers with collections in a single SQLite transaction.
pub fn batch_set_collections(
    conn: &mut Connection,
    paper_ids: &[Uuid],
    add_cols: &[Uuid],
    remove_cols: &[Uuid],
) -> Result<(), RepoError> {
    let tx = conn.transaction().map_err(RepoError::from_sqlite)?;
    for &paper_id in paper_ids {
        for &col_id in add_cols {
            CollectionRepo::add_paper(&tx, paper_id, col_id)?;
        }
        for &col_id in remove_cols {
            CollectionRepo::remove_paper(&tx, paper_id, col_id)?;
        }
    }
    tx.commit().map_err(RepoError::from_sqlite)?;
    Ok(())
}

/// Batch adds/removes tags across a list of papers in a single SQLite transaction.
pub fn batch_set_tags(
    conn: &mut Connection,
    paper_ids: &[Uuid],
    add_tags: &[String],
    remove_tags: &[String],
) -> Result<(), RepoError> {
    let tx = conn.transaction().map_err(RepoError::from_sqlite)?;
    for &paper_id in paper_ids {
        for tag in add_tags {
            TagRepo::add_tag(&tx, paper_id, tag)?;
        }
        for tag in remove_tags {
            TagRepo::remove_tag(&tx, paper_id, tag)?;
        }
    }
    tx.commit().map_err(RepoError::from_sqlite)?;
    Ok(())
}

/// Batch deletes a list of papers from SQLite in a single transaction and removes them from Tantivy index.
pub fn batch_delete_papers(
    conn: &mut Connection,
    paper_ids: &[Uuid],
    search_index: Option<&SearchIndex>,
) -> Result<usize, RepoError> {
    let tx = conn.transaction().map_err(RepoError::from_sqlite)?;
    let mut deleted_count = 0;
    for &paper_id in paper_ids {
        if PaperRepo::delete(&tx, paper_id)? {
            deleted_count += 1;
        }
    }
    tx.commit().map_err(RepoError::from_sqlite)?;

    if let Some(index) = search_index {
        for &paper_id in paper_ids {
            let _ = index.remove_paper(paper_id);
        }
    }
    Ok(deleted_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use papyrus_core::db::{open_in_memory, Collection, Paper};

    fn create_test_paper(conn: &Connection, title: &str) -> Paper {
        let paper = Paper {
            id: Uuid::now_v7(),
            file_path: format!("/path/to/{title}.pdf"),
            content_hash: format!("hash_{title}_{}", Uuid::now_v7()),
            title: Some(title.to_string()),
            authors: Some("Author".to_string()),
            year: Some(2024),
            journal: None,
            doi: None,
            abstract_text: None,
            text_path: None,
            annotated_pdf_path: None,
            toc_embedded_at: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        PaperRepo::insert(conn, &paper).unwrap();
        paper
    }

    fn create_test_collection(conn: &Connection, name: &str) -> Collection {
        let col = Collection {
            id: Uuid::now_v7(),
            name: name.to_string(),
            parent_id: None,
        };
        CollectionRepo::insert(conn, &col).unwrap();
        col
    }

    #[test]
    fn test_batch_set_collections() {
        let mut conn = open_in_memory().unwrap();
        let p1 = create_test_paper(&conn, "Paper 1");
        let p2 = create_test_paper(&conn, "Paper 2");
        let c1 = create_test_collection(&conn, "Col 1");
        let c2 = create_test_collection(&conn, "Col 2");
        let c3 = create_test_collection(&conn, "Col 3");

        // Initially associate p1 and p2 with c1
        CollectionRepo::add_paper(&conn, p1.id, c1.id).unwrap();
        CollectionRepo::add_paper(&conn, p2.id, c1.id).unwrap();

        // Batch associate p1 and p2 with c2 and c3, while removing from c1
        let paper_ids = vec![p1.id, p2.id];
        let add_cols = vec![c2.id, c3.id];
        let remove_cols = vec![c1.id];

        batch_set_collections(&mut conn, &paper_ids, &add_cols, &remove_cols).unwrap();

        // Verify p1 collections: now in c2 and c3, not c1
        let p1_cols = CollectionRepo::get_collections_for_paper(&conn, p1.id).unwrap();
        let p1_col_ids: Vec<Uuid> = p1_cols.into_iter().map(|c| c.id).collect();
        assert!(!p1_col_ids.contains(&c1.id));
        assert!(p1_col_ids.contains(&c2.id));
        assert!(p1_col_ids.contains(&c3.id));

        // Verify p2 collections: now in c2 and c3, not c1
        let p2_cols = CollectionRepo::get_collections_for_paper(&conn, p2.id).unwrap();
        let p2_col_ids: Vec<Uuid> = p2_cols.into_iter().map(|c| c.id).collect();
        assert!(!p2_col_ids.contains(&c1.id));
        assert!(p2_col_ids.contains(&c2.id));
        assert!(p2_col_ids.contains(&c3.id));
    }

    #[test]
    fn test_batch_set_tags() {
        let mut conn = open_in_memory().unwrap();
        let p1 = create_test_paper(&conn, "Paper 1");
        let p2 = create_test_paper(&conn, "Paper 2");

        // Initially tag p1 and p2 with "initial"
        TagRepo::add_tag(&conn, p1.id, "initial").unwrap();
        TagRepo::add_tag(&conn, p2.id, "initial").unwrap();

        let paper_ids = vec![p1.id, p2.id];
        let add_tags = vec!["rust".to_string(), "tui".to_string()];
        let remove_tags = vec!["initial".to_string()];

        batch_set_tags(&mut conn, &paper_ids, &add_tags, &remove_tags).unwrap();

        let p1_tags = TagRepo::get_tags_for_paper(&conn, p1.id).unwrap();
        assert_eq!(p1_tags, vec!["rust", "tui"]);

        let p2_tags = TagRepo::get_tags_for_paper(&conn, p2.id).unwrap();
        assert_eq!(p2_tags, vec!["rust", "tui"]);
    }

    #[test]
    fn test_batch_delete_papers() {
        let mut conn = open_in_memory().unwrap();
        let p1 = create_test_paper(&conn, "Paper 1");
        let p2 = create_test_paper(&conn, "Paper 2");
        let p3 = create_test_paper(&conn, "Paper 3");

        let paper_ids = vec![p1.id, p2.id];
        let deleted = batch_delete_papers(&mut conn, &paper_ids, None).unwrap();
        assert_eq!(deleted, 2);

        assert!(PaperRepo::get_by_id(&conn, p1.id).unwrap().is_none());
        assert!(PaperRepo::get_by_id(&conn, p2.id).unwrap().is_none());
        assert!(PaperRepo::get_by_id(&conn, p3.id).unwrap().is_some());
    }

    #[test]
    fn test_batch_delete_papers_with_search_index() {
        let mut conn = open_in_memory().unwrap();
        let p1 = create_test_paper(&conn, "Paper Index 1");
        let search_index = SearchIndex::create_in_ram().unwrap();
        search_index
            .index_paper(&p1, Some("Paper body text"))
            .unwrap();

        let hits = search_index.search("Index", 10).unwrap();
        assert_eq!(hits.len(), 1);

        let index_opt = Some(search_index);
        let deleted = batch_delete_papers(&mut conn, &[p1.id], index_opt.as_ref()).unwrap();
        assert_eq!(deleted, 1);

        assert!(PaperRepo::get_by_id(&conn, p1.id).unwrap().is_none());
        if let Some(ref idx) = index_opt {
            let hits_after = idx.search("Index", 10).unwrap();
            assert_eq!(hits_after.len(), 0);
        }
    }
}
