use std::path::PathBuf;

use crate::app::App;

impl App {
    /// Confirms importing metadata from a JSON file or directory path.
    pub(crate) fn confirm_import_metadata(&mut self) {
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
    pub(crate) fn confirm_add_paper(&mut self) {
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
