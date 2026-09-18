use uuid::Uuid;

/// Identity key for collections in the TUI (real DB collections and virtual collections).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CollectionKey {
    /// Virtual collection displaying all papers.
    All,
    /// Real SQLite database collection identified by its UUID.
    Real(Uuid),
    /// Virtual collection displaying recently added papers.
    RecentlyAdded,
    /// Virtual collection displaying papers not in any collection.
    Unfiled,
    /// Virtual collection displaying papers without any tags.
    Untagged,
}

impl CollectionKey {
    /// Returns the database UUID if this represents a real collection.
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            Self::Real(id) => Some(*id),
            _ => None,
        }
    }
}

/// Authoritative selection state in the TUI (Single Source of Truth).
///
/// List indices (`selected_collection`, `selected_paper`, `selected_toc`)
/// are derived UI caches for rendering and must be recomputed from this state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionState {
    /// Key of the currently selected collection.
    pub collection: CollectionKey,
    /// UUID of the currently selected paper, or `None` if the list is empty.
    pub paper_id: Option<Uuid>,
    /// UUID of the currently selected TOC entry, or `None` if no TOC entry is selected.
    pub toc_id: Option<Uuid>,
}

impl Default for SelectionState {
    fn default() -> Self {
        Self {
            collection: CollectionKey::All,
            paper_id: None,
            toc_id: None,
        }
    }
}

use crate::app::App;

impl App {
    /// Synchronizes `SelectionState` from the current UI indices.
    /// Also updates position memory for the currently selected collection and paper.
    pub fn update_selection_from_indices(&mut self) {
        if self.collections.is_empty() {
            self.selection.collection = CollectionKey::All;
            self.selection.paper_id = None;
            self.selection.toc_id = None;
            return;
        }

        let col_idx = self
            .selected_collection
            .min(self.collections.len().saturating_sub(1));
        self.selection.collection = self.collections[col_idx].key.clone();

        if self.papers.is_empty() {
            self.selection.paper_id = None;
            self.selection.toc_id = None;
        } else {
            let paper_idx = self.selected_paper.min(self.papers.len().saturating_sub(1));
            let pid = self.papers[paper_idx].id;
            self.selection.paper_id = Some(pid);
            self.last_paper_by_collection
                .insert(self.selection.collection.clone(), pid);

            if self.toc_preview.is_empty() {
                self.selection.toc_id = None;
            } else {
                let toc_idx = self
                    .selected_toc
                    .min(self.toc_preview.len().saturating_sub(1));
                let tid = self.toc_preview[toc_idx].id;
                self.selection.toc_id = Some(tid);
                self.last_toc_by_paper.insert(pid, tid);
            }
        }
    }

    /// Records the currently selected paper in position memory for the current collection.
    pub fn record_position_for_current_collection(&mut self) {
        if let Some(paper_id) = self.selection.paper_id {
            self.last_paper_by_collection
                .insert(self.selection.collection.clone(), paper_id);
        }
    }

    /// Records the currently selected TOC entry in position memory for the current paper.
    pub fn record_position_for_current_paper(&mut self) {
        if let (Some(paper_id), Some(toc_id)) = (self.selection.paper_id, self.selection.toc_id) {
            self.last_toc_by_paper.insert(paper_id, toc_id);
        }
    }

    /// Recomputes UI indices from authoritative `SelectionState` (UUIDs).
    ///
    /// If an item with the expected UUID is found, its new index is set.
    /// If the item was removed or is not found in the current view:
    /// - Falls back to position memory (`last_paper_by_collection` / `last_toc_by_paper`).
    /// - If still not found, clamps to the nearest valid index (or 0).
    pub fn restore_selection_by_uuid(&mut self) {
        // 1. Restore collection
        if self.collections.is_empty() {
            self.selected_collection = 0;
            self.selection.collection = CollectionKey::All;
        } else if let Some(pos) = self
            .collections
            .iter()
            .position(|c| c.key == self.selection.collection)
        {
            self.selected_collection = pos;
        } else {
            self.selected_collection = self.selected_collection.min(self.collections.len() - 1);
            self.selection.collection = self.collections[self.selected_collection].key.clone();
        }

        // 2. Sync papers list
        if self.collections.is_empty() {
            self.papers.clear();
            self.selected_paper = 0;
            self.selection.paper_id = None;
        } else {
            let key = self.collections[self.selected_collection].id;
            self.papers = self
                .papers_by_collection
                .get(&key)
                .cloned()
                .unwrap_or_default();

            // 3. Restore paper
            if self.papers.is_empty() {
                self.selected_paper = 0;
                self.selection.paper_id = None;
            } else {
                let target_paper = if let Some(pid) = self.selection.paper_id {
                    if self.papers.iter().any(|p| p.id == pid) {
                        Some(pid)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let target_paper = target_paper.or_else(|| {
                    self.last_paper_by_collection
                        .get(&self.selection.collection)
                        .copied()
                        .filter(|pid| self.papers.iter().any(|p| p.id == *pid))
                });

                if let Some(pid) = target_paper {
                    self.selected_paper = self.papers.iter().position(|p| p.id == pid).unwrap_or(0);
                    self.selection.paper_id = Some(pid);
                } else {
                    self.selected_paper = self.selected_paper.min(self.papers.len() - 1);
                    self.selection.paper_id = Some(self.papers[self.selected_paper].id);
                }
            }
        }

        // 4. Sync TOC preview
        if self.papers.is_empty() {
            self.toc_preview.clear();
            self.selected_toc = 0;
            self.selection.toc_id = None;
        } else {
            let paper_id = self.papers[self.selected_paper].id;
            self.toc_preview = self
                .tocs_by_paper
                .get(&paper_id)
                .cloned()
                .unwrap_or_default();

            // 5. Restore TOC
            if self.toc_preview.is_empty() {
                self.selected_toc = 0;
                self.selection.toc_id = None;
            } else {
                let target_toc = if let Some(tid) = self.selection.toc_id {
                    if self.toc_preview.iter().any(|t| t.id == tid) {
                        Some(tid)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let target_toc = target_toc.or_else(|| {
                    self.selection
                        .paper_id
                        .and_then(|pid| self.last_toc_by_paper.get(&pid).copied())
                        .filter(|tid| self.toc_preview.iter().any(|t| t.id == *tid))
                });

                if let Some(tid) = target_toc {
                    self.selected_toc = self
                        .toc_preview
                        .iter()
                        .position(|t| t.id == tid)
                        .unwrap_or(0);
                    self.selection.toc_id = Some(tid);
                } else {
                    self.selected_toc = self.selected_toc.min(self.toc_preview.len() - 1);
                    self.selection.toc_id = Some(self.toc_preview[self.selected_toc].id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use papyrus_core::db::Paper;

    #[test]
    fn test_collection_key_as_uuid() {
        assert_eq!(CollectionKey::All.as_uuid(), None);
        assert_eq!(CollectionKey::RecentlyAdded.as_uuid(), None);
        assert_eq!(CollectionKey::Unfiled.as_uuid(), None);
        assert_eq!(CollectionKey::Untagged.as_uuid(), None);

        let id = Uuid::new_v4();
        assert_eq!(CollectionKey::Real(id).as_uuid(), Some(id));
    }

    #[test]
    fn test_selection_state_default() {
        let state = SelectionState::default();
        assert_eq!(state.collection, CollectionKey::All);
        assert_eq!(state.paper_id, None);
        assert_eq!(state.toc_id, None);
    }

    #[test]
    fn test_collection_item_keys() {
        use crate::app::CollectionItem;

        let all_item = CollectionItem::new(None, "All Papers", 10);
        assert_eq!(all_item.key, CollectionKey::All);
        assert_eq!(all_item.id, None);

        let uid = Uuid::new_v4();
        let real_item = CollectionItem::new(Some(uid), "Real Col", 5);
        assert_eq!(real_item.key, CollectionKey::Real(uid));
        assert_eq!(real_item.id, Some(uid));

        let unfiled_item = CollectionItem::from_key(CollectionKey::Unfiled, "Unfiled", 2);
        assert_eq!(unfiled_item.key, CollectionKey::Unfiled);
        assert_eq!(unfiled_item.id, None);
    }

    fn dummy_paper(title: &str) -> Paper {
        Paper {
            id: Uuid::now_v7(),
            file_path: format!("/path/{title}.pdf"),
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
        }
    }

    #[test]
    fn test_selection_update_and_restore_reorder() {
        use crate::app::CollectionItem;
        use std::collections::HashMap;

        let col_id = Uuid::new_v4();
        let p1 = dummy_paper("Paper 1");
        let p2 = dummy_paper("Paper 2");
        let p3 = dummy_paper("Paper 3");

        let p2_id = p2.id;

        let col_item = CollectionItem::new(Some(col_id), "Col 1", 3);
        let mut papers_by_col = HashMap::new();
        papers_by_col.insert(Some(col_id), vec![p1.clone(), p2.clone(), p3.clone()]);

        let mut app = App::with_data(vec![col_item], papers_by_col, HashMap::new());
        assert_eq!(app.papers.len(), 3);

        // Select paper 2 (index 1)
        app.selected_paper = 1;
        app.update_selection_from_indices();
        assert_eq!(app.selection.paper_id, Some(p2_id));

        // Now simulate reordering: [p2, p3, p1] in papers_by_collection
        app.papers_by_collection
            .insert(Some(col_id), vec![p2, p3, p1]);

        // Call restore_selection_by_uuid
        app.restore_selection_by_uuid();

        // Selected paper should now be index 0, still matching p2_id!
        assert_eq!(app.selected_paper, 0);
        assert_eq!(app.selection.paper_id, Some(p2_id));
    }
}
