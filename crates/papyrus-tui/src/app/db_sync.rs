use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::app::{ActivePanel, App, CollectionItem};
use papyrus_core::db::{CollectionRepo, PaperRepo, RepoError, TocRepo};

impl App {
    /// Loads application state from a SQLite database connection, including an "All Papers" collection.
    pub fn from_db(conn: &Connection) -> Result<Self, RepoError> {
        Self::from_db_options(conn, true)
    }

    /// Loads application state from an owned SQLite database connection and stores the connection in App.
    pub fn from_db_conn(conn: Connection) -> Result<Self, RepoError> {
        Self::from_db_conn_options(conn, true)
    }

    /// Loads application state from an owned SQLite database connection with configurable "All Papers" entry.
    pub fn from_db_conn_options(
        conn: Connection,
        include_all_papers: bool,
    ) -> Result<Self, RepoError> {
        let mut app = Self::from_db_options(&conn, include_all_papers)?;
        app.set_db_conn(conn);
        Ok(app)
    }

    /// Loads application state from a SQLite database connection with configurable "All Papers" entry.
    pub fn from_db_options(conn: &Connection, include_all_papers: bool) -> Result<Self, RepoError> {
        let mut app = Self::new();
        app.load_from_db_options(conn, include_all_papers)?;
        Ok(app)
    }

    /// Reloads application state from a SQLite database connection.
    pub fn load_from_db(&mut self, conn: &Connection) -> Result<(), RepoError> {
        self.load_from_db_options(conn, true)
    }

    /// Reloads papers and collections from the internal SQLite connection, preserving current collection.
    pub fn reload_from_db(&mut self) -> Result<(), RepoError> {
        if let Some(ref conn) = self.db_conn {
            let all_papers = PaperRepo::list(conn)?;
            let db_collections = CollectionRepo::list(conn)?;

            let mut collections = Vec::new();
            let mut papers_by_collection = HashMap::new();
            let mut tocs_by_paper = HashMap::new();
            let mut tags_by_paper = HashMap::new();

            for paper in &all_papers {
                let tocs = TocRepo::get_by_paper(conn, paper.id)?;
                tocs_by_paper.insert(paper.id, tocs);
                let tags = papyrus_core::db::TagRepo::get_tags_for_paper(conn, paper.id)?;
                tags_by_paper.insert(paper.id, tags);
            }

            collections.push(CollectionItem::with_hierarchy(
                None,
                "All Papers",
                all_papers.len(),
                0,
                None,
            ));
            papers_by_collection.insert(None, all_papers);

            let mut paper_counts = HashMap::new();
            for col in &db_collections {
                let papers = CollectionRepo::get_papers(conn, col.id)?;
                paper_counts.insert(col.id, papers.len());
                papers_by_collection.insert(Some(col.id), papers);
            }

            let hierarchical_items =
                build_hierarchical_collection_items(&db_collections, &paper_counts);
            collections.extend(hierarchical_items);

            self.collections = collections;
            self.papers_by_collection = papers_by_collection;
            self.tocs_by_paper = tocs_by_paper;
            self.tags_by_paper = tags_by_paper;
            self.update_breadcrumbs();
            self.restore_selection_by_uuid();
        }
        Ok(())
    }

    /// Reloads application state from a SQLite database connection with configurable "All Papers" entry.
    pub fn load_from_db_options(
        &mut self,
        conn: &Connection,
        include_all_papers: bool,
    ) -> Result<(), RepoError> {
        let all_papers = PaperRepo::list(conn)?;
        let db_collections = CollectionRepo::list(conn)?;

        let mut collections = Vec::new();
        let mut papers_by_collection = HashMap::new();
        let mut tocs_by_paper = HashMap::new();
        let mut tags_by_paper = HashMap::new();

        // Load TOC entries and tags for all papers
        for paper in &all_papers {
            let tocs = TocRepo::get_by_paper(conn, paper.id)?;
            tocs_by_paper.insert(paper.id, tocs);
            let tags = papyrus_core::db::TagRepo::get_tags_for_paper(conn, paper.id)?;
            tags_by_paper.insert(paper.id, tags);
        }

        // Include "All Papers" virtual collection if requested or if there are no DB collections
        if include_all_papers || db_collections.is_empty() {
            collections.push(CollectionItem::with_hierarchy(
                None,
                "All Papers",
                all_papers.len(),
                0,
                None,
            ));
            papers_by_collection.insert(None, all_papers);
        }

        // Populate database collections with hierarchy
        let mut paper_counts = HashMap::new();
        for col in &db_collections {
            let papers = CollectionRepo::get_papers(conn, col.id)?;
            paper_counts.insert(col.id, papers.len());
            papers_by_collection.insert(Some(col.id), papers);
        }

        let hierarchical_items =
            build_hierarchical_collection_items(&db_collections, &paper_counts);
        collections.extend(hierarchical_items);

        self.collections = collections;
        self.papers_by_collection = papers_by_collection;
        self.tocs_by_paper = tocs_by_paper;
        self.tags_by_paper = tags_by_paper;
        self.active_panel = ActivePanel::Papers;
        self.selected_collection = 0;
        self.selected_paper = 0;
        self.selected_toc = 0;
        self.update_breadcrumbs();
        self.sync_current_selection();

        Ok(())
    }

    /// Synchronizes the papers list for the currently selected collection.
    pub fn sync_current_selection(&mut self) {
        if self.collections.is_empty() {
            self.selected_collection = 0;
            self.papers.clear();
            self.selected_paper = 0;
        } else {
            if self.selected_collection >= self.collections.len() {
                self.selected_collection = self.collections.len() - 1;
            }
            if !self.search_query.trim().is_empty() {
                self.apply_search_filter();
                return;
            }
            let key = self.collections[self.selected_collection].id;
            self.papers = self
                .papers_by_collection
                .get(&key)
                .cloned()
                .unwrap_or_default();

            crate::app::sorting::sort_papers(
                &mut self.papers,
                self.sort_field,
                self.sort_direction,
            );

            // Check position memory for the newly active collection
            let col_key = &self.collections[self.selected_collection].key;
            if let Some(&remembered_pid) = self.last_paper_by_collection.get(col_key) {
                if let Some(pos) = self.papers.iter().position(|p| p.id == remembered_pid) {
                    self.selected_paper = pos;
                } else {
                    self.selected_paper =
                        self.selected_paper.min(self.papers.len().saturating_sub(1));
                }
            } else {
                self.selected_paper = self.selected_paper.min(self.papers.len().saturating_sub(1));
            }
        }
        self.selected_toc = 0;
        self.sync_paper_selection();
        self.update_selection_from_indices();
    }

    /// Synchronizes the details and TOC preview for the currently selected paper.
    pub fn sync_paper_selection(&mut self) {
        if self.papers.is_empty() {
            self.selected_paper = 0;
            self.toc_preview.clear();
            self.selected_toc = 0;
        } else {
            if self.selected_paper >= self.papers.len() {
                self.selected_paper = self.papers.len() - 1;
            }
            let paper_id = self.papers[self.selected_paper].id;
            self.toc_preview = self
                .tocs_by_paper
                .get(&paper_id)
                .cloned()
                .unwrap_or_default();

            // Check position memory for the newly active paper
            if let Some(&remembered_tid) = self.last_toc_by_paper.get(&paper_id) {
                if let Some(pos) = self.toc_preview.iter().position(|t| t.id == remembered_tid) {
                    self.selected_toc = pos;
                } else {
                    self.selected_toc = self
                        .selected_toc
                        .min(self.toc_preview.len().saturating_sub(1));
                }
            } else {
                self.selected_toc = self
                    .selected_toc
                    .min(self.toc_preview.len().saturating_sub(1));
            }
        }
    }
}

/// Builds a depth-first hierarchically ordered list of `CollectionItem`s.
pub fn build_hierarchical_collection_items(
    db_collections: &[papyrus_core::db::Collection],
    paper_counts: &HashMap<Uuid, usize>,
) -> Vec<CollectionItem> {
    let mut children_by_parent: HashMap<Option<Uuid>, Vec<&papyrus_core::db::Collection>> =
        HashMap::new();
    let known_ids: HashSet<Uuid> = db_collections.iter().map(|c| c.id).collect();

    for col in db_collections {
        let parent_key = match col.parent_id {
            Some(pid) if known_ids.contains(&pid) => Some(pid),
            _ => None,
        };
        children_by_parent.entry(parent_key).or_default().push(col);
    }

    // Sort children alphabetically by name (case-insensitive)
    for children in children_by_parent.values_mut() {
        children.sort_by_key(|a| a.name.to_lowercase());
    }

    let mut result = Vec::new();
    if let Some(roots) = children_by_parent.get(&None) {
        for root in roots {
            append_collection_recursive(root, 0, &children_by_parent, paper_counts, &mut result);
        }
    }

    result
}

fn append_collection_recursive(
    col: &papyrus_core::db::Collection,
    depth: usize,
    children_by_parent: &HashMap<Option<Uuid>, Vec<&papyrus_core::db::Collection>>,
    paper_counts: &HashMap<Uuid, usize>,
    out: &mut Vec<CollectionItem>,
) {
    let count = paper_counts.get(&col.id).copied().unwrap_or(0);
    out.push(CollectionItem::with_hierarchy(
        Some(col.id),
        &col.name,
        count,
        depth,
        col.parent_id,
    ));

    if let Some(children) = children_by_parent.get(&Some(col.id)) {
        for child in children {
            append_collection_recursive(child, depth + 1, children_by_parent, paper_counts, out);
        }
    }
}
