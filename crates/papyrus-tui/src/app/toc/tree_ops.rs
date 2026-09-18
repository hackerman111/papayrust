use uuid::Uuid;

use crate::app::App;
use papyrus_core::db::{PaperRepo, TocRepo};
use papyrus_core::toc::{
    delete_entry, embed_toc_in_pdf, indent, is_annotated_copy_outdated, move_down, move_up, outdent,
};

impl App {
    /// Checks if the currently selected paper has an outdated annotated PDF copy.
    pub fn is_current_paper_annotated_outdated(&self) -> bool {
        let Some(paper) = self.current_paper() else {
            return false;
        };
        if paper.annotated_pdf_path.is_none() || paper.toc_embedded_at.is_none() {
            return false;
        }
        is_annotated_copy_outdated(paper)
    }

    /// Deletes the selected TOC entry and its subtree.
    pub fn delete_selected_toc(&mut self, id: Uuid) {
        if let Some(ref mut conn) = self.db_conn {
            match delete_entry(conn, id) {
                Ok(()) => {
                    self.reload_current_paper_and_tocs();
                    self.set_status("Deleted TOC entry and subtree");
                }
                Err(err) => {
                    self.set_status(format!("Failed to delete TOC entry: {err}"));
                }
            }
        } else if let Ok(mut conn) = papyrus_core::db::open_database(&self.config.database_path) {
            match delete_entry(&mut conn, id) {
                Ok(()) => {
                    self.reload_current_paper_and_tocs();
                    self.set_status("Deleted TOC entry and subtree");
                }
                Err(err) => {
                    self.set_status(format!("Failed to delete TOC entry: {err}"));
                }
            }
        } else {
            // In-memory fallback: cascade delete
            let mut ids_to_delete = vec![id];
            let mut idx = 0;
            while idx < ids_to_delete.len() {
                let parent = ids_to_delete[idx];
                for e in &self.toc_preview {
                    if e.parent_id == Some(parent) && !ids_to_delete.contains(&e.id) {
                        ids_to_delete.push(e.id);
                    }
                }
                idx += 1;
            }
            self.toc_preview.retain(|e| !ids_to_delete.contains(&e.id));
            if let Some(paper) = self.current_paper() {
                self.tocs_by_paper
                    .insert(paper.id, self.toc_preview.clone());
            }
            if self.selected_toc >= self.toc_preview.len() {
                self.selected_toc = self.toc_preview.len().saturating_sub(1);
            }
            self.set_status("Deleted TOC entry");
        }
    }

    /// Indents the specified TOC entry.
    pub fn indent_selected_toc(&mut self, id: Uuid) {
        if let Some(ref mut conn) = self.db_conn {
            match indent(conn, id) {
                Ok(_) => {
                    self.reload_current_paper_and_tocs();
                    self.set_status("Indented TOC entry");
                }
                Err(err) => {
                    self.set_status(format!("Failed to indent: {err}"));
                }
            }
        } else if let Ok(mut conn) = papyrus_core::db::open_database(&self.config.database_path) {
            match indent(&mut conn, id) {
                Ok(_) => {
                    self.reload_current_paper_and_tocs();
                    self.set_status("Indented TOC entry");
                }
                Err(err) => {
                    self.set_status(format!("Failed to indent: {err}"));
                }
            }
        }
    }

    /// Outdents the specified TOC entry.
    pub fn outdent_selected_toc(&mut self, id: Uuid) {
        if let Some(ref mut conn) = self.db_conn {
            match outdent(conn, id) {
                Ok(_) => {
                    self.reload_current_paper_and_tocs();
                    self.set_status("Outdented TOC entry");
                }
                Err(err) => {
                    self.set_status(format!("Failed to outdent: {err}"));
                }
            }
        } else if let Ok(mut conn) = papyrus_core::db::open_database(&self.config.database_path) {
            match outdent(&mut conn, id) {
                Ok(_) => {
                    self.reload_current_paper_and_tocs();
                    self.set_status("Outdented TOC entry");
                }
                Err(err) => {
                    self.set_status(format!("Failed to outdent: {err}"));
                }
            }
        }
    }

    /// Moves the specified TOC entry up among its siblings.
    pub fn move_up_selected_toc(&mut self, id: Uuid) {
        if let Some(ref mut conn) = self.db_conn {
            match move_up(conn, id) {
                Ok(_) => {
                    self.reload_current_paper_and_tocs();
                    self.set_status("Moved TOC entry up");
                }
                Err(err) => {
                    self.set_status(format!("Failed to move up: {err}"));
                }
            }
        } else if let Ok(mut conn) = papyrus_core::db::open_database(&self.config.database_path) {
            match move_up(&mut conn, id) {
                Ok(_) => {
                    self.reload_current_paper_and_tocs();
                    self.set_status("Moved TOC entry up");
                }
                Err(err) => {
                    self.set_status(format!("Failed to move up: {err}"));
                }
            }
        }
    }

    /// Moves the specified TOC entry down among its siblings.
    pub fn move_down_selected_toc(&mut self, id: Uuid) {
        if let Some(ref mut conn) = self.db_conn {
            match move_down(conn, id) {
                Ok(_) => {
                    self.reload_current_paper_and_tocs();
                    self.set_status("Moved TOC entry down");
                }
                Err(err) => {
                    self.set_status(format!("Failed to move down: {err}"));
                }
            }
        } else if let Ok(mut conn) = papyrus_core::db::open_database(&self.config.database_path) {
            match move_down(&mut conn, id) {
                Ok(_) => {
                    self.reload_current_paper_and_tocs();
                    self.set_status("Moved TOC entry down");
                }
                Err(err) => {
                    self.set_status(format!("Failed to move down: {err}"));
                }
            }
        }
    }

    /// Materializes TOC into annotated PDF copy for current paper.
    pub fn embed_selected_toc(&mut self) {
        let Some(paper) = self.current_paper().cloned() else {
            self.set_status("No paper selected to embed TOC");
            return;
        };

        let res = if let Some(ref mut conn) = self.db_conn {
            embed_toc_in_pdf(conn, &self.config, paper.id)
        } else if let Ok(mut conn) = papyrus_core::db::open_database(&self.config.database_path) {
            embed_toc_in_pdf(&mut conn, &self.config, paper.id)
        } else {
            self.set_status("No database connection available");
            return;
        };

        match res {
            Ok(annotated_path) => {
                self.reload_current_paper_and_tocs();
                self.set_status(format!(
                    "Embedded TOC in annotated PDF ({})",
                    annotated_path.display()
                ));
            }
            Err(err) => {
                self.set_status(format!("Failed to embed TOC: {err}"));
            }
        }
    }

    /// Helper to reload the current paper metadata and its TOC list from DB into App.
    pub fn reload_current_paper_and_tocs(&mut self) {
        let Some(pid) = self.current_paper().map(|p| p.id) else {
            return;
        };

        if let Some(ref conn) = self.db_conn {
            if let Ok(Some(paper)) = PaperRepo::get_by_id(conn, pid) {
                if let Some(p) = self.papers.iter_mut().find(|p| p.id == pid) {
                    *p = paper.clone();
                }
                for list in self.papers_by_collection.values_mut() {
                    for p in list.iter_mut() {
                        if p.id == pid {
                            *p = paper.clone();
                        }
                    }
                }
            }
            if let Ok(tocs) = TocRepo::get_by_paper(conn, pid) {
                self.tocs_by_paper.insert(pid, tocs);
                self.sync_paper_selection();
            }
        } else if let Ok(conn) = papyrus_core::db::open_database(&self.config.database_path) {
            if let Ok(Some(paper)) = PaperRepo::get_by_id(&conn, pid) {
                if let Some(p) = self.papers.iter_mut().find(|p| p.id == pid) {
                    *p = paper.clone();
                }
                for list in self.papers_by_collection.values_mut() {
                    for p in list.iter_mut() {
                        if p.id == pid {
                            *p = paper.clone();
                        }
                    }
                }
            }
            if let Ok(tocs) = TocRepo::get_by_paper(&conn, pid) {
                self.tocs_by_paper.insert(pid, tocs);
                self.sync_paper_selection();
            }
        }
    }
}
