pub use papyrus_core::Motion;

use crate::app::{ActivePanel, App};

impl App {
    /// Applies a navigation motion to the currently active panel or fullscreen view.
    pub fn apply_motion(&mut self, motion: Motion) {
        let count = self.pending_count.take().unwrap_or(1);
        self.pending_chord = None;

        if self.is_viewing_fullscreen_toc || self.active_panel == ActivePanel::Details {
            let len = self.toc_preview.len();
            if let Some(new_idx) = calculate_target_index(self.selected_toc, len, motion, count) {
                if new_idx != self.selected_toc {
                    self.selected_toc = new_idx;
                    self.record_position_for_current_paper();
                    self.update_selection_from_indices();
                }
            }
            return;
        }

        match self.active_panel {
            ActivePanel::Collections => {
                let len = self.collections.len();
                if let Some(new_idx) =
                    calculate_target_index(self.selected_collection, len, motion, count)
                {
                    if new_idx != self.selected_collection {
                        self.record_position_for_current_collection();
                        self.selected_collection = new_idx;
                        self.selected_paper = 0;
                        self.selected_toc = 0;
                        self.sync_current_selection();
                    }
                }
            }
            ActivePanel::Papers => {
                let len = self.papers.len();
                if let Some(new_idx) =
                    calculate_target_index(self.selected_paper, len, motion, count)
                {
                    if new_idx != self.selected_paper {
                        self.record_position_for_current_paper();
                        self.selected_paper = new_idx;
                        self.selected_toc = 0;
                        self.sync_paper_selection();
                        self.update_selection_from_indices();
                        if self.visual_mode {
                            self.update_visual_range();
                        }
                    }
                }
            }
            ActivePanel::Details => unreachable!(),
        }
    }

    /// Handles a numeric digit keypress for count prefix accumulation.
    pub fn handle_digit(&mut self, digit: usize) {
        let current = self.pending_count.unwrap_or(0);
        let next = current
            .saturating_mul(10)
            .saturating_add(digit)
            .min(999_999);
        self.pending_count = Some(next);
    }
}

/// Calculates the target 0-based index for a motion, clamped to `[0, len - 1]`.
pub fn calculate_target_index(
    current: usize,
    len: usize,
    motion: Motion,
    count: usize,
) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let last = len.saturating_sub(1);
    let target = match motion {
        Motion::Relative(step) => {
            let total_step = step.saturating_mul(count as isize);
            let next = current as isize + total_step;
            next.clamp(0, last as isize) as usize
        }
        Motion::First => 0,
        Motion::Last => last,
        Motion::Absolute(target_1based) => {
            if target_1based == 0 {
                0
            } else {
                target_1based.saturating_sub(1).min(last)
            }
        }
        Motion::HalfPageDown => {
            let step = count.saturating_mul(10).max(1) as isize;
            let next = current as isize + step;
            next.clamp(0, last as isize) as usize
        }
        Motion::HalfPageUp => {
            let step = count.saturating_mul(10).max(1) as isize;
            let next = current as isize - step;
            next.clamp(0, last as isize) as usize
        }
    };
    Some(target)
}
