use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension, Row};
use uuid::Uuid;

use crate::db::error::RepoError;
use crate::db::models::{TocEntry, TocSource};

/// Repository for table of contents entries.
pub struct TocRepo;

impl TocRepo {
    /// Inserts a single table of contents entry.
    pub fn insert(conn: &Connection, entry: &TocEntry) -> Result<(), RepoError> {
        conn.execute(
            "INSERT INTO toc_entries (
                id, paper_id, parent_id, title, page_number, order_index, source,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                entry.id.to_string(),
                entry.paper_id.to_string(),
                entry.parent_id.map(|u| u.to_string()),
                entry.title,
                entry.page_number,
                entry.order_index,
                entry.source.as_str(),
                entry.created_at,
                entry.updated_at,
            ],
        )
        .map_err(RepoError::from_sqlite)?;

        Ok(())
    }

    /// Inserts a batch of table of contents entries atomically within a transaction/savepoint.
    pub fn insert_batch(conn: &Connection, entries: &[TocEntry]) -> Result<(), RepoError> {
        if entries.is_empty() {
            return Ok(());
        }

        conn.execute_batch("SAVEPOINT toc_insert_batch;")
            .map_err(RepoError::from_sqlite)?;

        let run = || -> Result<(), RepoError> {
            let mut stmt = conn
                .prepare(
                    "INSERT INTO toc_entries (
                        id, paper_id, parent_id, title, page_number, order_index, source,
                        created_at, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                )
                .map_err(RepoError::from_sqlite)?;

            for entry in entries {
                stmt.execute(params![
                    entry.id.to_string(),
                    entry.paper_id.to_string(),
                    entry.parent_id.map(|u| u.to_string()),
                    entry.title,
                    entry.page_number,
                    entry.order_index,
                    entry.source.as_str(),
                    entry.created_at,
                    entry.updated_at,
                ])
                .map_err(RepoError::from_sqlite)?;
            }

            Ok(())
        };

        match run() {
            Ok(()) => {
                conn.execute_batch("RELEASE toc_insert_batch;")
                    .map_err(RepoError::from_sqlite)?;
                Ok(())
            }
            Err(e) => {
                let _ =
                    conn.execute_batch("ROLLBACK TO toc_insert_batch; RELEASE toc_insert_batch;");
                Err(e)
            }
        }
    }

    /// Retrieves all table of contents entries for a paper in depth-first preorder.
    pub fn get_by_paper(conn: &Connection, paper_id: Uuid) -> Result<Vec<TocEntry>, RepoError> {
        let mut stmt = conn
            .prepare(
                "SELECT id, paper_id, parent_id, title, page_number, order_index, source,
                        created_at, updated_at
                 FROM toc_entries
                 WHERE paper_id = ?1
                 ORDER BY order_index ASC, created_at ASC",
            )
            .map_err(RepoError::from_sqlite)?;

        let rows = stmt
            .query_map(params![paper_id.to_string()], Self::row_to_toc_entry)
            .map_err(RepoError::from_sqlite)?;

        let mut raw_entries = Vec::new();
        for entry_res in rows {
            raw_entries.push(entry_res.map_err(RepoError::from_sqlite)?);
        }

        if raw_entries.is_empty() {
            return Ok(raw_entries);
        }

        let mut by_parent: HashMap<Option<Uuid>, Vec<TocEntry>> = HashMap::new();
        for entry in raw_entries {
            by_parent.entry(entry.parent_id).or_default().push(entry);
        }

        for siblings in by_parent.values_mut() {
            siblings.sort_by(|a, b| {
                a.order_index
                    .cmp(&b.order_index)
                    .then_with(|| a.created_at.cmp(&b.created_at))
            });
        }

        fn collect_preorder(
            parent_id: Option<Uuid>,
            by_parent: &mut HashMap<Option<Uuid>, Vec<TocEntry>>,
            out: &mut Vec<TocEntry>,
        ) {
            if let Some(children) = by_parent.remove(&parent_id) {
                for child in children {
                    let child_id = child.id;
                    out.push(child);
                    collect_preorder(Some(child_id), by_parent, out);
                }
            }
        }

        let mut ordered = Vec::new();
        collect_preorder(None, &mut by_parent, &mut ordered);

        if !by_parent.is_empty() {
            let mut remaining_parents: Vec<_> = by_parent.keys().copied().collect();
            remaining_parents.sort();
            for pid in remaining_parents {
                collect_preorder(pid, &mut by_parent, &mut ordered);
            }
        }

        Ok(ordered)
    }

    /// Finds a table of contents entry by its unique identifier.
    pub fn get_by_id(conn: &Connection, id: Uuid) -> Result<Option<TocEntry>, RepoError> {
        conn.query_row(
            "SELECT id, paper_id, parent_id, title, page_number, order_index, source,
                    created_at, updated_at
             FROM toc_entries WHERE id = ?1",
            params![id.to_string()],
            Self::row_to_toc_entry,
        )
        .optional()
        .map_err(RepoError::from_sqlite)
    }

    /// Deletes a table of contents entry by id.
    ///
    /// Note: Subtree entries (where `parent_id = id`) will cascade-delete
    /// via the SQLite self-referencing foreign key.
    pub fn delete(conn: &Connection, id: Uuid) -> Result<bool, RepoError> {
        let rows_affected = conn
            .execute(
                "DELETE FROM toc_entries WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(RepoError::from_sqlite)?;

        Ok(rows_affected > 0)
    }

    /// Deletes all table of contents entries for a given paper.
    pub fn delete_by_paper(conn: &Connection, paper_id: Uuid) -> Result<usize, RepoError> {
        let rows_affected = conn
            .execute(
                "DELETE FROM toc_entries WHERE paper_id = ?1",
                params![paper_id.to_string()],
            )
            .map_err(RepoError::from_sqlite)?;

        Ok(rows_affected)
    }

    /// Updates an existing table of contents entry.
    pub fn update(conn: &Connection, entry: &TocEntry) -> Result<(), RepoError> {
        let rows_affected = conn
            .execute(
                "UPDATE toc_entries SET
                    paper_id = ?2,
                    parent_id = ?3,
                    title = ?4,
                    page_number = ?5,
                    order_index = ?6,
                    source = ?7,
                    created_at = ?8,
                    updated_at = ?9
                 WHERE id = ?1",
                params![
                    entry.id.to_string(),
                    entry.paper_id.to_string(),
                    entry.parent_id.map(|u| u.to_string()),
                    entry.title,
                    entry.page_number,
                    entry.order_index,
                    entry.source.as_str(),
                    entry.created_at,
                    entry.updated_at,
                ],
            )
            .map_err(RepoError::from_sqlite)?;

        if rows_affected == 0 {
            return Err(RepoError::NotFound {
                entity: "toc_entry",
                id: entry.id.to_string(),
            });
        }

        Ok(())
    }

    fn row_to_toc_entry(row: &Row) -> Result<TocEntry, rusqlite::Error> {
        let id_str: String = row.get(0)?;
        let id = Uuid::parse_str(&id_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;

        let paper_id_str: String = row.get(1)?;
        let paper_id = Uuid::parse_str(&paper_id_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
        })?;

        let parent_id_opt: Option<String> = row.get(2)?;
        let parent_id = match parent_id_opt {
            Some(p) => Some(Uuid::parse_str(&p).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?),
            None => None,
        };

        let title: String = row.get(3)?;
        let page_number: u32 = row.get(4)?;
        let order_index: i32 = row.get(5)?;
        let source_str: String = row.get(6)?;
        let source = TocSource::from_str_internal(&source_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            )
        })?;
        let created_at: String = row.get(7)?;
        let updated_at: String = row.get(8)?;

        Ok(TocEntry {
            id,
            paper_id,
            parent_id,
            title,
            page_number,
            order_index,
            source,
            created_at,
            updated_at,
        })
    }
}
