use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::error::RepoError;

/// A pending TOC entry before insertion/import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingTocEntry {
    pub title: String,
    pub page: u32,
    pub level: usize,
}

impl PendingTocEntry {
    pub fn new(title: impl Into<String>, page: u32, level: usize) -> Self {
        Self {
            title: title.into(),
            page,
            level,
        }
    }
}

/// Errors occurring during TOC operations, imports, or materialization.
#[derive(Debug, thiserror::Error)]
pub enum TocError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("TOC entry not found: {0}")]
    NotFound(Uuid),

    #[error("Paper not found: {0}")]
    PaperNotFound(Uuid),

    #[error("Database error: {0}")]
    Db(#[from] RepoError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("PDF error: {0}")]
    Pdf(String),

    #[error("TOC Import error: {0}")]
    Import(#[from] TocImportError),
}

impl From<rusqlite::Error> for TocError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Db(RepoError::from(err))
    }
}

/// Errors during TOC text or JSON importing.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TocImportError {
    #[error("Line {line_no}: {reason}")]
    Line { line_no: usize, reason: String },

    #[error("Empty outline")]
    EmptyOutline,

    #[error("Page number {page} exceeds total page count {page_count}")]
    PageOutOfBounds { page: u32, page_count: u32 },

    #[error("IO error: {0}")]
    Io(String),

    #[error("JSON error: {0}")]
    Json(String),

    #[error("PDF error: {0}")]
    Pdf(String),
}
