use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

use crate::db::error::RepoError;

/// Errors that can occur during export operations.
#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Export destination already exists: {0}")]
    AlreadyExists(PathBuf),

    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    #[error("Paper not found with ID: {0}")]
    PaperNotFound(Uuid),

    #[error("Paper file not found for paper {id}: {path}")]
    PaperFileNotFound { id: Uuid, path: String },

    #[error("Path traversal detected: {0}")]
    PathTraversal(String),

    #[error("Invalid path in archive: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Db(#[from] RepoError),

    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("SQLite backup error: {0}")]
    Backup(String),
}

impl From<rusqlite::Error> for ExportError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Db(RepoError::from(err))
    }
}
