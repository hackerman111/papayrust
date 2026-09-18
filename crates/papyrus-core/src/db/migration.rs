use crate::db::connection::DbError;
use rusqlite::Connection;

const MIGRATIONS: &[(u32, &str)] = &[(1, SCHEMA_V1)];

const SCHEMA_V1: &str = r#"
CREATE TABLE papers (
    id                  TEXT PRIMARY KEY NOT NULL,
    file_path           TEXT NOT NULL UNIQUE,
    content_hash        TEXT NOT NULL UNIQUE,

    title               TEXT,
    authors             TEXT,
    year                INTEGER,
    journal             TEXT,
    doi                 TEXT,
    abstract_text       TEXT,

    text_path           TEXT,
    annotated_pdf_path  TEXT,
    toc_embedded_at     TEXT,

    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);

CREATE TABLE collections (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL UNIQUE,
    parent_id   TEXT,
    FOREIGN KEY (parent_id)
        REFERENCES collections(id)
        ON DELETE SET NULL
);

CREATE TABLE paper_collections (
    paper_id       TEXT NOT NULL,
    collection_id  TEXT NOT NULL,

    PRIMARY KEY (paper_id, collection_id),

    FOREIGN KEY (paper_id)
        REFERENCES papers(id)
        ON DELETE CASCADE,

    FOREIGN KEY (collection_id)
        REFERENCES collections(id)
        ON DELETE CASCADE
);

CREATE TABLE toc_entries (
    id            TEXT PRIMARY KEY NOT NULL,
    paper_id      TEXT NOT NULL,
    parent_id     TEXT,
    title         TEXT NOT NULL,
    page_number   INTEGER NOT NULL CHECK (page_number >= 1),
    order_index   INTEGER NOT NULL,
    source        TEXT NOT NULL CHECK (source IN ('auto', 'manual', 'imported')),
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,

    FOREIGN KEY (paper_id)
        REFERENCES papers(id)
        ON DELETE CASCADE,

    FOREIGN KEY (parent_id)
        REFERENCES toc_entries(id)
        ON DELETE CASCADE
);

CREATE INDEX idx_paper_collections_collection
    ON paper_collections(collection_id);

CREATE INDEX idx_toc_entries_paper
    ON toc_entries(paper_id);

CREATE INDEX idx_toc_entries_parent
    ON toc_entries(parent_id);

CREATE INDEX idx_toc_entries_siblings
    ON toc_entries(paper_id, parent_id, order_index);
"#;

/// Returns the highest schema version currently recorded in `schema_migrations`,
/// or `0` if no migrations have been applied yet.
pub fn current_schema_version(conn: &Connection) -> Result<u32, DbError> {
    let table_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_migrations')",
        [],
        |row| row.get(0),
    )?;

    if !table_exists {
        return Ok(0);
    }

    let version: Option<u32> =
        conn.query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })?;

    Ok(version.unwrap_or(0))
}

/// Applies all pending schema migrations inside atomic transactions.
///
/// This operation is idempotent; migrations that have already been recorded
/// in `schema_migrations` are skipped.
pub fn apply_migrations(conn: &mut Connection) -> Result<(), DbError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version     INTEGER PRIMARY KEY NOT NULL,
            applied_at  TEXT NOT NULL
        );",
    )?;

    for &(version, sql) in MIGRATIONS {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let is_applied: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [version],
            |row| row.get(0),
        )?;

        if !is_applied {
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
                [version],
            )?;
            tracing::info!("Applied database migration version {}", version);
        }
        tx.commit()?;
    }

    Ok(())
}
