use rusqlite::Connection;
use std::path::Path;
use thiserror::Error;

/// Database errors.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("database I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("migration error: {0}")]
    Migration(String),
}

/// Applies standard SQLite PRAGMAs to an open connection.
///
/// Configures:
/// - `foreign_keys = ON`
/// - `synchronous = NORMAL`
/// - `busy_timeout = 5000`
/// - `journal_mode = WAL` (for file-based databases; for in-memory databases, SQLite returns "memory", which is handled gracefully)
pub fn configure_connection(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;\nPRAGMA synchronous = NORMAL;\nPRAGMA busy_timeout = 5000;",
    )?;

    let journal_mode: String =
        conn.query_row("PRAGMA journal_mode = WAL;", [], |row| row.get(0))?;
    if journal_mode.eq_ignore_ascii_case("memory") {
        tracing::debug!("In-memory SQLite journal_mode is 'memory'");
    } else if !journal_mode.eq_ignore_ascii_case("wal") {
        tracing::warn!(
            "SQLite journal_mode was set to '{}', expected 'wal'",
            journal_mode
        );
    }

    Ok(())
}

/// Opens a SQLite database at the specified path, configures PRAGMAs, and applies migrations.
pub fn open_database(path: &Path) -> Result<Connection, DbError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|err| {
                std::io::Error::new(
                    err.kind(),
                    format!(
                        "failed to create database directory '{}': {err}",
                        parent.display()
                    ),
                )
            })?;
        }
    }

    let mut conn = Connection::open(path)?;
    configure_connection(&conn)?;
    crate::db::migration::apply_migrations(&mut conn)?;
    ensure_tag_tables(&conn)?;
    Ok(conn)
}

/// Opens an in-memory SQLite database, configures PRAGMAs, and applies migrations.
pub fn open_in_memory() -> Result<Connection, DbError> {
    let mut conn = Connection::open_in_memory()?;
    configure_connection(&conn)?;
    crate::db::migration::apply_migrations(&mut conn)?;
    ensure_tag_tables(&conn)?;
    Ok(conn)
}

/// Ensures that the `tags` and `paper_tags` tables and indices exist.
pub fn ensure_tag_tables(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tags (
            id    TEXT PRIMARY KEY NOT NULL,
            name  TEXT NOT NULL UNIQUE
        );
        CREATE TABLE IF NOT EXISTS paper_tags (
            paper_id  TEXT NOT NULL,
            tag_id    TEXT NOT NULL,
            PRIMARY KEY (paper_id, tag_id),
            FOREIGN KEY (paper_id) REFERENCES papers(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_paper_tags_tag ON paper_tags(tag_id);",
    )?;
    Ok(())
}
