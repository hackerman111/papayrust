use crate::app::{ActivePanel, App};

impl App {
    /// Prepares delete confirmation description based on the active panel.
    pub(crate) fn open_delete_confirm(&mut self) {
        match self.active_panel {
            ActivePanel::Collections => {
                if self.collections.is_empty() {
                    self.set_status("No collection to delete");
                    return;
                }
                let col = &self.collections[self.selected_collection];
                if col.id.is_none() {
                    self.set_status("Cannot delete 'All Papers' virtual collection");
                    return;
                }
                self.delete_target_description = format!("Collection '{}'", col.name);
                self.is_confirming_delete = true;
                self.needs_clear = true;
            }
            ActivePanel::Papers => {
                if let Some(paper) = self.current_paper() {
                    let title = paper.title.as_deref().unwrap_or(&paper.file_path);
                    self.delete_target_description = format!("Paper '{title}'");
                    self.is_confirming_delete = true;
                    self.needs_clear = true;
                } else {
                    self.set_status("No paper to delete");
                }
            }
            ActivePanel::Details => {}
        }
    }

    /// Executes deletion of the selected collection or paper.
    pub(crate) fn execute_delete(&mut self) {
        match self.active_panel {
            ActivePanel::Collections => {
                if let Some(col_id) = self
                    .collections
                    .get(self.selected_collection)
                    .and_then(|c| c.id)
                {
                    let col_name = self
                        .collections
                        .get(self.selected_collection)
                        .map(|c| c.name.clone())
                        .unwrap_or_default();

                    self.last_paper_by_collection
                        .remove(&crate::app::CollectionKey::Real(col_id));
                    if let Some(ref conn) = self.db_conn {
                        match papyrus_core::db::CollectionRepo::delete(conn, col_id) {
                            Ok(_) => {
                                let _ = self.reload_from_db();
                                self.set_status(format!("Deleted collection '{col_name}'"));
                            }
                            Err(err) => {
                                self.set_status(format!("Failed to delete collection: {err}"));
                            }
                        }
                    } else {
                        self.collections.retain(|c| c.id != Some(col_id));
                        self.papers_by_collection.remove(&Some(col_id));
                        self.selected_collection = self
                            .selected_collection
                            .min(self.collections.len().saturating_sub(1));
                        self.sync_current_selection();
                        self.set_status(format!("Deleted collection '{col_name}'"));
                    }
                }
            }
            ActivePanel::Papers => {
                if let Some(paper) = self.current_paper().cloned() {
                    let title = paper.title.as_deref().unwrap_or("paper").to_string();
                    self.last_paper_by_collection
                        .retain(|_, pid| *pid != paper.id);
                    self.last_toc_by_paper.remove(&paper.id);
                    if let Some(ref conn) = self.db_conn {
                        match papyrus_core::db::PaperRepo::delete(conn, paper.id) {
                            Ok(_) => {
                                if let Some(ref index) = self.search_index {
                                    let _ = index.remove_paper(paper.id);
                                }
                                let _ = self.reload_from_db();
                                self.set_status(format!("Deleted paper '{title}'"));
                            }
                            Err(err) => {
                                self.set_status(format!("Failed to delete paper: {err}"));
                            }
                        }
                    } else {
                        for list in self.papers_by_collection.values_mut() {
                            list.retain(|p| p.id != paper.id);
                        }
                        self.tags_by_paper.remove(&paper.id);
                        self.tocs_by_paper.remove(&paper.id);
                        self.sync_current_selection();
                        self.set_status(format!("Deleted paper '{title}'"));
                    }
                }
            }
            ActivePanel::Details => {}
        }
        self.is_confirming_delete = false;
        self.delete_target_description.clear();
        self.needs_clear = true;
    }

    /// Removes the currently selected paper from the active real collection.
    pub(crate) fn remove_current_paper_from_collection(&mut self) {
        let Some(col) = self.current_collection() else {
            self.set_status("No collection selected");
            return;
        };
        let Some(col_id) = col.id else {
            self.set_status("Cannot remove from 'All Papers'; press 'D' to delete from database");
            return;
        };
        let col_name = col.name.clone();

        let Some(paper) = self.current_paper().cloned() else {
            self.set_status("No paper selected");
            return;
        };

        if let Some(ref mut conn) = self.db_conn {
            match papyrus_core::db::CollectionRepo::remove_paper(conn, paper.id, col_id) {
                Ok(_) => {
                    let _ = self.reload_from_db();
                    let title = paper.title.as_deref().unwrap_or(&paper.file_path);
                    self.set_status(format!("Removed '{title}' from collection '{col_name}'"));
                }
                Err(err) => {
                    self.set_status(format!("Failed to remove paper from collection: {err}"));
                }
            }
        } else {
            // In-memory fallback
            if let Some(papers) = self.papers_by_collection.get_mut(&Some(col_id)) {
                papers.retain(|p| p.id != paper.id);
            }
            self.sync_current_selection();
            self.set_status(format!("Removed paper from collection '{col_name}'"));
        }
    }

    /// Batch removes all visual selected papers from the active real collection.
    pub(crate) fn batch_remove_papers_from_collection(&mut self) {
        let Some(col) = self.current_collection() else {
            self.set_status("No collection selected");
            return;
        };
        let Some(col_id) = col.id else {
            self.set_status("Cannot remove from 'All Papers'; press 'D' to delete from database");
            return;
        };
        let col_name = col.name.clone();

        let target_papers = self.visual_selected_papers();
        if target_papers.is_empty() {
            self.set_status("No papers selected");
            return;
        }

        let count = target_papers.len();
        if let Some(ref mut conn) = self.db_conn {
            match crate::app::batch_set_collections(conn, &target_papers, &[], &[col_id]) {
                Ok(_) => {
                    let _ = self.reload_from_db();
                    self.exit_visual_mode();
                    self.set_status(format!(
                        "Removed {count} papers from collection '{col_name}'"
                    ));
                }
                Err(err) => {
                    self.set_status(format!("Failed to remove papers from collection: {err}"));
                }
            }
        } else {
            if let Some(papers) = self.papers_by_collection.get_mut(&Some(col_id)) {
                papers.retain(|p| !target_papers.contains(&p.id));
            }
            self.sync_current_selection();
            self.exit_visual_mode();
            self.set_status(format!(
                "Removed {count} papers from collection '{col_name}'"
            ));
        }
    }
}
