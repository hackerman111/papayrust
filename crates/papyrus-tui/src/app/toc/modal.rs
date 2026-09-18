use std::path::PathBuf;
use uuid::Uuid;

use crate::app::App;
use papyrus_core::db::{TocEntry, TocSource};
use papyrus_core::time::current_timestamp_utc;
use papyrus_core::toc::{add_entry, edit_entry, import_toc, PendingTocEntry};
use papyrus_core::TocImportSource;

/// State of the TOC item add/edit modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocEditState {
    /// ID of entry being edited, or None if adding new entry.
    pub entry_id: Option<Uuid>,
    /// Parent ID under which this entry will be created/edited.
    pub parent_id: Option<Uuid>,
    /// Title text buffer.
    pub title_buffer: String,
    /// Page number text buffer.
    pub page_buffer: String,
    /// Currently focused field (0 = title, 1 = page).
    pub active_field: usize,
}

/// Source type selector in TOC import modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TocImportSourceType {
    #[default]
    PdfOutline,
    TextFile,
    JsonFile,
}

impl TocImportSourceType {
    pub fn next(self) -> Self {
        match self {
            Self::PdfOutline => Self::TextFile,
            Self::TextFile => Self::JsonFile,
            Self::JsonFile => Self::PdfOutline,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::PdfOutline => Self::JsonFile,
            Self::TextFile => Self::PdfOutline,
            Self::JsonFile => Self::TextFile,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::PdfOutline => "PDF Outline (internal)",
            Self::TextFile => "Indented Text File",
            Self::JsonFile => "JSON File",
        }
    }
}

/// State of the TOC import modal dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocImportState {
    /// Selected import source.
    pub source_type: TocImportSourceType,
    /// Path buffer for Text/JSON import.
    pub file_path_buffer: String,
    /// Whether to merge into existing TOC or replace it.
    pub merge_mode: bool,
    /// Currently focused field (0 = source_type, 1 = file_path, 2 = merge_mode).
    pub active_field: usize,
    /// Error message if last import attempt failed.
    pub error_message: Option<String>,
    /// Optional preview of entries to be imported.
    pub preview_entries: Vec<PendingTocEntry>,
}

impl App {
    /// Returns true if the TOC add/edit modal dialog is open.
    pub fn is_editing_toc(&self) -> bool {
        self.toc_edit_state.is_some()
    }

    /// Returns true if the TOC import modal dialog is open.
    pub fn is_importing_toc(&self) -> bool {
        self.toc_import_state.is_some()
    }

    /// Opens the modal to add a new TOC entry.
    pub fn start_add_toc(&mut self, parent_id: Option<Uuid>) {
        if self.current_paper().is_none() {
            self.set_status("No paper selected");
            return;
        }

        let default_page = self
            .current_toc()
            .map(|t| t.page_number)
            .unwrap_or(1)
            .to_string();

        self.toc_edit_state = Some(TocEditState {
            entry_id: None,
            parent_id,
            title_buffer: String::new(),
            page_buffer: default_page,
            active_field: 0,
        });
    }

    /// Opens the modal to edit the specified TOC entry.
    pub fn start_edit_toc(&mut self, entry_id: Uuid) {
        let Some(entry) = self.toc_preview.iter().find(|t| t.id == entry_id).cloned() else {
            self.set_status("TOC entry not found");
            return;
        };

        self.toc_edit_state = Some(TocEditState {
            entry_id: Some(entry.id),
            parent_id: entry.parent_id,
            title_buffer: entry.title,
            page_buffer: entry.page_number.to_string(),
            active_field: 0,
        });
    }

    /// Cancels TOC add/edit modal.
    pub fn cancel_toc_modal(&mut self) {
        self.toc_edit_state = None;
    }

    /// Cycles to the next field in the TOC edit modal.
    pub fn toc_edit_next_field(&mut self) {
        if let Some(ref mut state) = self.toc_edit_state {
            state.active_field = (state.active_field + 1) % 2;
        }
    }

    /// Cycles to the previous field in the TOC edit modal.
    pub fn toc_edit_prev_field(&mut self) {
        if let Some(ref mut state) = self.toc_edit_state {
            state.active_field = if state.active_field == 0 { 1 } else { 0 };
        }
    }

    /// Pushes a character to the active field of the TOC edit modal.
    pub fn toc_edit_push_char(&mut self, c: char) {
        if let Some(ref mut state) = self.toc_edit_state {
            match state.active_field {
                0 => state.title_buffer.push(c),
                1 if c.is_ascii_digit() => {
                    state.page_buffer.push(c);
                }
                _ => {}
            }
        }
    }

    /// Removes a character from the active field of the TOC edit modal.
    pub fn toc_edit_pop_char(&mut self) {
        if let Some(ref mut state) = self.toc_edit_state {
            match state.active_field {
                0 => {
                    state.title_buffer.pop();
                }
                1 => {
                    state.page_buffer.pop();
                }
                _ => {}
            }
        }
    }

    /// Saves the TOC entry (add or edit) and closes the modal.
    pub fn save_toc_modal(&mut self) {
        let Some(state) = self.toc_edit_state.clone() else {
            return;
        };
        let Some(paper) = self.current_paper().cloned() else {
            self.set_status("No paper selected");
            self.toc_edit_state = None;
            return;
        };

        let title = state.title_buffer.trim().to_string();
        if title.is_empty() {
            self.set_status("Title cannot be empty");
            return;
        }

        let page_num: u32 = match state.page_buffer.trim().parse() {
            Ok(p) if p > 0 => p,
            _ => {
                self.set_status("Page number must be at least 1");
                return;
            }
        };

        let saved_entry_id;

        if let Some(ref mut conn) = self.db_conn {
            match state.entry_id {
                None => {
                    match add_entry(
                        conn,
                        paper.id,
                        state.parent_id,
                        title,
                        page_num,
                        TocSource::Manual,
                    ) {
                        Ok(entry) => {
                            saved_entry_id = Some(entry.id);
                            self.set_status(format!("Added TOC entry '{}'", entry.title));
                        }
                        Err(err) => {
                            self.set_status(format!("Error adding TOC entry: {err}"));
                            return;
                        }
                    }
                }
                Some(id) => match edit_entry(conn, id, Some(title), Some(page_num)) {
                    Ok(entry) => {
                        saved_entry_id = Some(entry.id);
                        self.set_status(format!("Updated TOC entry '{}'", entry.title));
                    }
                    Err(err) => {
                        self.set_status(format!("Error updating TOC entry: {err}"));
                        return;
                    }
                },
            }
        } else if let Ok(mut conn) = papyrus_core::db::open_database(&self.config.database_path) {
            match state.entry_id {
                None => {
                    match add_entry(
                        &mut conn,
                        paper.id,
                        state.parent_id,
                        title,
                        page_num,
                        TocSource::Manual,
                    ) {
                        Ok(entry) => {
                            saved_entry_id = Some(entry.id);
                            self.set_status(format!("Added TOC entry '{}'", entry.title));
                        }
                        Err(err) => {
                            self.set_status(format!("Error adding TOC entry: {err}"));
                            return;
                        }
                    }
                }
                Some(id) => match edit_entry(&mut conn, id, Some(title), Some(page_num)) {
                    Ok(entry) => {
                        saved_entry_id = Some(entry.id);
                        self.set_status(format!("Updated TOC entry '{}'", entry.title));
                    }
                    Err(err) => {
                        self.set_status(format!("Error updating TOC entry: {err}"));
                        return;
                    }
                },
            }
        } else {
            // In-memory fallback
            let now = current_timestamp_utc();
            match state.entry_id {
                None => {
                    let entry = TocEntry {
                        id: Uuid::now_v7(),
                        paper_id: paper.id,
                        parent_id: state.parent_id,
                        title,
                        page_number: page_num,
                        order_index: self.toc_preview.len() as i32,
                        source: TocSource::Manual,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                    };
                    saved_entry_id = Some(entry.id);
                    self.toc_preview.push(entry);
                }
                Some(id) => {
                    if let Some(entry) = self.toc_preview.iter_mut().find(|t| t.id == id) {
                        entry.title = title;
                        entry.page_number = page_num;
                        entry.updated_at = now.clone();
                    }
                    saved_entry_id = Some(id);
                }
            }
            if let Some(p) = self.papers.iter_mut().find(|p| p.id == paper.id) {
                p.updated_at = now;
            }
        }

        self.reload_current_paper_and_tocs();
        if let Some(target_id) = saved_entry_id {
            if let Some(idx) = self.toc_preview.iter().position(|t| t.id == target_id) {
                self.selected_toc = idx;
            }
        }

        self.toc_edit_state = None;
    }

    /// Opens the TOC import modal dialog.
    pub fn start_import_toc(&mut self) {
        if self.current_paper().is_none() {
            self.set_status("No paper selected");
            return;
        }

        self.toc_import_state = Some(TocImportState {
            source_type: TocImportSourceType::PdfOutline,
            file_path_buffer: String::new(),
            merge_mode: false,
            active_field: 0,
            error_message: None,
            preview_entries: Vec::new(),
        });
    }

    /// Cancels TOC import modal.
    pub fn cancel_import_toc(&mut self) {
        self.toc_import_state = None;
    }

    /// Toggles the source type in TOC import modal.
    pub fn toc_import_toggle_source(&mut self) {
        if let Some(ref mut state) = self.toc_import_state {
            state.source_type = state.source_type.next();
            state.error_message = None;
        }
    }

    /// Toggles merge mode in TOC import modal.
    pub fn toc_import_toggle_merge(&mut self) {
        if let Some(ref mut state) = self.toc_import_state {
            state.merge_mode = !state.merge_mode;
        }
    }

    /// Moves to the next field in TOC import modal.
    pub fn toc_import_next_field(&mut self) {
        if let Some(ref mut state) = self.toc_import_state {
            state.active_field = (state.active_field + 1) % 3;
        }
    }

    /// Moves to the previous field in TOC import modal.
    pub fn toc_import_prev_field(&mut self) {
        if let Some(ref mut state) = self.toc_import_state {
            state.active_field = if state.active_field == 0 {
                2
            } else {
                state.active_field - 1
            };
        }
    }

    /// Pushes a character to the file path in TOC import modal.
    pub fn toc_import_push_char(&mut self, c: char) {
        if let Some(ref mut state) = self.toc_import_state {
            state.file_path_buffer.push(c);
        }
    }

    /// Removes a character from the file path in TOC import modal.
    pub fn toc_import_pop_char(&mut self) {
        if let Some(ref mut state) = self.toc_import_state {
            state.file_path_buffer.pop();
        }
    }

    /// Confirms TOC import from the selected source.
    pub fn confirm_import_toc(&mut self) {
        let Some(state) = self.toc_import_state.clone() else {
            return;
        };
        let Some(paper) = self.current_paper().cloned() else {
            self.set_status("No paper selected");
            self.toc_import_state = None;
            return;
        };

        let source = match state.source_type {
            TocImportSourceType::PdfOutline => TocImportSource::PdfOutline,
            TocImportSourceType::TextFile => {
                let p = state.file_path_buffer.trim();
                if p.is_empty() {
                    if let Some(ref mut st) = self.toc_import_state {
                        st.error_message = Some("Please specify text file path".to_string());
                    }
                    return;
                }
                TocImportSource::TextFile(PathBuf::from(p))
            }
            TocImportSourceType::JsonFile => {
                let p = state.file_path_buffer.trim();
                if p.is_empty() {
                    if let Some(ref mut st) = self.toc_import_state {
                        st.error_message = Some("Please specify JSON file path".to_string());
                    }
                    return;
                }
                TocImportSource::JsonFile(PathBuf::from(p))
            }
        };

        let pdf_path = PathBuf::from(&paper.file_path);

        let res = if let Some(ref mut conn) = self.db_conn {
            import_toc(conn, paper.id, &pdf_path, &source, state.merge_mode)
        } else if let Ok(mut conn) = papyrus_core::db::open_database(&self.config.database_path) {
            import_toc(&mut conn, paper.id, &pdf_path, &source, state.merge_mode)
        } else {
            if let Some(ref mut st) = self.toc_import_state {
                st.error_message = Some("No database connection available".to_string());
            }
            return;
        };

        match res {
            Ok(imported) => {
                self.reload_current_paper_and_tocs();
                self.set_status(format!("Imported {} TOC entries", imported.len()));
                self.toc_import_state = None;
            }
            Err(err) => {
                let msg = format!("{err}");
                if let Some(ref mut st) = self.toc_import_state {
                    st.error_message = Some(msg.clone());
                }
                self.set_status(format!("Import failed: {msg}"));
            }
        }
    }
}
