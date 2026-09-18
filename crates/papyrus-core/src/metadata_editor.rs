use rusqlite::Connection;
use std::path::Path;
use uuid::Uuid;

use crate::db::error::RepoError;
use crate::db::models::Paper;
use crate::db::paper_repo::PaperRepo;
use crate::search::error::SearchError;
use crate::search::SearchIndex;
use crate::time::current_timestamp_utc;

/// Input payload for updating a paper's editable metadata.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdatePaperMetadata {
    pub title: String,
    pub authors: Option<String>,
    pub year: Option<i64>,
    pub journal: Option<String>,
    pub doi: Option<String>,
    pub abstract_text: Option<String>,
}

impl UpdatePaperMetadata {
    /// Creates a new update payload with the specified title and all other fields set to `None`.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            authors: None,
            year: None,
            journal: None,
            doi: None,
            abstract_text: None,
        }
    }
}

/// Errors that can occur while validating or saving paper metadata.
#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error("validation error: {0}")]
    Validation(String),

    #[error("paper not found with id: {0}")]
    NotFound(Uuid),

    #[error("database error: {0}")]
    Db(#[from] RepoError),

    #[error("search error: {0}")]
    Search(#[from] SearchError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<rusqlite::Error> for MetadataError {
    fn from(err: rusqlite::Error) -> Self {
        MetadataError::Db(RepoError::from_sqlite(err))
    }
}

/// Updates metadata for a research paper:
/// 1. Validates title (non-empty after trim) and year (1000..=3000 if Some).
/// 2. Retrieves existing paper from SQLite database by `paper_id`.
/// 3. Updates metadata fields and updates `updated_at` with current UTC timestamp.
/// 4. Saves changes to SQLite database via `PaperRepo::update`.
/// 5. If `search_index` is provided:
///    - If `paper.text_path` exists and points to a file, reads body text.
///    - Indexes paper in Tantivy search index.
/// 6. Returns the updated `Paper`.
pub fn update_metadata(
    conn: &mut Connection,
    paper_id: Uuid,
    update: UpdatePaperMetadata,
    search_index: Option<&SearchIndex>,
) -> Result<Paper, MetadataError> {
    let trimmed_title = update.title.trim();
    if trimmed_title.is_empty() {
        return Err(MetadataError::Validation(
            "Title cannot be empty".to_string(),
        ));
    }

    if let Some(year) = update.year {
        if !(1000..=3000).contains(&year) {
            return Err(MetadataError::Validation("Invalid year".to_string()));
        }
    }

    let mut paper = match PaperRepo::get_by_id(conn, paper_id)? {
        Some(p) => p,
        None => return Err(MetadataError::NotFound(paper_id)),
    };

    paper.title = Some(trimmed_title.to_string());
    paper.authors = update.authors;
    paper.year = update.year;
    paper.journal = update.journal;
    paper.doi = update.doi;
    paper.abstract_text = update.abstract_text;
    paper.updated_at = current_timestamp_utc();

    PaperRepo::update(conn, &paper)?;

    if let Some(index) = search_index {
        let body_text = match &paper.text_path {
            Some(ref path_str) => {
                let p = Path::new(path_str);
                if p.exists() {
                    Some(std::fs::read_to_string(p)?)
                } else {
                    None
                }
            }
            None => None,
        };
        index.index_paper(&paper, body_text.as_deref())?;
    }

    Ok(paper)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{apply_migrations, open_in_memory};

    fn setup_test_db() -> Connection {
        let mut conn = open_in_memory().expect("open memory db");
        apply_migrations(&mut conn).expect("apply migrations");
        conn
    }

    fn sample_paper(id: Uuid) -> Paper {
        Paper {
            id,
            file_path: "/papers/test.pdf".to_string(),
            content_hash: "hash123".to_string(),
            title: Some("Initial Title".to_string()),
            authors: Some("Initial Author".to_string()),
            year: Some(2020),
            journal: Some("Old Journal".to_string()),
            doi: Some("10.1000/1".to_string()),
            abstract_text: Some("Initial abstract".to_string()),
            text_path: None,
            annotated_pdf_path: None,
            toc_embedded_at: None,
            created_at: "2020-01-01T00:00:00Z".to_string(),
            updated_at: "2020-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_update_metadata_validation() {
        let mut conn = setup_test_db();
        let id = Uuid::now_v7();
        let paper = sample_paper(id);
        PaperRepo::insert(&conn, &paper).expect("insert paper");

        // Empty title
        let update_empty = UpdatePaperMetadata {
            title: "".to_string(),
            ..Default::default()
        };
        let err = update_metadata(&mut conn, id, update_empty, None).unwrap_err();
        assert!(
            matches!(err, MetadataError::Validation(ref msg) if msg == "Title cannot be empty")
        );

        // Whitespace-only title
        let update_ws = UpdatePaperMetadata {
            title: "   \n\t ".to_string(),
            ..Default::default()
        };
        let err = update_metadata(&mut conn, id, update_ws, None).unwrap_err();
        assert!(
            matches!(err, MetadataError::Validation(ref msg) if msg == "Title cannot be empty")
        );

        // Year too small
        let update_year_small = UpdatePaperMetadata {
            title: "Valid Title".to_string(),
            year: Some(999),
            ..Default::default()
        };
        let err = update_metadata(&mut conn, id, update_year_small, None).unwrap_err();
        assert!(matches!(err, MetadataError::Validation(ref msg) if msg == "Invalid year"));

        // Year too large
        let update_year_large = UpdatePaperMetadata {
            title: "Valid Title".to_string(),
            year: Some(3001),
            ..Default::default()
        };
        let err = update_metadata(&mut conn, id, update_year_large, None).unwrap_err();
        assert!(matches!(err, MetadataError::Validation(ref msg) if msg == "Invalid year"));

        // Not found
        let missing_id = Uuid::now_v7();
        let update_valid = UpdatePaperMetadata {
            title: "Valid Title".to_string(),
            year: Some(2024),
            ..Default::default()
        };
        let err = update_metadata(&mut conn, missing_id, update_valid, None).unwrap_err();
        assert!(matches!(err, MetadataError::NotFound(mid) if mid == missing_id));
    }

    #[test]
    fn test_update_metadata_success_and_search_index() {
        let mut conn = setup_test_db();
        let id = Uuid::now_v7();
        let paper = sample_paper(id);
        PaperRepo::insert(&conn, &paper).expect("insert paper");

        let search_index = SearchIndex::create_in_ram().expect("ram index");
        search_index
            .index_paper(&paper, Some("initial body text"))
            .expect("initial index");

        // Initial search matches old title
        let res = search_index.search("Initial", 10).expect("search");
        assert_eq!(res.len(), 1);

        let update = UpdatePaperMetadata {
            title: "Deep Learning Architectures".to_string(),
            authors: Some("LeCun, Bengio, Hinton".to_string()),
            year: Some(2021),
            journal: Some("Nature".to_string()),
            doi: Some("10.1038/nature14539".to_string()),
            abstract_text: Some("Deep convolutional and recurrent networks".to_string()),
        };

        let updated = update_metadata(&mut conn, id, update, Some(&search_index))
            .expect("update metadata must succeed");

        assert_eq!(
            updated.title.as_deref(),
            Some("Deep Learning Architectures")
        );
        assert_eq!(updated.authors.as_deref(), Some("LeCun, Bengio, Hinton"));
        assert_eq!(updated.year, Some(2021));
        assert_eq!(updated.journal.as_deref(), Some("Nature"));
        assert_eq!(updated.doi.as_deref(), Some("10.1038/nature14539"));
        assert_eq!(
            updated.abstract_text.as_deref(),
            Some("Deep convolutional and recurrent networks")
        );
        assert_ne!(updated.updated_at, "2020-01-01T00:00:00Z");
        assert_eq!(updated.created_at, "2020-01-01T00:00:00Z");
        assert_eq!(updated.file_path, "/papers/test.pdf");
        assert_eq!(updated.content_hash, "hash123");

        // Search index updated: matches new terms, does not match old terms
        let new_res = search_index.search("Architectures", 10).expect("search");
        assert_eq!(new_res.len(), 1);
        assert_eq!(new_res[0].id, id);

        let old_res = search_index.search("Initial", 10).expect("search");
        assert!(old_res.is_empty(), "Old title must no longer match");
    }
}
