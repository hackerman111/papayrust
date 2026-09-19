use crate::app::App;

impl App {
    /// Confirms tag editing, saving the tags into SQLite (or in-memory) and updating state.
    pub(crate) fn confirm_edit_tags(&mut self) {
        let tags: Vec<String> = self
            .tags_input_buffer
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if let Some(paper) = self.current_paper().cloned() {
            if let Some(ref conn) = self.db_conn {
                match papyrus_core::db::TagRepo::set_tags_for_paper(conn, paper.id, &tags) {
                    Ok(()) => {
                        self.tags_by_paper.insert(paper.id, tags);
                        let title = paper.title.as_deref().unwrap_or("paper");
                        self.set_status(format!("Updated tags for '{title}'"));
                    }
                    Err(err) => {
                        self.set_status(format!("Failed to save tags: {err}"));
                    }
                }
            } else {
                self.tags_by_paper.insert(paper.id, tags);
                self.set_status("Updated tags (in-memory)");
            }
        }
        self.is_editing_tags = false;
        self.tags_input_buffer.clear();
        self.needs_clear = true;
    }
}
