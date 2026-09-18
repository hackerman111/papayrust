use papyrus_core::metadata_editor::{update_metadata, UpdatePaperMetadata};

use crate::app::App;

impl App {
    /// Opens the metadata editing dialog for the currently selected paper.
    pub fn start_edit_metadata(&mut self) {
        let Some(paper) = self.current_paper() else {
            self.set_status("No paper selected to edit");
            return;
        };

        self.edit_buffers = [
            paper.title.clone().unwrap_or_default(),
            paper.authors.clone().unwrap_or_default(),
            paper.year.map(|y| y.to_string()).unwrap_or_default(),
            paper.journal.clone().unwrap_or_default(),
            paper.doi.clone().unwrap_or_default(),
            paper.abstract_text.clone().unwrap_or_default(),
        ];
        self.editing_field_index = 0;
        self.is_editing_metadata = true;
    }

    /// Cancels metadata editing without saving changes.
    pub fn cancel_edit_metadata(&mut self) {
        self.is_editing_metadata = false;
        self.editing_field_index = 0;
        for buf in &mut self.edit_buffers {
            buf.clear();
        }
    }

    /// Moves focus to the next metadata field (wrapping from 5 to 0).
    pub fn next_edit_field(&mut self) {
        if self.is_editing_metadata {
            self.editing_field_index = (self.editing_field_index + 1) % 6;
        }
    }

    /// Moves focus to the previous metadata field (wrapping from 0 to 5).
    pub fn previous_edit_field(&mut self) {
        if self.is_editing_metadata {
            self.editing_field_index = if self.editing_field_index == 0 {
                5
            } else {
                self.editing_field_index - 1
            };
        }
    }

    /// Appends a character to the currently focused metadata field buffer.
    pub fn push_edit_char(&mut self, c: char) {
        if self.is_editing_metadata && self.editing_field_index < 6 {
            self.edit_buffers[self.editing_field_index].push(c);
        }
    }

    /// Removes the last character from the currently focused metadata field buffer.
    pub fn pop_edit_char(&mut self) {
        if self.is_editing_metadata && self.editing_field_index < 6 {
            self.edit_buffers[self.editing_field_index].pop();
        }
    }

    /// Validates and saves edited metadata to SQLite and Tantivy, then closes the dialog.
    pub fn save_metadata(&mut self) {
        if !self.is_editing_metadata {
            return;
        }

        let Some(current) = self.current_paper().cloned() else {
            self.set_status("No paper selected to edit");
            self.is_editing_metadata = false;
            return;
        };

        let title = self.edit_buffers[0].trim().to_string();
        if title.is_empty() {
            self.set_status("Title cannot be empty");
            return;
        }

        let authors = {
            let s = self.edit_buffers[1].trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        };

        let year = {
            let s = self.edit_buffers[2].trim();
            if s.is_empty() {
                None
            } else {
                match s.parse::<i64>() {
                    Ok(y) => {
                        if !(1000..=3000).contains(&y) {
                            self.set_status("Invalid year");
                            return;
                        }
                        Some(y)
                    }
                    Err(_) => {
                        self.set_status("Invalid year");
                        return;
                    }
                }
            }
        };

        let journal = {
            let s = self.edit_buffers[3].trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        };

        let doi = {
            let s = self.edit_buffers[4].trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        };

        let abstract_text = {
            let s = self.edit_buffers[5].trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        };

        let update = UpdatePaperMetadata {
            title,
            authors,
            year,
            journal,
            doi,
            abstract_text,
        };

        let search_index_ref = self.search_index.as_deref();

        let updated_paper = if let Some(ref mut conn) = self.db_conn {
            match update_metadata(conn, current.id, update, search_index_ref) {
                Ok(p) => p,
                Err(err) => {
                    self.set_status(format!("Error: {err}"));
                    return;
                }
            }
        } else if let Ok(mut conn) = papyrus_core::db::open_database(&self.config.database_path) {
            match update_metadata(&mut conn, current.id, update, search_index_ref) {
                Ok(p) => p,
                Err(err) => {
                    self.set_status(format!("Error: {err}"));
                    return;
                }
            }
        } else {
            let mut p = current.clone();
            p.title = Some(update.title);
            p.authors = update.authors;
            p.year = update.year;
            p.journal = update.journal;
            p.doi = update.doi;
            p.abstract_text = update.abstract_text;
            p.updated_at = papyrus_core::time::current_timestamp_utc();
            if let Some(index) = search_index_ref {
                let _ = index.index_paper(&p, None);
            }
            p
        };

        // Update in self.papers
        if let Some(paper) = self.papers.iter_mut().find(|p| p.id == updated_paper.id) {
            *paper = updated_paper.clone();
        }

        // Update in self.papers_by_collection
        for list in self.papers_by_collection.values_mut() {
            for paper in list.iter_mut() {
                if paper.id == updated_paper.id {
                    *paper = updated_paper.clone();
                }
            }
        }

        // Re-evaluate active search filter
        self.apply_search_filter();

        self.set_status(format!(
            "Updated metadata for {}",
            updated_paper.title.as_deref().unwrap_or("paper")
        ));
        self.is_editing_metadata = false;
    }
}
