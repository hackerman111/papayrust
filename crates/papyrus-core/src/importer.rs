use lopdf::Document;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

pub mod json_metadata;
pub use json_metadata::import_metadata_from_path;

use crate::config::Config;
use crate::db::collection_repo::CollectionRepo;
use crate::db::error::RepoError;
use crate::db::models::Paper;
use crate::db::paper_repo::PaperRepo;
use crate::db::toc_repo::TocRepo;
use crate::pdf::metadata::extract_metadata;
use crate::pdf::outline::extract_outlines;
use crate::time::current_timestamp_utc;

/// Errors that can occur during the PDF import pipeline.
#[derive(Debug, Error)]
pub enum ImportError {
    #[error("duplicate paper: existing_id={existing_id}, content_hash={content_hash}, file_path={file_path}")]
    Duplicate {
        existing_id: Uuid,
        content_hash: String,
        file_path: String,
    },

    #[error("invalid or corrupt PDF at {path}: {reason}")]
    InvalidPdf { path: PathBuf, reason: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("database error: {0}")]
    Db(#[from] RepoError),
}

impl From<rusqlite::Error> for ImportError {
    fn from(err: rusqlite::Error) -> Self {
        ImportError::Db(RepoError::from_sqlite(err))
    }
}

/// Pipeline for importing a PDF file into the library and database.
pub struct Importer;

impl Importer {
    /// Imports a PDF into the database using the supplied configuration.
    pub fn import(
        conn: &mut Connection,
        config: &Config,
        file_path: &Path,
        collection_id: Option<Uuid>,
    ) -> Result<Paper, ImportError> {
        import_paper(conn, config, file_path, collection_id)
    }
}

/// Imports a PDF into the database:
/// 1. Reads file bytes and computes SHA-256 `content_hash`.
/// 2. Checks for deduplication by `content_hash`. If found, returns `Err(ImportError::Duplicate)`.
/// 3. Parses PDF via `lopdf`. If invalid/corrupt, returns `Err(ImportError::InvalidPdf)`.
/// 4. Extracts metadata (title, authors, year, doi, journal).
/// 5. If `config.toc.auto_extract_on_import` is true, extracts `/Outlines`.
/// 6. Executes atomic immediate SQLite transaction:
///    - Saves `Paper` via `PaperRepo::insert`.
///    - If outlines exist, saves them via `TocRepo::insert_batch`.
///    - If `collection_id` is provided, links paper via `CollectionRepo::add_paper`.
///    - Commits transaction.
/// 7. Returns the newly created `Paper`.
pub fn import_paper(
    conn: &mut Connection,
    config: &Config,
    file_path: &Path,
    collection_id: Option<Uuid>,
) -> Result<Paper, ImportError> {
    // 1. Read file bytes and compute SHA-256
    let bytes = std::fs::read(file_path)?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let content_hash = format!("{:x}", hasher.finalize());

    // 2. Check for duplicate by content_hash
    if let Some(existing) = PaperRepo::get_by_content_hash(conn, &content_hash)? {
        return Err(ImportError::Duplicate {
            existing_id: existing.id,
            content_hash: existing.content_hash,
            file_path: existing.file_path,
        });
    }

    // 3. Load and parse PDF
    let doc = Document::load_mem(&bytes).map_err(|err| ImportError::InvalidPdf {
        path: file_path.to_path_buf(),
        reason: err.to_string(),
    })?;

    // 4. Extract metadata
    let metadata = extract_metadata(&doc, file_path);

    // 5. Generate identifiers and timestamps
    let paper_id = Uuid::now_v7();
    let now = current_timestamp_utc();

    // 6. Extract outlines if enabled in config
    let toc_entries = if config.toc.auto_extract_on_import {
        match extract_outlines(&doc, paper_id) {
            Ok(entries) => entries,
            Err(err) => {
                tracing::warn!(
                    paper_id = %paper_id,
                    error = %err,
                    "Failed to extract outlines during import"
                );
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let paper = Paper {
        id: paper_id,
        file_path: file_path.to_string_lossy().to_string(),
        content_hash,
        title: Some(metadata.title),
        authors: metadata.authors,
        year: metadata.year,
        journal: metadata.journal,
        doi: metadata.doi,
        abstract_text: None,
        text_path: None,
        annotated_pdf_path: None,
        toc_embedded_at: None,
        created_at: now.clone(),
        updated_at: now,
    };

    // 7. Atomic transaction
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

    PaperRepo::insert(&tx, &paper)?;

    if !toc_entries.is_empty() {
        TocRepo::insert_batch(&tx, &toc_entries)?;
    }

    if let Some(col_id) = collection_id {
        CollectionRepo::add_paper(&tx, paper.id, col_id)?;
    }

    tx.commit()?;

    Ok(paper)
}
