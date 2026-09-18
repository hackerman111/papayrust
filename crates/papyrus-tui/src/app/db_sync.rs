use rusqlite::Connection;
use std::collections::HashMap;

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
        let prev_collection_idx = self.selected_collection;
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

            collections.push(CollectionItem::new(None, "All Papers", all_papers.len()));
            papers_by_collection.insert(None, all_papers);

            for col in db_collections {
                let papers = CollectionRepo::get_papers(conn, col.id)?;
                collections.push(CollectionItem::new(Some(col.id), col.name, papers.len()));
                papers_by_collection.insert(Some(col.id), papers);
            }

            self.collections = collections;
            self.papers_by_collection = papers_by_collection;
            self.tocs_by_paper = tocs_by_paper;
            self.tags_by_paper = tags_by_paper;
            self.selected_collection =
                prev_collection_idx.min(self.collections.len().saturating_sub(1));
            self.sync_current_selection();
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
            collections.push(CollectionItem::new(None, "All Papers", all_papers.len()));
            papers_by_collection.insert(None, all_papers);
        }

        // Populate database collections
        for col in db_collections {
            let papers = CollectionRepo::get_papers(conn, col.id)?;
            collections.push(CollectionItem::new(Some(col.id), col.name, papers.len()));
            papers_by_collection.insert(Some(col.id), papers);
        }

        self.collections = collections;
        self.papers_by_collection = papers_by_collection;
        self.tocs_by_paper = tocs_by_paper;
        self.tags_by_paper = tags_by_paper;
        self.active_panel = ActivePanel::Papers;
        self.selected_collection = 0;
        self.selected_paper = 0;
        self.selected_toc = 0;
        self.sync_current_selection();

        Ok(())
    }

    /// Synchronizes the papers list for the currently selected collection.
    pub fn sync_current_selection(&mut self) {
        if self.collections.is_empty() {
            self.selected_collection = 0;
            self.papers.clear();
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
        }
        self.selected_toc = 0;
        self.sync_paper_selection();
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
            if self.toc_preview.is_empty() {
                self.selected_toc = 0;
            } else if self.selected_toc >= self.toc_preview.len() {
                self.selected_toc = self.toc_preview.len() - 1;
            }
        }
    }
}
