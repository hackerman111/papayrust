use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::db::error::RepoError;

/// Repository for paper tags and tag associations.
pub struct TagRepo;

impl TagRepo {
    /// Retrieves all tags for a given paper, sorted alphabetically.
    pub fn get_tags_for_paper(conn: &Connection, paper_id: Uuid) -> Result<Vec<String>, RepoError> {
        let mut stmt = conn
            .prepare(
                "SELECT t.name FROM tags t
                 JOIN paper_tags pt ON pt.tag_id = t.id
                 WHERE pt.paper_id = ?1
                 ORDER BY t.name ASC",
            )
            .map_err(RepoError::from_sqlite)?;

        let rows = stmt
            .query_map(params![paper_id.to_string()], |row| row.get::<_, String>(0))
            .map_err(RepoError::from_sqlite)?;

        let mut tags = Vec::new();
        for r in rows {
            tags.push(r.map_err(RepoError::from_sqlite)?);
        }
        Ok(tags)
    }

    /// Sets the tags for a paper, replacing any existing tags for that paper.
    pub fn set_tags_for_paper(
        conn: &Connection,
        paper_id: Uuid,
        tags: &[String],
    ) -> Result<(), RepoError> {
        let paper_id_str = paper_id.to_string();

        // Remove existing associations
        conn.execute(
            "DELETE FROM paper_tags WHERE paper_id = ?1",
            params![paper_id_str],
        )
        .map_err(RepoError::from_sqlite)?;

        for tag_name in tags {
            let trimmed = tag_name.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Ensure tag exists in tags table
            let tag_id: String = match conn.query_row(
                "SELECT id FROM tags WHERE name = ?1",
                params![trimmed],
                |row| row.get(0),
            ) {
                Ok(id) => id,
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    let new_id = Uuid::now_v7().to_string();
                    conn.execute(
                        "INSERT INTO tags (id, name) VALUES (?1, ?2)",
                        params![new_id, trimmed],
                    )
                    .map_err(RepoError::from_sqlite)?;
                    new_id
                }
                Err(e) => return Err(RepoError::from_sqlite(e)),
            };

            // Associate paper with tag
            conn.execute(
                "INSERT OR IGNORE INTO paper_tags (paper_id, tag_id) VALUES (?1, ?2)",
                params![paper_id_str, tag_id],
            )
            .map_err(RepoError::from_sqlite)?;
        }

        Ok(())
    }

    /// Retrieves all distinct tag names in the library.
    pub fn get_all_tags(conn: &Connection) -> Result<Vec<String>, RepoError> {
        let mut stmt = conn
            .prepare("SELECT name FROM tags ORDER BY name ASC")
            .map_err(RepoError::from_sqlite)?;

        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(RepoError::from_sqlite)?;

        let mut tags = Vec::new();
        for r in rows {
            tags.push(r.map_err(RepoError::from_sqlite)?);
        }
        Ok(tags)
    }

    /// Finds all paper IDs associated with a specific tag name.
    pub fn get_paper_ids_by_tag(conn: &Connection, tag_name: &str) -> Result<Vec<Uuid>, RepoError> {
        let mut stmt = conn
            .prepare(
                "SELECT pt.paper_id FROM paper_tags pt
                 JOIN tags t ON t.id = pt.tag_id
                 WHERE t.name = ?1 COLLATE NOCASE",
            )
            .map_err(RepoError::from_sqlite)?;

        let rows = stmt
            .query_map(params![tag_name], |row| row.get::<_, String>(0))
            .map_err(RepoError::from_sqlite)?;

        let mut ids = Vec::new();
        for r in rows {
            let id_str = r.map_err(RepoError::from_sqlite)?;
            if let Ok(id) = Uuid::parse_str(&id_str) {
                ids.push(id);
            }
        }
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Paper;
    use crate::db::paper_repo::PaperRepo;

    fn make_test_paper(conn: &Connection, title: &str) -> Paper {
        let paper = Paper {
            id: Uuid::now_v7(),
            file_path: format!("/path/to/{title}.pdf"),
            content_hash: format!("hash_{title}"),
            title: Some(title.to_string()),
            authors: None,
            year: None,
            journal: None,
            doi: None,
            abstract_text: None,
            text_path: None,
            annotated_pdf_path: None,
            toc_embedded_at: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        PaperRepo::insert(conn, &paper).unwrap();
        paper
    }

    #[test]
    fn test_tag_crud_and_paper_association() {
        let conn = crate::db::open_in_memory().unwrap();
        let paper1 = make_test_paper(&conn, "Paper 1");
        let paper2 = make_test_paper(&conn, "Paper 2");

        // Paper 1 tags: ml, survey
        TagRepo::set_tags_for_paper(&conn, paper1.id, &["ml".to_string(), "survey".to_string()])
            .unwrap();

        // Paper 2 tags: ml, rl
        TagRepo::set_tags_for_paper(&conn, paper2.id, &["ml".to_string(), "rl".to_string()])
            .unwrap();

        let p1_tags = TagRepo::get_tags_for_paper(&conn, paper1.id).unwrap();
        assert_eq!(p1_tags, vec!["ml", "survey"]);

        let all_tags = TagRepo::get_all_tags(&conn).unwrap();
        assert_eq!(all_tags, vec!["ml", "rl", "survey"]);

        let ml_papers = TagRepo::get_paper_ids_by_tag(&conn, "ml").unwrap();
        assert_eq!(ml_papers.len(), 2);

        let survey_papers = TagRepo::get_paper_ids_by_tag(&conn, "survey").unwrap();
        assert_eq!(survey_papers, vec![paper1.id]);

        // Replace tags on Paper 1
        TagRepo::set_tags_for_paper(&conn, paper1.id, &["nlp".to_string()]).unwrap();
        let p1_updated = TagRepo::get_tags_for_paper(&conn, paper1.id).unwrap();
        assert_eq!(p1_updated, vec!["nlp"]);

        // Paper deletion cascades to paper_tags
        PaperRepo::delete(&conn, paper1.id).unwrap();
        let p1_empty = TagRepo::get_tags_for_paper(&conn, paper1.id).unwrap();
        assert!(p1_empty.is_empty());
    }
}
