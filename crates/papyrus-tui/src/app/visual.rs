use uuid::Uuid;

use crate::app::{ActivePanel, App};

impl App {
    /// Enters visual selection mode if in the Papers panel and papers are present.
    pub fn enter_visual_mode(&mut self) {
        if self.active_panel == ActivePanel::Papers && !self.papers.is_empty() {
            self.visual_mode = true;
            self.visual_anchor = Some(self.selected_paper);
            self.visual_selected_uuids.clear();
            self.visual_selected_uuids
                .insert(self.papers[self.selected_paper].id);
        }
    }

    /// Exits visual mode, clearing anchor and selection set.
    pub fn exit_visual_mode(&mut self) {
        self.visual_mode = false;
        self.visual_anchor = None;
        self.visual_selected_uuids.clear();
    }

    /// Toggles visual mode on or off.
    pub fn toggle_visual_mode(&mut self) {
        if self.visual_mode {
            self.exit_visual_mode();
        } else {
            self.enter_visual_mode();
        }
    }

    /// Updates the visual selection range to span from anchor to current selected paper.
    pub fn update_visual_range(&mut self) {
        if self.visual_mode {
            if let Some(anchor) = self.visual_anchor {
                let start = anchor.min(self.selected_paper);
                let end = anchor.max(self.selected_paper);
                if let Some(slice) = self.papers.get(start..=end) {
                    for paper in slice {
                        self.visual_selected_uuids.insert(paper.id);
                    }
                }
            }
        }
    }

    /// Toggles selection of the currently highlighted paper in visual mode.
    pub fn toggle_current_paper_selection(&mut self) {
        if self.visual_mode {
            if let Some(paper) = self.papers.get(self.selected_paper) {
                let id = paper.id;
                if self.visual_selected_uuids.contains(&id) {
                    self.visual_selected_uuids.remove(&id);
                } else {
                    self.visual_selected_uuids.insert(id);
                }
            }
        }
    }

    /// Returns paper UUIDs selected in visual mode (preserving list order),
    /// or the single selected paper, or empty if none.
    pub fn visual_selected_papers(&self) -> Vec<Uuid> {
        if self.visual_mode && !self.visual_selected_uuids.is_empty() {
            self.papers
                .iter()
                .filter(|p| self.visual_selected_uuids.contains(&p.id))
                .map(|p| p.id)
                .collect()
        } else if let Some(p) = self.papers.get(self.selected_paper) {
            vec![p.id]
        } else {
            Vec::new()
        }
    }
}
