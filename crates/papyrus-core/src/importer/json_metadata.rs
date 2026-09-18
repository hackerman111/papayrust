use rusqlite::Connection;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::db::paper_repo::PaperRepo;
use crate::db::tag_repo::TagRepo;
use crate::metadata_editor::{update_metadata, MetadataError, UpdatePaperMetadata};
use crate::search::SearchIndex;

/// Metadata entry structure parsed from JSON files.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct JsonPaperMetadata {
    pub doi: Option<String>,
    pub title: Option<String>,
    pub authors: Option<String>,
    pub year: Option<i64>,
    pub journal: Option<String>,
    pub abstract_text: Option<String>,
    #[serde(alias = "abstract")]
    pub abstract_alias: Option<String>,
    pub file: Option<String>,
    pub filename: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Imports metadata from a JSON file or a directory containing JSON files.
/// Returns the number of papers successfully updated.
pub fn import_metadata_from_path(
    conn: &mut Connection,
    path: &Path,
    selected_paper_id: Option<Uuid>,
    search_index: Option<&SearchIndex>,
) -> Result<usize, MetadataError> {
    if !path.exists() {
        return Err(MetadataError::Validation(format!(
            "Path does not exist: {}",
            path.display()
        )));
    }

    if path.is_dir() {
        let mut json_files = Vec::new();
        collect_json_files(path, &mut json_files);
        json_files.sort();

        let mut total_updated = 0;
        for file in json_files {
            // In directory mode, we match without defaulting to selected_paper_id
            let updated = import_metadata_from_file(conn, &file, None, search_index)?;
            total_updated += updated;
        }
        Ok(total_updated)
    } else {
        import_metadata_from_file(conn, path, selected_paper_id, search_index)
    }
}

/// Recursively collects all `.json` files in `dir`.
fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_json_files(&p, out);
        } else if p.is_file() {
            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                if ext.eq_ignore_ascii_case("json") {
                    out.push(p);
                }
            }
        }
    }
}

/// Imports metadata from a single JSON file.
fn import_metadata_from_file(
    conn: &mut Connection,
    path: &Path,
    selected_paper_id: Option<Uuid>,
    search_index: Option<&SearchIndex>,
) -> Result<usize, MetadataError> {
    let content = std::fs::read_to_string(path)?;
    let items: Vec<JsonPaperMetadata> =
        if let Ok(single) = serde_json::from_str::<JsonPaperMetadata>(&content) {
            vec![single]
        } else if let Ok(multiple) = serde_json::from_str::<Vec<JsonPaperMetadata>>(&content) {
            multiple
        } else {
            return Err(MetadataError::Validation(format!(
                "Failed to parse JSON from {}: expected JSON object or array of objects",
                path.display()
            )));
        };

    if items.is_empty() {
        return Ok(0);
    }

    let is_single_item = items.len() == 1;
    let mut updated_count = 0;

    for item in items {
        let all_papers = PaperRepo::list(conn)?;
        let matched_paper = all_papers.iter().find(|p| {
            // 1. Match by DOI if present
            if let (Some(ref item_doi), Some(ref p_doi)) = (&item.doi, &p.doi) {
                let trimmed_item = item_doi.trim();
                let trimmed_p = p_doi.trim();
                if !trimmed_item.is_empty() && trimmed_item.eq_ignore_ascii_case(trimmed_p) {
                    return true;
                }
            }

            // 2. Match by file or filename if present
            let file_query = item.file.as_deref().or(item.filename.as_deref());
            if let Some(q) = file_query {
                let q_trimmed = q.trim();
                if !q_trimmed.is_empty() {
                    let p_path = Path::new(&p.file_path);
                    let p_fname = p_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    let p_stem = p_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    if p_fname.eq_ignore_ascii_case(q_trimmed)
                        || p_stem.eq_ignore_ascii_case(q_trimmed)
                        || p.file_path.ends_with(q_trimmed)
                    {
                        return true;
                    }
                }
            }

            // 3. Match by exact title if present
            if let (Some(ref item_title), Some(ref p_title)) = (&item.title, &p.title) {
                let t_item = item_title.trim();
                let t_p = p_title.trim();
                if !t_item.is_empty() && t_item.eq_ignore_ascii_case(t_p) {
                    return true;
                }
            }

            // 4. Fallback: single item in file and selected_paper_id is provided
            if is_single_item && selected_paper_id == Some(p.id) {
                return true;
            }

            false
        });

        if let Some(paper) = matched_paper {
            let paper_id = paper.id;
            let current_title = paper.title.clone().unwrap_or_default();
            let new_title = item
                .title
                .filter(|t| !t.trim().is_empty())
                .unwrap_or(current_title);

            let new_authors = item.authors.or_else(|| paper.authors.clone());
            let new_year = item.year.or(paper.year);
            let new_journal = item.journal.or_else(|| paper.journal.clone());
            let new_doi = item.doi.or_else(|| paper.doi.clone());
            let new_abstract = item
                .abstract_text
                .or(item.abstract_alias)
                .or_else(|| paper.abstract_text.clone());

            let update = UpdatePaperMetadata {
                title: new_title,
                authors: new_authors,
                year: new_year,
                journal: new_journal,
                doi: new_doi,
                abstract_text: new_abstract,
            };

            update_metadata(conn, paper_id, update, search_index)?;

            if let Some(ref tags) = item.tags {
                TagRepo::set_tags_for_paper(conn, paper_id, tags)?;
            }

            updated_count += 1;
        }
    }

    Ok(updated_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{apply_migrations, open_in_memory, Paper};
    use tempfile::tempdir;

    fn sample_paper(id: Uuid, title: &str, doi: Option<&str>, file_name: &str) -> Paper {
        Paper {
            id,
            file_path: format!("/papers/{file_name}"),
            content_hash: format!("hash_{title}"),
            title: Some(title.to_string()),
            authors: Some("Original Author".to_string()),
            year: Some(2020),
            journal: Some("Old Journal".to_string()),
            doi: doi.map(|s| s.to_string()),
            abstract_text: Some("Old abstract".to_string()),
            text_path: None,
            annotated_pdf_path: None,
            toc_embedded_at: None,
            created_at: "2020-01-01T00:00:00Z".to_string(),
            updated_at: "2020-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_import_single_json_by_doi() {
        let mut conn = open_in_memory().unwrap();
        apply_migrations(&mut conn).unwrap();

        let id = Uuid::now_v7();
        let paper = sample_paper(id, "Old Title", Some("10.1000/182"), "paper.pdf");
        PaperRepo::insert(&conn, &paper).unwrap();

        let dir = tempdir().unwrap();
        let json_path = dir.path().join("meta.json");
        std::fs::write(
            &json_path,
            r#"{
                "doi": "10.1000/182",
                "title": "New Updated Title",
                "authors": "New Author A, New Author B",
                "year": 2024,
                "tags": ["machine-learning", "ai"]
            }"#,
        )
        .unwrap();

        let count = import_metadata_from_path(&mut conn, &json_path, None, None).unwrap();
        assert_eq!(count, 1);

        let updated = PaperRepo::get_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(updated.title.as_deref(), Some("New Updated Title"));
        assert_eq!(
            updated.authors.as_deref(),
            Some("New Author A, New Author B")
        );
        assert_eq!(updated.year, Some(2024));

        let tags = TagRepo::get_tags_for_paper(&conn, id).unwrap();
        assert_eq!(tags, vec!["ai", "machine-learning"]);
    }

    #[test]
    fn test_import_single_json_by_selected_paper_id() {
        let mut conn = open_in_memory().unwrap();
        apply_migrations(&mut conn).unwrap();

        let id = Uuid::now_v7();
        let paper = sample_paper(id, "Some Paper", None, "some.pdf");
        PaperRepo::insert(&conn, &paper).unwrap();

        let dir = tempdir().unwrap();
        let json_path = dir.path().join("meta.json");
        std::fs::write(
            &json_path,
            r#"{
                "title": "Renamed Paper",
                "journal": "Nature AI",
                "year": 2025
            }"#,
        )
        .unwrap();

        let count = import_metadata_from_path(&mut conn, &json_path, Some(id), None).unwrap();
        assert_eq!(count, 1);

        let updated = PaperRepo::get_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(updated.title.as_deref(), Some("Renamed Paper"));
        assert_eq!(updated.journal.as_deref(), Some("Nature AI"));
        assert_eq!(updated.year, Some(2025));
    }
}
