use crate::app::App;

impl App {
    /// Starts interactive search mode.
    pub fn start_search(&mut self) {
        self.is_searching = true;
        self.search_collection_index = self.selected_collection;
        self.apply_search_filter();
    }

    /// Cycles the target collection for search filtering.
    pub fn cycle_search_collection(&mut self) {
        if !self.collections.is_empty() {
            self.search_collection_index =
                (self.search_collection_index + 1) % self.collections.len();
            self.apply_search_filter();
        }
    }

    /// Cancels search mode, clears query, and restores unfiltered paper list.
    pub fn cancel_search(&mut self) {
        self.is_searching = false;
        self.search_query.clear();
        self.search_collection_index = self.selected_collection;
        self.sync_current_selection();
    }

    /// Confirms search, exiting input mode while keeping filtered results.
    pub fn confirm_search(&mut self) {
        self.is_searching = false;
    }

    /// Sets the search query string and applies filtering.
    pub fn set_search_query(&mut self, query: impl Into<String>) {
        self.search_query = query.into();
        self.apply_search_filter();
    }

    /// Appends a character to the search query and updates the filter.
    pub fn push_search_char(&mut self, c: char) {
        self.search_query.push(c);
        self.apply_search_filter();
    }

    /// Removes the last character from the search query and updates the filter.
    pub fn pop_search_char(&mut self) {
        self.search_query.pop();
        self.apply_search_filter();
    }

    /// Filters the active papers list based on `search_query` across the selected search collection.
    pub fn apply_search_filter(&mut self) {
        let col_idx = if self.is_searching {
            self.search_collection_index
                .min(self.collections.len().saturating_sub(1))
        } else {
            self.selected_collection
                .min(self.collections.len().saturating_sub(1))
        };
        let key = self.collections.get(col_idx).and_then(|c| c.id);
        let base_papers = self
            .papers_by_collection
            .get(&key)
            .cloned()
            .unwrap_or_default();

        let trimmed = self.search_query.trim();
        if trimmed.is_empty() {
            self.papers = base_papers;
        } else {
            let q_lower = trimmed.to_lowercase();
            let (tag_only, search_term) = if let Some(stripped) = q_lower.strip_prefix('#') {
                (true, stripped.trim())
            } else if let Some(stripped) = q_lower.strip_prefix("tag:") {
                (true, stripped.trim())
            } else {
                (false, q_lower.as_str())
            };

            let index_ranks: std::collections::HashMap<uuid::Uuid, usize> = if !tag_only {
                if let Some(ref index) = self.search_index {
                    index
                        .search(trimmed, 100)
                        .map(|results| {
                            results
                                .into_iter()
                                .enumerate()
                                .map(|(r, res)| (res.id, r))
                                .collect()
                        })
                        .unwrap_or_default()
                } else {
                    std::collections::HashMap::new()
                }
            } else {
                std::collections::HashMap::new()
            };

            let has_index = self.search_index.is_some() && !tag_only;

            let mut matched: Vec<papyrus_core::db::Paper> = base_papers
                .into_iter()
                .filter(|p| {
                    let matches_tag = self.tags_by_paper.get(&p.id).is_some_and(|tags| {
                        tags.iter().any(|t| t.to_lowercase().contains(search_term))
                    });

                    if tag_only {
                        matches_tag
                    } else if has_index {
                        index_ranks.contains_key(&p.id)
                            || p.title
                                .as_deref()
                                .unwrap_or("")
                                .to_lowercase()
                                .contains(search_term)
                            || matches_tag
                    } else {
                        matches_tag
                            || p.title
                                .as_deref()
                                .unwrap_or("")
                                .to_lowercase()
                                .contains(search_term)
                            || p.authors
                                .as_deref()
                                .unwrap_or("")
                                .to_lowercase()
                                .contains(search_term)
                            || p.abstract_text
                                .as_deref()
                                .unwrap_or("")
                                .to_lowercase()
                                .contains(search_term)
                            || p.year
                                .map(|y| y.to_string())
                                .unwrap_or_default()
                                .contains(search_term)
                    }
                })
                .collect();

            if has_index && !index_ranks.is_empty() {
                matched.sort_by_key(|p| index_ranks.get(&p.id).copied().unwrap_or(usize::MAX));
            }

            self.papers = matched;
        }

        if self.selected_paper >= self.papers.len() {
            self.selected_paper = 0;
        }
        self.sync_paper_selection();
    }
}
