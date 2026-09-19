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
                if self.visual_mode && self.active_panel == ActivePanel::Papers {
                    self.exit_visual_mode();
                }
                self.record_position_for_current_collection();
                self.record_position_for_current_paper();
                self.active_panel = self.active_panel.next();
            }
            Action::PreviousPanel => {
                if self.visual_mode && self.active_panel == ActivePanel::Papers {
                    self.exit_visual_mode();
                }
                self.record_position_for_current_collection();
                self.record_position_for_current_paper();
                self.active_panel = self.active_panel.previous();
            }
            Action::PanelLeft => {
                if self.visual_mode && self.active_panel == ActivePanel::Papers {
                    self.exit_visual_mode();
                }
                self.record_position_for_current_collection();
                self.record_position_for_current_paper();
                self.active_panel = self.active_panel.left();
            }
            Action::PanelRight => {
                if self.visual_mode && self.active_panel == ActivePanel::Papers {
                    self.exit_visual_mode();
                }
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
            Action::RemoveFromCollection => {
                self.remove_current_paper_from_collection();
            }
            Action::BatchRemoveFromCollection => {
                self.batch_remove_papers_from_collection();
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
            Action::VisualModeToggle => {
                self.toggle_visual_mode();
            }
            Action::VisualModeCancel => {
                self.exit_visual_mode();
            }
            Action::VisualModeToggleItem => {
                self.toggle_current_paper_selection();
            }
            Action::QuickOpenModalOpen => {
                self.open_quick_open();
            }
            Action::CollectionMembershipModalOpen => {
                self.open_collection_membership();
            }
            Action::TagModalOpen => {
                self.open_tag_picker();
            }
            Action::PickerInput(c) => {
                if let Some(ref mut picker) = self.active_picker {
                    picker.push_query_char(c);
                }
            }
            Action::PickerBackspace => {
                if let Some(ref mut picker) = self.active_picker {
                    picker.pop_query_char();
                }
            }
            Action::PickerMoveDown => {
                if let Some(ref mut picker) = self.active_picker {
                    picker.move_cursor(1);
                }
            }
            Action::PickerMoveUp => {
                if let Some(ref mut picker) = self.active_picker {
                    picker.move_cursor(-1);
                }
            }
            Action::PickerPageDown => {
                if let Some(ref mut picker) = self.active_picker {
                    picker.move_cursor(10);
                }
            }
            Action::PickerPageUp => {
                if let Some(ref mut picker) = self.active_picker {
                    picker.move_cursor(-10);
                }
            }
            Action::PickerToggleItem => {
                if let Some(ref mut picker) = self.active_picker {
                    picker.toggle_selected();
                }
            }
            Action::PickerConfirm => {
                self.confirm_active_picker();
            }
            Action::PickerCancel => {
                self.cancel_active_picker();
            }
            Action::BatchDeleteConfirm if self.visual_mode => {
                let papers = self.visual_selected_papers();
                if !papers.is_empty() {
                    if let Some(ref mut conn) = self.db_conn {
                        let search_index_ref = self.search_index.as_deref();
                        let _ = crate::app::batch_delete_papers(conn, &papers, search_index_ref);
                    }
                    if self.db_conn.is_some() {
                        let _ = self.reload_from_db();
                    } else {
                        for list in self.papers_by_collection.values_mut() {
                            list.retain(|p| !papers.contains(&p.id));
                        }
                        for pid in &papers {
                            self.tags_by_paper.remove(pid);
                            self.tocs_by_paper.remove(pid);
                            self.last_toc_by_paper.remove(pid);
                        }
                        self.last_paper_by_collection
                            .retain(|_, pid| !papers.contains(pid));
                        self.sync_current_selection();
                    }
                    self.set_status(format!("Deleted {} papers", papers.len()));
                }
                self.exit_visual_mode();
                self.needs_clear = true;
            }
            _ => {}
        }
    }
}
