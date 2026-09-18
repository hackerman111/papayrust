use papyrus_core::opener;
use papyrus_core::Action;
use std::path::PathBuf;

use crate::app::autocomplete::autocomplete_path;
use crate::app::{ActivePanel, App};

impl App {
    /// Dispatches an action, updating application state accordingly.
    pub fn dispatch(&mut self, action: Action) {
        match &action {
            Action::CountDigit(_)
            | Action::PendingChord(_)
            | Action::Motion(_)
            | Action::MoveUp
            | Action::MoveDown
            | Action::ResetNavigationState => {}
            _ => {
                self.pending_chord = None;
                self.pending_count = None;
            }
        }

        match action {
            Action::NextPanel => {
                self.record_position_for_current_collection();
                self.record_position_for_current_paper();
                self.active_panel = self.active_panel.next();
            }
            Action::PreviousPanel => {
                self.record_position_for_current_collection();
                self.record_position_for_current_paper();
                self.active_panel = self.active_panel.previous();
            }
            Action::PanelLeft => {
                self.record_position_for_current_collection();
                self.record_position_for_current_paper();
                self.active_panel = self.active_panel.left();
            }
            Action::PanelRight => {
                self.record_position_for_current_collection();
                self.record_position_for_current_paper();
                self.active_panel = self.active_panel.right();
            }
            Action::FocusPapers => {
                self.record_position_for_current_collection();
                self.record_position_for_current_paper();
                self.active_panel = ActivePanel::Papers;
            }
            Action::ToggleLayoutMode => {
                self.layout_mode = match self.layout_mode {
                    crate::app::LayoutMode::MultiPanel => crate::app::LayoutMode::SinglePanel,
                    crate::app::LayoutMode::SinglePanel => crate::app::LayoutMode::MultiPanel,
                };
                self.needs_clear = true;
            }
            Action::CyclePaperSort => {
                self.cycle_paper_sort();
            }
            Action::Motion(motion) => {
                self.apply_motion(motion);
            }
            Action::CountDigit(d) => {
                self.handle_digit(d);
            }
            Action::PendingChord(c) => {
                self.pending_chord = Some(c);
            }
            Action::ResetNavigationState => {
                self.pending_count = None;
                self.pending_chord = None;
            }
            Action::MoveUp => {
                self.apply_motion(papyrus_core::Motion::Relative(-1));
            }
            Action::MoveDown => {
                self.apply_motion(papyrus_core::Motion::Relative(1));
            }
            Action::Open => {
                let Some(paper) = self.current_paper() else {
                    self.set_status("No paper selected to open");
                    return;
                };
                match opener::open_paper(paper, None, &self.config, &self.runner) {
                    Ok(()) => {
                        let name = paper.title.as_deref().unwrap_or(&paper.file_path);
                        self.set_status(format!("Opened {name}"));
                    }
                    Err(err) => {
                        self.set_status(format!("Failed to open: {err}"));
                    }
                }
            }
            Action::OpenAtPage(page) => {
                let Some(paper) = self.current_paper() else {
                    self.set_status("No paper selected to open");
                    return;
                };
                match opener::open_paper(paper, Some(page), &self.config, &self.runner) {
                    Ok(()) => {
                        let name = paper.title.as_deref().unwrap_or(&paper.file_path);
                        self.set_status(format!("Opened {name} at page {page}"));
                    }
                    Err(err) => {
                        self.set_status(format!("Failed to open at page {page}: {err}"));
                    }
                }
            }
            Action::Search => {
                self.is_searching = !self.is_searching;
            }
            Action::SearchInput(c) if self.is_searching => {
                self.push_search_char(c);
            }
            Action::SearchBackspace if self.is_searching => {
                self.pop_search_char();
            }
            Action::SearchConfirm => {
                self.confirm_search();
            }
            Action::SearchCancel => {
                self.cancel_search();
            }
            Action::EditMetadata => {
                self.start_edit_metadata();
            }
            Action::EditMetadataNextField if self.is_editing_metadata => {
                self.next_edit_field();
            }
            Action::EditMetadataPrevField if self.is_editing_metadata => {
                self.previous_edit_field();
            }
            Action::EditMetadataInput(c) if self.is_editing_metadata => {
                self.push_edit_char(c);
            }
            Action::EditMetadataBackspace if self.is_editing_metadata => {
                self.pop_edit_char();
            }
            Action::EditMetadataSave if self.is_editing_metadata => {
                self.save_metadata();
            }
            Action::EditMetadataCancel if self.is_editing_metadata => {
                self.cancel_edit_metadata();
            }

            // TOC actions
            Action::OpenToc | Action::TocImportModalOpen => {
                self.start_import_toc();
            }
            Action::TocAddEntry { parent_id } => {
                self.start_add_toc(parent_id);
            }
            Action::TocEditEntry { id } => {
                self.start_edit_toc(id);
            }
            Action::TocDeleteEntry { id } => {
                self.delete_selected_toc(id);
            }
            Action::TocIndent { id } => {
                self.indent_selected_toc(id);
            }
            Action::TocOutdent { id } => {
                self.outdent_selected_toc(id);
            }
            Action::TocMoveUp { id } => {
                self.move_up_selected_toc(id);
            }
            Action::TocMoveDown { id } => {
                self.move_down_selected_toc(id);
            }
            Action::TocEmbed => {
                self.embed_selected_toc();
            }
            Action::TocImport { source } => {
                if let Some(paper) = self.current_paper().cloned() {
                    let pdf_path = PathBuf::from(&paper.file_path);
                    if let Some(ref mut conn) = self.db_conn {
                        match papyrus_core::toc::import_toc(
                            conn, paper.id, &pdf_path, &source, false,
                        ) {
                            Ok(entries) => {
                                self.reload_current_paper_and_tocs();
                                self.set_status(format!("Imported {} TOC entries", entries.len()));
                            }
                            Err(err) => {
                                self.set_status(format!("Failed to import TOC: {err}"));
                            }
                        }
                    }
                }
            }

            // TOC Edit Modal actions
            Action::TocEditModalNextField => self.toc_edit_next_field(),
            Action::TocEditModalPrevField => self.toc_edit_prev_field(),
            Action::TocEditModalInput(c) => self.toc_edit_push_char(c),
            Action::TocEditModalBackspace => self.toc_edit_pop_char(),
            Action::TocEditModalSave => self.save_toc_modal(),
            Action::TocEditModalCancel => self.cancel_toc_modal(),

            // TOC Import Modal actions
            Action::TocImportModalNextField => self.toc_import_next_field(),
            Action::TocImportModalPrevField => self.toc_import_prev_field(),
            Action::TocImportModalToggleSource => self.toc_import_toggle_source(),
            Action::TocImportModalToggleMerge => self.toc_import_toggle_merge(),
            Action::TocImportModalInput(c) => self.toc_import_push_char(c),
            Action::TocImportModalBackspace => self.toc_import_pop_char(),
            Action::TocImportModalConfirm => self.confirm_import_toc(),
            Action::TocImportModalCancel => self.cancel_import_toc(),

            Action::ExportLibrary => {
                if let Some(ref conn) = self.db_conn {
                    let options = papyrus_core::ExportOptions::default();
                    match papyrus_core::export_library(conn, &self.config, &options) {
                        Ok(res) => {
                            self.set_status(format!(
                                "Library exported successfully to {}",
                                res.archive_path.display()
                            ));
                        }
                        Err(err) => {
                            self.set_status(format!("Export failed: {err}"));
                        }
                    }
                } else {
                    self.set_status("Cannot export library: database connection unavailable");
                }
            }

            // Add Paper Modal actions
            Action::AddPaperModalOpen => {
                self.is_adding_paper = true;
                self.add_paper_path_buffer.clear();
            }
            Action::AddPaperModalInput(c) => {
                self.add_paper_path_buffer.push(c);
            }
            Action::AddPaperModalBackspace => {
                self.add_paper_path_buffer.pop();
            }
            Action::AddPaperModalCancel => {
                self.is_adding_paper = false;
                self.add_paper_path_buffer.clear();
            }
            Action::AddPaperModalConfirm => {
                self.confirm_add_paper();
            }
            Action::AddPaperModalAutocomplete => {
                if let Some(completed) = autocomplete_path(&self.add_paper_path_buffer) {
                    self.add_paper_path_buffer = completed;
                }
            }

            // Create Collection & Subcollection Modal actions
            Action::CreateCollectionModalOpen => {
                self.create_collection_parent_id = None;
                self.is_creating_collection = true;
                self.collection_name_buffer.clear();
            }
            Action::CreateSubcollectionModalOpen => {
                let Some(col) = self.current_collection() else {
                    self.set_status("No collection selected");
                    return;
                };
                let Some(col_id) = col.id else {
                    self.set_status("Cannot create subcollection inside 'All Papers'");
                    return;
                };
                self.create_collection_parent_id = Some(col_id);
                self.is_creating_collection = true;
                self.collection_name_buffer.clear();
                self.needs_clear = true;
            }
            Action::CreateCollectionModalInput(c) => {
                self.collection_name_buffer.push(c);
            }
            Action::CreateCollectionModalBackspace => {
                self.collection_name_buffer.pop();
            }
            Action::CreateCollectionModalCancel => {
                self.is_creating_collection = false;
                self.create_collection_parent_id = None;
                self.collection_name_buffer.clear();
            }
            Action::CreateCollectionModalConfirm => {
                self.confirm_create_collection();
            }

            // Rename Collection Modal actions
            Action::RenameCollectionModalOpen => {
                let Some(col) = self.current_collection() else {
                    self.set_status("No collection selected to rename");
                    return;
                };
                if col.id.is_none() {
                    self.set_status("Cannot rename 'All Papers' virtual collection");
                    return;
                }
                self.rename_collection_buffer = col.name.clone();
                self.is_renaming_collection = true;
                self.needs_clear = true;
            }
            Action::RenameCollectionModalInput(c) => {
                self.rename_collection_buffer.push(c);
            }
            Action::RenameCollectionModalBackspace => {
                self.rename_collection_buffer.pop();
            }
            Action::RenameCollectionModalCancel => {
                self.is_renaming_collection = false;
                self.rename_collection_buffer.clear();
                self.needs_clear = true;
            }
            Action::RenameCollectionModalConfirm => {
                self.confirm_rename_collection();
            }

            // Export Collection Modal actions
            Action::ExportCollectionModalOpen => {
                if self.collections.is_empty() {
                    self.set_status("No collection selected to export");
                    return;
                }
                self.is_exporting_collection = true;
                self.export_path_buffer.clear();
                self.needs_clear = true;
            }
            Action::ExportCollectionModalInput(c) => {
                self.export_path_buffer.push(c);
            }
            Action::ExportCollectionModalBackspace => {
                self.export_path_buffer.pop();
            }
            Action::ExportCollectionModalCancel => {
                self.is_exporting_collection = false;
                self.export_path_buffer.clear();
                self.needs_clear = true;
            }
            Action::ExportCollectionModalAutocomplete => {
                if let Some(completed) = autocomplete_path(&self.export_path_buffer) {
                    self.export_path_buffer = completed;
                }
            }
            Action::ExportCollectionModalConfirm => {
                self.confirm_export_collection();
            }

            // Import Metadata Modal actions
            Action::ImportMetadataModalOpen => {
                self.is_importing_metadata = true;
                self.import_metadata_buffer.clear();
                self.needs_clear = true;
            }
            Action::ImportMetadataModalInput(c) => {
                self.import_metadata_buffer.push(c);
            }
            Action::ImportMetadataModalBackspace => {
                self.import_metadata_buffer.pop();
            }
            Action::ImportMetadataModalCancel => {
                self.is_importing_metadata = false;
                self.import_metadata_buffer.clear();
                self.needs_clear = true;
            }
            Action::ImportMetadataModalAutocomplete => {
                if let Some(completed) = autocomplete_path(&self.import_metadata_buffer) {
                    self.import_metadata_buffer = completed;
                }
            }
            Action::ImportMetadataModalConfirm => {
                self.confirm_import_metadata();
            }

            // Fullscreen TOC actions
            Action::OpenFullscreenToc => {
                if self.toc_preview.is_empty() {
                    self.set_status("No TOC available for this paper");
                } else {
                    self.is_viewing_fullscreen_toc = true;
                    self.toc_scroll_offset = 0;
                    self.needs_clear = true;
                }
            }
            Action::CloseFullscreenToc => {
                self.is_viewing_fullscreen_toc = false;
                self.needs_clear = true;
            }

            // Tags Modal actions
            Action::EditTagsModalOpen => {
                if let Some(paper) = self.current_paper() {
                    let current_tags = self
                        .tags_by_paper
                        .get(&paper.id)
                        .cloned()
                        .unwrap_or_default();
                    self.tags_input_buffer = current_tags.join(", ");
                    self.is_editing_tags = true;
                    self.needs_clear = true;
                } else {
                    self.set_status("No paper selected to edit tags");
                }
            }
            Action::EditTagsModalInput(c) => {
                self.tags_input_buffer.push(c);
            }
            Action::EditTagsModalBackspace => {
                self.tags_input_buffer.pop();
            }
            Action::EditTagsModalCancel => {
                self.is_editing_tags = false;
                self.tags_input_buffer.clear();
                self.needs_clear = true;
            }
            Action::EditTagsModalConfirm => {
                self.confirm_edit_tags();
            }

            // Delete Confirmation actions
            Action::DeleteConfirmOpen => {
                self.open_delete_confirm();
            }
            Action::DeleteConfirmCancel => {
                self.is_confirming_delete = false;
                self.delete_target_description.clear();
                self.needs_clear = true;
            }
            Action::DeleteConfirmExecute => {
                self.execute_delete();
            }

            // Search cycling
            Action::SearchCycleCollection => {
                self.cycle_search_collection();
            }

            // Help Modal
            Action::HelpModalToggle => {
                self.is_showing_help = !self.is_showing_help;
                self.needs_clear = true;
            }

            Action::Quit => {
                self.running = false;
            }
            _ => {}
        }
    }

    /// Confirms creating a new collection from `self.collection_name_buffer`.
    fn confirm_create_collection(&mut self) {
        let name = self.collection_name_buffer.trim().to_string();
        if name.is_empty() {
            self.set_status("Collection name cannot be empty");
            return;
        }

        let parent_id = self.create_collection_parent_id;

        if let Some(ref conn) = self.db_conn {
            let new_col = papyrus_core::db::Collection {
                id: uuid::Uuid::now_v7(),
                name: name.to_string(),
                parent_id,
            };

            match papyrus_core::db::CollectionRepo::insert(conn, &new_col) {
                Ok(()) => {
                    let _ = self.reload_from_db();
                    if let Some(idx) = self
                        .collections
                        .iter()
                        .position(|c| c.id == Some(new_col.id))
                    {
                        self.selected_collection = idx;
                        self.selected_paper = 0;
                        self.selected_toc = 0;
                        self.sync_current_selection();
                    }
                    self.is_creating_collection = false;
                    self.create_collection_parent_id = None;
                    self.collection_name_buffer.clear();
                    self.needs_clear = true;
                    self.set_status(format!("Created collection '{}'", new_col.name));
                }
                Err(err) => {
                    self.set_status(format!("Failed to create collection: {err}"));
                }
            }
        } else {
            let new_id = uuid::Uuid::now_v7();
            let parent_depth = parent_id
                .and_then(|pid| {
                    self.collections
                        .iter()
                        .find(|c| c.id == Some(pid))
                        .map(|c| c.depth)
                })
                .unwrap_or(0);
            let depth = if parent_id.is_some() {
                parent_depth + 1
            } else {
                0
            };
            let new_item = crate::app::CollectionItem::with_hierarchy(
                Some(new_id),
                &name,
                0,
                depth,
                parent_id,
            );
            self.add_collection(new_item, Vec::new());
            self.selected_collection = self.collections.len().saturating_sub(1);
            self.selected_paper = 0;
            self.selected_toc = 0;
            self.sync_current_selection();
            self.is_creating_collection = false;
            self.create_collection_parent_id = None;
            self.collection_name_buffer.clear();
            self.needs_clear = true;
            self.set_status(format!("Created collection '{name}'"));
        }
    }

    /// Confirms renaming the currently selected collection.
    fn confirm_rename_collection(&mut self) {
        let new_name = self.rename_collection_buffer.trim().to_string();
        if new_name.is_empty() {
            self.set_status("Collection name cannot be empty");
            return;
        }
        let Some(col) = self.current_collection() else {
            self.set_status("No collection selected");
            return;
        };
        let Some(col_id) = col.id else {
            self.set_status("Cannot rename 'All Papers' virtual collection");
            return;
        };

        if let Some(ref conn) = self.db_conn {
            match papyrus_core::db::CollectionRepo::rename(conn, col_id, &new_name) {
                Ok(_) => {
                    let _ = self.reload_from_db();
                    self.is_renaming_collection = false;
                    self.rename_collection_buffer.clear();
                    self.needs_clear = true;
                    self.set_status(format!("Renamed collection to '{new_name}'"));
                }
                Err(err) => {
                    self.set_status(format!("Failed to rename collection: {err}"));
                }
            }
        } else {
            if let Some(col_item) = self.collections.get_mut(self.selected_collection) {
                col_item.name = new_name.clone();
            }
            self.is_renaming_collection = false;
            self.rename_collection_buffer.clear();
            self.needs_clear = true;
            self.set_status(format!("Renamed collection to '{new_name}'"));
        }
    }

    /// Confirms exporting the currently selected collection (or library) to a ZIP archive.
    fn confirm_export_collection(&mut self) {
        let Some(col) = self.current_collection().cloned() else {
            self.set_status("No collection selected to export");
            return;
        };

        let raw_path = self.export_path_buffer.trim();
        let target_path = if !raw_path.is_empty() {
            let expanded = if let Some(stripped) = raw_path.strip_prefix("~/") {
                if let Some(home) = std::env::var_os("HOME") {
                    PathBuf::from(home).join(stripped)
                } else {
                    PathBuf::from(raw_path)
                }
            } else {
                PathBuf::from(raw_path)
            };
            if expanded.is_dir() || raw_path.ends_with('/') {
                let default_name = if let Some(ref col_id) = col.id {
                    let clean = col.name.replace(['/', '\\', ' '], "_");
                    format!("papyrus_collection_{clean}_{col_id}.zip")
                } else {
                    "papyrus_library_export.zip".to_string()
                };
                expanded.join(default_name)
            } else if expanded.extension().is_none() {
                expanded.with_extension("zip")
            } else {
                expanded
            }
        } else {
            PathBuf::new()
        };

        let options = papyrus_core::ExportOptions {
            output_path: target_path,
            overwrite: true,
            include_database: true,
            skip_missing_files: true,
        };

        if let Some(ref conn) = self.db_conn {
            let result = if let Some(col_id) = col.id {
                papyrus_core::export_collection(conn, &self.config, col_id, &options)
            } else {
                papyrus_core::export_library(conn, &self.config, &options)
            };

            match result {
                Ok(res) => {
                    self.is_exporting_collection = false;
                    self.export_path_buffer.clear();
                    self.needs_clear = true;
                    self.set_status(format!(
                        "Exported {} papers to '{}'",
                        res.paper_count,
                        res.archive_path.display()
                    ));
                }
                Err(err) => {
                    self.set_status(format!("Export failed: {err}"));
                }
            }
        } else {
            self.is_exporting_collection = false;
            self.export_path_buffer.clear();
            self.needs_clear = true;
            self.set_status("Cannot export collection: database connection unavailable");
        }
    }

    /// Confirms importing metadata from a JSON file or directory path.
    fn confirm_import_metadata(&mut self) {
        let raw_path = self.import_metadata_buffer.trim();
        if raw_path.is_empty() {
            self.set_status("Please enter a JSON file or directory path");
            return;
        }

        let expanded_path = if let Some(stripped) = raw_path.strip_prefix("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                PathBuf::from(home).join(stripped)
            } else {
                PathBuf::from(raw_path)
            }
        } else {
            PathBuf::from(raw_path)
        };

        if !expanded_path.exists() {
            self.set_status(format!("Path not found: '{}'", expanded_path.display()));
            return;
        }

        let selected_paper_id = self.current_paper().map(|p| p.id);

        if let Some(ref mut conn) = self.db_conn {
            let index_ref = self.search_index.as_deref();
            match papyrus_core::import_metadata_from_path(
                conn,
                &expanded_path,
                selected_paper_id,
                index_ref,
            ) {
                Ok(count) => {
                    let _ = self.reload_from_db();
                    self.is_importing_metadata = false;
                    self.import_metadata_buffer.clear();
                    self.needs_clear = true;
                    self.set_status(format!("Updated metadata for {count} paper(s)"));
                }
                Err(err) => {
                    self.set_status(format!("Metadata import failed: {err}"));
                }
            }
        } else {
            self.is_importing_metadata = false;
            self.import_metadata_buffer.clear();
            self.needs_clear = true;
            self.set_status("Cannot import metadata: database connection unavailable");
        }
    }

    /// Confirms importing a paper or directory of papers from `self.add_paper_path_buffer`.
    fn confirm_add_paper(&mut self) {
        let path_str = self.add_paper_path_buffer.trim();
        if path_str.is_empty() {
            self.set_status("Please enter a file or directory path");
            return;
        }

        let expanded_path = if let Some(stripped) = path_str.strip_prefix("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                PathBuf::from(home).join(stripped)
            } else {
                PathBuf::from(path_str)
            }
        } else {
            PathBuf::from(path_str)
        };

        if !expanded_path.exists() {
            self.set_status(format!("Path not found: '{}'", expanded_path.display()));
            return;
        }

        let target_collection_id = if !self.collections.is_empty() {
            self.collections[self.selected_collection].id
        } else {
            None
        };

        if let Some(ref mut conn) = self.db_conn {
            if expanded_path.is_dir() {
                let mut pdf_files = Vec::new();
                collect_pdf_files_recursive(&expanded_path, &mut pdf_files);
                pdf_files.sort();

                if pdf_files.is_empty() {
                    self.set_status(format!(
                        "No PDF files found in '{}'",
                        expanded_path.display()
                    ));
                    self.is_adding_paper = false;
                    self.add_paper_path_buffer.clear();
                    return;
                }

                let mut imported_count = 0usize;
                let mut duplicate_count = 0usize;
                let mut error_count = 0usize;

                for file in &pdf_files {
                    match papyrus_core::importer::import_paper(
                        conn,
                        &self.config,
                        file,
                        target_collection_id,
                    ) {
                        Ok(paper) => {
                            if let Some(ref search_index) = self.search_index {
                                let body_text = papyrus_core::pdf::extract_text(file).ok();
                                let _ = search_index.index_paper(&paper, body_text.as_deref());
                            }
                            imported_count += 1;
                        }
                        Err(papyrus_core::importer::ImportError::Duplicate { .. }) => {
                            duplicate_count += 1;
                        }
                        Err(_) => {
                            error_count += 1;
                        }
                    }
                }

                let _ = self.reload_from_db();
                self.is_adding_paper = false;
                self.add_paper_path_buffer.clear();
                self.needs_clear = true;

                let mut msg = format!(
                    "Imported {imported_count} papers from '{}'",
                    expanded_path.display()
                );
                if duplicate_count > 0 {
                    msg.push_str(&format!(" ({duplicate_count} duplicates skipped)"));
                }
                if error_count > 0 {
                    msg.push_str(&format!(" ({error_count} failed)"));
                }
                self.set_status(msg);
                return;
            }

            match papyrus_core::importer::import_paper(
                conn,
                &self.config,
                &expanded_path,
                target_collection_id,
            ) {
                Ok(paper) => {
                    if let Some(ref search_index) = self.search_index {
                        let body_text = papyrus_core::pdf::extract_text(&expanded_path).ok();
                        let _ = search_index.index_paper(&paper, body_text.as_deref());
                    }

                    let _ = self.reload_from_db();

                    if let Some(idx) = self.papers.iter().position(|p| p.id == paper.id) {
                        self.selected_paper = idx;
                        self.sync_paper_selection();
                    }

                    self.is_adding_paper = false;
                    self.add_paper_path_buffer.clear();
                    self.needs_clear = true;
                    self.set_status(format!(
                        "Added paper '{}'",
                        paper.title.as_deref().unwrap_or("[Untitled]")
                    ));
                }
                Err(papyrus_core::importer::ImportError::Duplicate { existing_id, .. }) => {
                    self.needs_clear = true;
                    self.set_status(format!(
                        "Duplicate paper: already exists with ID {existing_id}"
                    ));
                }
                Err(err) => {
                    self.needs_clear = true;
                    self.set_status(format!("Import failed: {err}"));
                }
            }
        } else {
            self.is_adding_paper = false;
            self.add_paper_path_buffer.clear();
            self.set_status("Cannot add paper: database connection unavailable");
        }
    }

    /// Confirms tag editing, saving the tags into SQLite (or in-memory) and updating state.
    fn confirm_edit_tags(&mut self) {
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

    /// Prepares delete confirmation description based on the active panel.
    fn open_delete_confirm(&mut self) {
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
    fn execute_delete(&mut self) {
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
}

/// Recursively traverses `dir` collecting all files with `.pdf` extension.
fn collect_pdf_files_recursive(dir: &std::path::Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_pdf_files_recursive(&path, files);
        } else if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext.eq_ignore_ascii_case("pdf") {
                    files.push(path);
                }
            }
        }
    }
}
