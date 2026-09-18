use rusqlite::{params, Connection, OptionalExtension, Row};
use uuid::Uuid;

use crate::db::error::RepoError;
use crate::db::models::{Collection, Paper};
use crate::db::paper_repo::PaperRepo;

/// Repository for collection operations and paper-collection associations.
pub struct CollectionRepo;

impl CollectionRepo {
    /// Inserts a new collection.
    pub fn insert(conn: &Connection, collection: &Collection) -> Result<(), RepoError> {
        conn.execute(
            "INSERT INTO collections (id, name, parent_id) VALUES (?1, ?2, ?3)",
            params![
                collection.id.to_string(),
                collection.name,
                collection.parent_id.map(|u| u.to_string()),
            ],
        )
        .map_err(RepoError::from_sqlite)?;

        Ok(())
    }

    /// Finds a collection by its identifier.
    pub fn get_by_id(conn: &Connection, id: Uuid) -> Result<Option<Collection>, RepoError> {
        conn.query_row(
            "SELECT id, name, parent_id FROM collections WHERE id = ?1",
            params![id.to_string()],
            Self::row_to_collection,
        )
        .optional()
        .map_err(RepoError::from_sqlite)
    }

    /// Finds a collection by its unique name.
    pub fn get_by_name(conn: &Connection, name: &str) -> Result<Option<Collection>, RepoError> {
        conn.query_row(
            "SELECT id, name, parent_id FROM collections WHERE name = ?1",
            params![name],
            Self::row_to_collection,
        )
        .optional()
        .map_err(RepoError::from_sqlite)
    }

    /// Renames a collection. Returns true if the collection was found and updated.
    pub fn rename(conn: &Connection, id: Uuid, new_name: &str) -> Result<bool, RepoError> {
        let rows_affected = conn
            .execute(
                "UPDATE collections SET name = ?2 WHERE id = ?1",
                params![id.to_string(), new_name],
            )
            .map_err(RepoError::from_sqlite)?;

        Ok(rows_affected > 0)
    }

    /// Deletes a collection by id.
    ///
    /// Note: Does NOT delete papers in the collection. SQLite cascades remove
    /// rows from `paper_collections`, but papers themselves remain intact.
    pub fn delete(conn: &Connection, id: Uuid) -> Result<bool, RepoError> {
        let rows_affected = conn
            .execute(
                "DELETE FROM collections WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(RepoError::from_sqlite)?;

        Ok(rows_affected > 0)
    }

    /// Lists all collections ordered by name ascending.
    pub fn list(conn: &Connection) -> Result<Vec<Collection>, RepoError> {
        let mut stmt = conn
            .prepare("SELECT id, name, parent_id FROM collections ORDER BY name ASC")
            .map_err(RepoError::from_sqlite)?;

        let rows = stmt
            .query_map([], Self::row_to_collection)
            .map_err(RepoError::from_sqlite)?;

        let mut collections = Vec::new();
        for col_res in rows {
            collections.push(col_res.map_err(RepoError::from_sqlite)?);
        }

        Ok(collections)
    }

    /// Associates a paper with a collection.
    pub fn add_paper(
        conn: &Connection,
        paper_id: Uuid,
        collection_id: Uuid,
    ) -> Result<(), RepoError> {
        conn.execute(
            "INSERT INTO paper_collections (paper_id, collection_id) VALUES (?1, ?2)",
            params![paper_id.to_string(), collection_id.to_string()],
        )
        .map_err(RepoError::from_sqlite)?;

        Ok(())
    }

    /// Removes a paper from a collection. Returns true if the association was removed.
    pub fn remove_paper(
        conn: &Connection,
        paper_id: Uuid,
        collection_id: Uuid,
    ) -> Result<bool, RepoError> {
        let rows_affected = conn
            .execute(
                "DELETE FROM paper_collections WHERE paper_id = ?1 AND collection_id = ?2",
                params![paper_id.to_string(), collection_id.to_string()],
            )
            .map_err(RepoError::from_sqlite)?;

        Ok(rows_affected > 0)
    }

    /// Retrieves all papers belonging to a given collection.
    pub fn get_papers(conn: &Connection, collection_id: Uuid) -> Result<Vec<Paper>, RepoError> {
        let mut stmt = conn
            .prepare(
                "SELECT p.id, p.file_path, p.content_hash, p.title, p.authors, p.year,
                        p.journal, p.doi, p.abstract_text, p.text_path, p.annotated_pdf_path,
                        p.toc_embedded_at, p.created_at, p.updated_at
                 FROM papers p
                 INNER JOIN paper_collections pc ON p.id = pc.paper_id
                 WHERE pc.collection_id = ?1
                 ORDER BY p.created_at DESC",
            )
            .map_err(RepoError::from_sqlite)?;

        let rows = stmt
            .query_map(params![collection_id.to_string()], PaperRepo::row_to_paper)
            .map_err(RepoError::from_sqlite)?;

        let mut papers = Vec::new();
        for paper_res in rows {
            papers.push(paper_res.map_err(RepoError::from_sqlite)?);
        }

        Ok(papers)
    }

    /// Retrieves all collections to which a given paper belongs.
    pub fn get_collections_for_paper(
        conn: &Connection,
        paper_id: Uuid,
    ) -> Result<Vec<Collection>, RepoError> {
        let mut stmt = conn
            .prepare(
                "SELECT c.id, c.name, c.parent_id
                 FROM collections c
                 INNER JOIN paper_collections pc ON c.id = pc.collection_id
                 WHERE pc.paper_id = ?1
                 ORDER BY c.name ASC",
            )
            .map_err(RepoError::from_sqlite)?;

        let rows = stmt
            .query_map(params![paper_id.to_string()], Self::row_to_collection)
            .map_err(RepoError::from_sqlite)?;

        let mut collections = Vec::new();
        for col_res in rows {
            collections.push(col_res.map_err(RepoError::from_sqlite)?);
        }

        Ok(collections)
    }

    fn row_to_collection(row: &Row) -> Result<Collection, rusqlite::Error> {
        let id_str: String = row.get(0)?;
        let id = Uuid::parse_str(&id_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;
        let name: String = row.get(1)?;
        let parent_id_opt: Option<String> = row.get(2)?;
        let parent_id = match parent_id_opt {
            Some(s) => Some(Uuid::parse_str(&s).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?),
            None => None,
        };

        Ok(Collection {
            id,
            name,
            parent_id,
        })
    }
}
