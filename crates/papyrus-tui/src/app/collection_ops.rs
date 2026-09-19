use std::path::PathBuf;

use crate::app::App;

impl App {
    /// Confirms creating a new collection from `self.collection_name_buffer`.
    pub(crate) fn confirm_create_collection(&mut self) {
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
    pub(crate) fn confirm_rename_collection(&mut self) {
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
    pub(crate) fn confirm_export_collection(&mut self) {
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
}
