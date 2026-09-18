use rusqlite::{params, Connection, OptionalExtension, Row};
use uuid::Uuid;

use crate::db::error::RepoError;
use crate::db::models::Paper;

/// Repository for paper persistence and queries.
pub struct PaperRepo;

impl PaperRepo {
    /// Inserts a new paper into the database.
    pub fn insert(conn: &Connection, paper: &Paper) -> Result<(), RepoError> {
        conn.execute(
            "INSERT INTO papers (
                id, file_path, content_hash, title, authors, year, journal, doi,
                abstract_text, text_path, annotated_pdf_path, toc_embedded_at,
                created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14
            )",
            params![
                paper.id.to_string(),
                paper.file_path,
                paper.content_hash,
                paper.title,
                paper.authors,
                paper.year,
                paper.journal,
                paper.doi,
                paper.abstract_text,
                paper.text_path,
                paper.annotated_pdf_path,
                paper.toc_embedded_at,
                paper.created_at,
                paper.updated_at,
            ],
        )
        .map_err(RepoError::from_sqlite)?;

        Ok(())
    }

    /// Finds a paper by its unique identifier.
    pub fn get_by_id(conn: &Connection, id: Uuid) -> Result<Option<Paper>, RepoError> {
        conn.query_row(
            "SELECT id, file_path, content_hash, title, authors, year, journal, doi,
                    abstract_text, text_path, annotated_pdf_path, toc_embedded_at,
                    created_at, updated_at
             FROM papers WHERE id = ?1",
            params![id.to_string()],
            Self::row_to_paper,
        )
        .optional()
        .map_err(RepoError::from_sqlite)
    }

    /// Finds a paper by its unique content hash.
    pub fn get_by_content_hash(conn: &Connection, hash: &str) -> Result<Option<Paper>, RepoError> {
        conn.query_row(
            "SELECT id, file_path, content_hash, title, authors, year, journal, doi,
                    abstract_text, text_path, annotated_pdf_path, toc_embedded_at,
                    created_at, updated_at
             FROM papers WHERE content_hash = ?1",
            params![hash],
            Self::row_to_paper,
        )
        .optional()
        .map_err(RepoError::from_sqlite)
    }

    /// Finds a paper by its unique file path.
    pub fn get_by_file_path(conn: &Connection, path: &str) -> Result<Option<Paper>, RepoError> {
        conn.query_row(
            "SELECT id, file_path, content_hash, title, authors, year, journal, doi,
                    abstract_text, text_path, annotated_pdf_path, toc_embedded_at,
                    created_at, updated_at
             FROM papers WHERE file_path = ?1",
            params![path],
            Self::row_to_paper,
        )
        .optional()
        .map_err(RepoError::from_sqlite)
    }

    /// Updates an existing paper record.
    pub fn update(conn: &Connection, paper: &Paper) -> Result<(), RepoError> {
        let rows_affected = conn
            .execute(
                "UPDATE papers SET
                    file_path = ?2,
                    content_hash = ?3,
                    title = ?4,
                    authors = ?5,
                    year = ?6,
                    journal = ?7,
                    doi = ?8,
                    abstract_text = ?9,
                    text_path = ?10,
                    annotated_pdf_path = ?11,
                    toc_embedded_at = ?12,
                    created_at = ?13,
                    updated_at = ?14
                 WHERE id = ?1",
                params![
                    paper.id.to_string(),
                    paper.file_path,
                    paper.content_hash,
                    paper.title,
                    paper.authors,
                    paper.year,
                    paper.journal,
                    paper.doi,
                    paper.abstract_text,
                    paper.text_path,
                    paper.annotated_pdf_path,
                    paper.toc_embedded_at,
                    paper.created_at,
                    paper.updated_at,
                ],
            )
            .map_err(RepoError::from_sqlite)?;

        if rows_affected == 0 {
            return Err(RepoError::NotFound {
                entity: "paper",
                id: paper.id.to_string(),
            });
        }

        Ok(())
    }

    /// Deletes a paper by its id. Returns true if a record was deleted, false otherwise.
    pub fn delete(conn: &Connection, id: Uuid) -> Result<bool, RepoError> {
        let rows_affected = conn
            .execute("DELETE FROM papers WHERE id = ?1", params![id.to_string()])
            .map_err(RepoError::from_sqlite)?;

        Ok(rows_affected > 0)
    }

    /// Lists all papers ordered by creation date descending.
    pub fn list(conn: &Connection) -> Result<Vec<Paper>, RepoError> {
        let mut stmt = conn
            .prepare(
                "SELECT id, file_path, content_hash, title, authors, year, journal, doi,
                        abstract_text, text_path, annotated_pdf_path, toc_embedded_at,
                        created_at, updated_at
                 FROM papers ORDER BY created_at DESC",
            )
            .map_err(RepoError::from_sqlite)?;

        let rows = stmt
            .query_map([], Self::row_to_paper)
            .map_err(RepoError::from_sqlite)?;

        let mut papers = Vec::new();
        for paper_res in rows {
            papers.push(paper_res.map_err(RepoError::from_sqlite)?);
        }

        Ok(papers)
    }

    pub(crate) fn row_to_paper(row: &Row) -> Result<Paper, rusqlite::Error> {
        let id_str: String = row.get(0)?;
        let id = Uuid::parse_str(&id_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;

        Ok(Paper {
            id,
            file_path: row.get(1)?,
            content_hash: row.get(2)?,
            title: row.get(3)?,
            authors: row.get(4)?,
            year: row.get(5)?,
            journal: row.get(6)?,
            doi: row.get(7)?,
            abstract_text: row.get(8)?,
            text_path: row.get(9)?,
            annotated_pdf_path: row.get(10)?,
            toc_embedded_at: row.get(11)?,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
        })
    }
}
