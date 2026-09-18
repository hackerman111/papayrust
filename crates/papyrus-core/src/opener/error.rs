use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when resolving, preparing, or launching an external viewer.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OpenerError {
    #[error("file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("invalid command template: {0}")]
    InvalidTemplate(String),

    #[error("failed to launch viewer: {0}")]
    LaunchFailed(String),

    #[error("invalid page number: {0}")]
    InvalidPage(u32),
}
