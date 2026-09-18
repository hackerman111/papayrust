use thiserror::Error;

/// Repository layer errors with fine-grained constraint and query failure classification.
#[derive(Debug, Error)]
pub enum RepoError {
    #[error("unique constraint violation on '{field}': {message}")]
    UniqueViolation { field: String, message: String },

    #[error("foreign key constraint violation: {message}")]
    ForeignKeyViolation { message: String },

    #[error("{entity} not found with id '{id}'")]
    NotFound { entity: &'static str, id: String },

    #[error("sqlite error: {0}")]
    Sqlite(rusqlite::Error),

    #[error("invalid data: {0}")]
    InvalidData(String),
}

impl From<rusqlite::Error> for RepoError {
    fn from(err: rusqlite::Error) -> Self {
        RepoError::from_sqlite(err)
    }
}

impl RepoError {
    /// Converts a rusqlite error into a structured `RepoError`, inspecting SQLite
    /// extended result codes and constraint messages.
    pub fn from_sqlite(err: rusqlite::Error) -> Self {
        match &err {
            rusqlite::Error::SqliteFailure(ffi_err, msg) => {
                let msg_str = msg.as_deref().unwrap_or("");
                // SQLite extended error codes:
                // 2067: SQLITE_CONSTRAINT_UNIQUE
                // 1555: SQLITE_CONSTRAINT_PRIMARYKEY
                // 787:  SQLITE_CONSTRAINT_FOREIGNKEY
                if ffi_err.extended_code == 2067
                    || ffi_err.extended_code == 1555
                    || msg_str.contains("UNIQUE constraint failed")
                    || msg_str.contains("PRIMARY KEY")
                {
                    let field = if let Some(idx) = msg_str.find("UNIQUE constraint failed: ") {
                        msg_str[idx + "UNIQUE constraint failed: ".len()..]
                            .trim()
                            .to_string()
                    } else if msg_str.contains("PRIMARY KEY") {
                        "PRIMARY KEY".to_string()
                    } else {
                        "unique_constraint".to_string()
                    };
                    RepoError::UniqueViolation {
                        field,
                        message: msg_str.to_string(),
                    }
                } else if ffi_err.extended_code == 787
                    || msg_str.contains("FOREIGN KEY constraint failed")
                {
                    RepoError::ForeignKeyViolation {
                        message: msg_str.to_string(),
                    }
                } else {
                    RepoError::Sqlite(err)
                }
            }
            _ => RepoError::Sqlite(err),
        }
    }
}
