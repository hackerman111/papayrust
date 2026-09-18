use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::db::models::{Collection, Paper, TocEntry};
use crate::toc::PendingTocEntry;

/// Root metadata document inside the exported ZIP archive (`manifest.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub exported_at: String,
    pub collections: Vec<ManifestCollection>,
    pub papers: Vec<ManifestPaper>,
}

/// Collection metadata entry in `manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestCollection {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
}

impl From<&Collection> for ManifestCollection {
    fn from(c: &Collection) -> Self {
        Self {
            id: c.id,
            name: c.name.clone(),
            parent_id: c.parent_id,
        }
    }
}

/// Paper metadata entry in `manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestPaper {
    pub id: Uuid,
    pub file_path: String,
    pub content_hash: String,
    pub title: Option<String>,
    pub authors: Option<String>,
    pub year: Option<i64>,
    pub journal: Option<String>,
    pub doi: Option<String>,
    pub abstract_text: Option<String>,
    pub annotated_pdf_path: Option<String>,
    pub collections: Vec<Uuid>,
    pub toc_entries: Vec<PendingTocEntry>,
    pub created_at: String,
    pub updated_at: String,
}

/// Converts a preorder sequence of `TocEntry` items into `PendingTocEntry` items with computed tree levels.
pub fn compute_toc_hierarchy(entries: &[TocEntry]) -> Vec<PendingTocEntry> {
    if entries.is_empty() {
        return Vec::new();
    }

    let mut levels: HashMap<Uuid, usize> = HashMap::with_capacity(entries.len());
    let mut result = Vec::with_capacity(entries.len());

    for entry in entries {
        let level = match entry.parent_id {
            Some(parent_id) => levels.get(&parent_id).copied().unwrap_or(0) + 1,
            None => 0,
        };
        levels.insert(entry.id, level);
        result.push(PendingTocEntry {
            title: entry.title.clone(),
            page: entry.page_number,
            level,
        });
    }

    result
}

/// Constructs a `ManifestPaper` instance.
pub fn build_manifest_paper(
    paper: &Paper,
    archive_file_path: String,
    archive_annotated_path: Option<String>,
    collection_ids: Vec<Uuid>,
    toc_entries: &[TocEntry],
) -> ManifestPaper {
    ManifestPaper {
        id: paper.id,
        file_path: archive_file_path,
        content_hash: paper.content_hash.clone(),
        title: paper.title.clone(),
        authors: paper.authors.clone(),
        year: paper.year,
        journal: paper.journal.clone(),
        doi: paper.doi.clone(),
        abstract_text: paper.abstract_text.clone(),
        annotated_pdf_path: archive_annotated_path,
        collections: collection_ids,
        toc_entries: compute_toc_hierarchy(toc_entries),
        created_at: paper.created_at.clone(),
        updated_at: paper.updated_at.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::TocSource;

    #[test]
    fn test_compute_toc_hierarchy_empty() {
        assert_eq!(compute_toc_hierarchy(&[]), vec![]);
    }

    #[test]
    fn test_compute_toc_hierarchy_nested() {
        let root1_id = Uuid::new_v4();
        let child1_id = Uuid::new_v4();
        let grandchild_id = Uuid::new_v4();
        let root2_id = Uuid::new_v4();

        let entries = vec![
            TocEntry {
                id: root1_id,
                paper_id: Uuid::nil(),
                parent_id: None,
                title: "Chapter 1".into(),
                page_number: 1,
                order_index: 0,
                source: TocSource::Manual,
                created_at: "".into(),
                updated_at: "".into(),
            },
            TocEntry {
                id: child1_id,
                paper_id: Uuid::nil(),
                parent_id: Some(root1_id),
                title: "Section 1.1".into(),
                page_number: 2,
                order_index: 0,
                source: TocSource::Manual,
                created_at: "".into(),
                updated_at: "".into(),
            },
            TocEntry {
                id: grandchild_id,
                paper_id: Uuid::nil(),
                parent_id: Some(child1_id),
                title: "SubSection 1.1.1".into(),
                page_number: 3,
                order_index: 0,
                source: TocSource::Manual,
                created_at: "".into(),
                updated_at: "".into(),
            },
            TocEntry {
                id: root2_id,
                paper_id: Uuid::nil(),
                parent_id: None,
                title: "Chapter 2".into(),
                page_number: 10,
                order_index: 1,
                source: TocSource::Manual,
                created_at: "".into(),
                updated_at: "".into(),
            },
        ];

        let hierarchy = compute_toc_hierarchy(&entries);
        assert_eq!(hierarchy.len(), 4);
        assert_eq!(hierarchy[0].title, "Chapter 1");
        assert_eq!(hierarchy[0].level, 0);
        assert_eq!(hierarchy[1].title, "Section 1.1");
        assert_eq!(hierarchy[1].level, 1);
        assert_eq!(hierarchy[2].title, "SubSection 1.1.1");
        assert_eq!(hierarchy[2].level, 2);
        assert_eq!(hierarchy[3].title, "Chapter 2");
        assert_eq!(hierarchy[3].level, 0);
    }
}
