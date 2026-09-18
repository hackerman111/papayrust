pub mod autocomplete;
pub mod db_sync;
pub mod dispatch;
pub mod metadata;
pub mod navigation;
pub mod panel;
pub mod picker;
pub mod search;
pub mod selection;
pub mod sorting;
pub mod toc;

#[cfg(test)]
mod tests;

pub use navigation::Motion;
pub use panel::{ActivePanel, CollectionItem};
pub use picker::{GenericPicker, PickerItem};
pub use selection::{CollectionKey, SelectionState};
pub use sorting::{PaperSortField, SortDirection};
pub use toc::{TocEditState, TocImportSourceType, TocImportState};

/// Layout mode for the main panel area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutMode {
    #[default]
    MultiPanel,
    SinglePanel,
}

use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use papyrus_core::config::Config;
use papyrus_core::db::{Paper, TocEntry};
use papyrus_core::opener::{CommandRunner, ProcessCommandRunner};
use papyrus_core::search::SearchIndex;

/// Primary TUI application state.
pub struct App {
    /// Currently focused panel.
    pub active_panel: ActivePanel,
    /// List of collections displayed in the left panel.
    pub collections: Vec<CollectionItem>,
    /// 0-based index of the currently selected collection.
    pub selected_collection: usize,
    /// List of papers belonging to the selected collection.
    pub papers: Vec<Paper>,
    /// 0-based index of the currently selected paper.
    pub selected_paper: usize,
    /// Table of contents preview entries for the currently selected paper.
    pub toc_preview: Vec<TocEntry>,
    /// 0-based index of the selected TOC entry in the details panel.
    pub selected_toc: usize,
    /// Authoritative selection state (collection, paper, TOC).
    pub selection: SelectionState,
    /// In-memory cache remembering the last selected paper UUID for each collection key.
    pub last_paper_by_collection: HashMap<CollectionKey, Uuid>,
    /// In-memory cache remembering the last selected TOC entry UUID for each paper UUID.
    pub last_toc_by_paper: HashMap<Uuid, Uuid>,
    /// Optional status/help message displayed in the bottom bar.
    pub status_message: Option<String>,
    /// Flag indicating whether the application is running.
    pub running: bool,
    /// Application configuration.
    pub config: Config,
    /// Flag indicating whether search mode is active.
    pub is_searching: bool,
    /// Active search / filter query string.
    pub search_query: String,
    /// Flag indicating whether the metadata editing modal is open.
    pub is_editing_metadata: bool,
    /// 0-based index of the currently active editing field (0..=5).
    pub editing_field_index: usize,
    /// Input buffers for the 6 metadata fields:
    /// [0] Title, [1] Authors, [2] Year, [3] Journal, [4] DOI, [5] Abstract.
    pub edit_buffers: [String; 6],
    /// State of the TOC add/edit modal, if open.
    pub toc_edit_state: Option<TocEditState>,
    /// State of the TOC import modal, if open.
    pub toc_import_state: Option<TocImportState>,
    /// Flag indicating whether the add paper modal is open.
    pub is_adding_paper: bool,
    /// Path input buffer for adding a paper.
    pub add_paper_path_buffer: String,
    /// Flag indicating whether the create collection modal is open.
    pub is_creating_collection: bool,
    /// Name input buffer for creating a collection.
    pub collection_name_buffer: String,
    /// Flag indicating that the terminal buffer should be cleared before the next draw.
    pub needs_clear: bool,
    /// Flag indicating whether the dedicated fullscreen TOC view is active.
    pub is_viewing_fullscreen_toc: bool,
    /// Vertical scroll offset for the fullscreen TOC view.
    pub toc_scroll_offset: usize,
    /// Flag indicating whether the tag editing modal is open.
    pub is_editing_tags: bool,
    /// Input buffer for editing paper tags.
    pub tags_input_buffer: String,
    /// Flag indicating whether the delete confirmation modal is open.
    pub is_confirming_delete: bool,
    /// Description of the target to be deleted shown in the confirmation modal.
    pub delete_target_description: String,
    /// Flag indicating whether the keyboard shortcuts / help modal is open.
    pub is_showing_help: bool,
    /// Index of the currently targeted collection during search mode.
    pub search_collection_index: usize,
    /// Flag indicating whether the rename collection modal is open.
    pub is_renaming_collection: bool,
    /// Buffer for the new collection name.
    pub rename_collection_buffer: String,
    /// Flag indicating whether the export collection modal is open.
    pub is_exporting_collection: bool,
    /// Buffer for the export archive destination path.
    pub export_path_buffer: String,
    /// Flag indicating whether the import metadata modal is open.
    pub is_importing_metadata: bool,
    /// Buffer for the import metadata path.
    pub import_metadata_buffer: String,
    /// Optional parent collection ID when creating a subcollection.
    pub create_collection_parent_id: Option<Uuid>,
    /// In-memory cache mapping paper id to list of tags.
    pub tags_by_paper: HashMap<Uuid, Vec<String>>,
    /// Active layout mode: multi-panel (3 panels) or single-panel (focused panel fullscreen).
    pub layout_mode: LayoutMode,
    /// Active sort field for the papers list.
    pub sort_field: PaperSortField,
    /// Active sort direction for the papers list.
    pub sort_direction: SortDirection,
    /// Pending numeric count prefix for Vim motions (e.g. 5j, 12G).
    pub pending_count: Option<usize>,
    /// Pending character chord for multi-key sequences (e.g. 'g' in gg).
    pub pending_chord: Option<char>,
    /// Cached breadcrumbs for each collection key.
    pub collection_breadcrumbs: HashMap<CollectionKey, String>,
    /// Command runner for launching external PDF viewers.
    pub(crate) runner: Arc<dyn CommandRunner>,

    /// Optional search index for Tantivy full-text search.
    pub(crate) search_index: Option<Arc<SearchIndex>>,
    /// SQLite connection for persistence operations in TUI.
    pub(crate) db_conn: Option<Connection>,
    /// In-memory cache mapping collection id (None for All Papers) to list of papers.
    pub(crate) papers_by_collection: HashMap<Option<Uuid>, Vec<Paper>>,
    /// In-memory cache mapping paper id to list of TOC entries.
    pub(crate) tocs_by_paper: HashMap<Uuid, Vec<TocEntry>>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Creates a new, empty App.
    pub fn new() -> Self {
        Self {
            active_panel: ActivePanel::Collections,
            collections: Vec::new(),
            selected_collection: 0,
            papers: Vec::new(),
            selected_paper: 0,
            toc_preview: Vec::new(),
            selected_toc: 0,
            selection: SelectionState::default(),
            last_paper_by_collection: HashMap::new(),
            last_toc_by_paper: HashMap::new(),
            status_message: None,
            running: true,
            config: Config::default(),
            is_searching: false,
            search_query: String::new(),
            is_editing_metadata: false,
            editing_field_index: 0,
            edit_buffers: Default::default(),
            toc_edit_state: None,
            toc_import_state: None,
            is_adding_paper: false,
            add_paper_path_buffer: String::new(),
            is_creating_collection: false,
            collection_name_buffer: String::new(),
            needs_clear: false,
            is_viewing_fullscreen_toc: false,
            toc_scroll_offset: 0,
            is_editing_tags: false,
            tags_input_buffer: String::new(),
            is_confirming_delete: false,
            delete_target_description: String::new(),
            is_showing_help: false,
            search_collection_index: 0,
            is_renaming_collection: false,
            rename_collection_buffer: String::new(),
            is_exporting_collection: false,
            export_path_buffer: String::new(),
            is_importing_metadata: false,
            import_metadata_buffer: String::new(),
            create_collection_parent_id: None,
            tags_by_paper: HashMap::new(),
            layout_mode: LayoutMode::default(),
            sort_field: PaperSortField::default(),
            sort_direction: SortDirection::default(),
            pending_count: None,
            pending_chord: None,
            collection_breadcrumbs: HashMap::new(),
            runner: Arc::new(ProcessCommandRunner),
            search_index: None,
            db_conn: None,
            papers_by_collection: HashMap::new(),
            tocs_by_paper: HashMap::new(),
        }
    }

    /// Creates an App initialized with explicit collections, papers mapping, and TOC entries.
    pub fn with_data(
        collections: Vec<CollectionItem>,
        papers_by_collection: HashMap<Option<Uuid>, Vec<Paper>>,
        tocs_by_paper: HashMap<Uuid, Vec<TocEntry>>,
    ) -> Self {
        let mut app = Self {
            active_panel: ActivePanel::Papers,
            collections,
            selected_collection: 0,
            papers: Vec::new(),
            selected_paper: 0,
            toc_preview: Vec::new(),
            selected_toc: 0,
            selection: SelectionState::default(),
            last_paper_by_collection: HashMap::new(),
            last_toc_by_paper: HashMap::new(),
            status_message: None,
            running: true,
            config: Config::default(),
            is_searching: false,
            search_query: String::new(),
            is_editing_metadata: false,
            editing_field_index: 0,
            edit_buffers: Default::default(),
            toc_edit_state: None,
            toc_import_state: None,
            is_adding_paper: false,
            add_paper_path_buffer: String::new(),
            is_creating_collection: false,
            collection_name_buffer: String::new(),
            needs_clear: false,
            is_viewing_fullscreen_toc: false,
            toc_scroll_offset: 0,
            is_editing_tags: false,
            tags_input_buffer: String::new(),
            is_confirming_delete: false,
            delete_target_description: String::new(),
            is_showing_help: false,
            search_collection_index: 0,
            is_renaming_collection: false,
            rename_collection_buffer: String::new(),
            is_exporting_collection: false,
            export_path_buffer: String::new(),
            is_importing_metadata: false,
            import_metadata_buffer: String::new(),
            create_collection_parent_id: None,
            tags_by_paper: HashMap::new(),
            layout_mode: LayoutMode::default(),
            sort_field: PaperSortField::default(),
            sort_direction: SortDirection::default(),
            pending_count: None,
            pending_chord: None,
            collection_breadcrumbs: HashMap::new(),
            runner: Arc::new(ProcessCommandRunner),
            search_index: None,
            db_conn: None,
            papers_by_collection,
            tocs_by_paper,
        };
        app.update_breadcrumbs();
        app.sync_current_selection();
        app
    }

    /// Adds a collection and its associated papers to the in-memory state.
    pub fn add_collection(&mut self, item: CollectionItem, papers: Vec<Paper>) {
        let key = item.id;
        self.collections.push(item);
        self.papers_by_collection.insert(key, papers);
        self.update_breadcrumbs();
        self.sync_current_selection();
    }

    /// Sets the table of contents entries for a specific paper.
    pub fn set_paper_tocs(&mut self, paper_id: Uuid, tocs: Vec<TocEntry>) {
        self.tocs_by_paper.insert(paper_id, tocs);
        self.sync_paper_selection();
    }

    /// Sets the tags for a specific paper.
    pub fn set_paper_tags(&mut self, paper_id: Uuid, tags: Vec<String>) {
        self.tags_by_paper.insert(paper_id, tags);
    }

    /// Returns the tags for the currently selected paper.
    pub fn current_paper_tags(&self) -> Vec<String> {
        self.current_paper()
            .and_then(|p| self.tags_by_paper.get(&p.id).cloned())
            .unwrap_or_default()
    }

    /// Sets a temporary status message in the bottom bar.
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }

    /// Sets the command runner used for opening external viewers.
    pub fn set_runner<R: CommandRunner + 'static>(&mut self, runner: R) {
        self.runner = Arc::new(runner);
    }

    /// Builder method to set the command runner.
    pub fn with_runner<R: CommandRunner + 'static>(mut self, runner: R) -> Self {
        self.runner = Arc::new(runner);
        self
    }

    /// Sets the command runner as an Arc<dyn CommandRunner>.
    pub fn set_runner_arc(&mut self, runner: Arc<dyn CommandRunner>) {
        self.runner = runner;
    }

    /// Builder method to set the command runner as an Arc<dyn CommandRunner>.
    pub fn with_runner_arc(mut self, runner: Arc<dyn CommandRunner>) -> Self {
        self.runner = runner;
        self
    }

    /// Returns a clone of the command runner.
    pub fn runner(&self) -> Arc<dyn CommandRunner> {
        Arc::clone(&self.runner)
    }

    /// Sets the application configuration.
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    /// Builder method to set the application configuration.
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    /// Clears the status message.
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Returns the currently selected collection, if any.
    pub fn current_collection(&self) -> Option<&CollectionItem> {
        self.collections.get(self.selected_collection)
    }

    /// Returns the currently selected paper, if any.
    pub fn current_paper(&self) -> Option<&Paper> {
        self.papers.get(self.selected_paper)
    }

    /// Returns the currently selected table of contents entry, if any.
    pub fn current_toc(&self) -> Option<&TocEntry> {
        self.toc_preview.get(self.selected_toc)
    }

    /// Sets the optional Tantivy search index.
    pub fn set_search_index(&mut self, index: Arc<SearchIndex>) {
        self.search_index = Some(index);
    }

    /// Builder method to set the optional Tantivy search index.
    pub fn with_search_index(mut self, index: Arc<SearchIndex>) -> Self {
        self.search_index = Some(index);
        self
    }

    /// Returns a reference to the optional search index.
    pub fn search_index(&self) -> Option<&Arc<SearchIndex>> {
        self.search_index.as_ref()
    }

    /// Sets the SQLite database connection used for data operations.
    pub fn set_db_conn(&mut self, conn: Connection) {
        self.db_conn = Some(conn);
    }

    /// Builder method to set the SQLite database connection.
    pub fn with_db_conn(mut self, conn: Connection) -> Self {
        self.db_conn = Some(conn);
        self
    }

    /// Returns a reference to the SQLite database connection, if set.
    pub fn db_conn(&self) -> Option<&Connection> {
        self.db_conn.as_ref()
    }

    /// Returns a mutable reference to the SQLite database connection, if set.
    pub fn db_conn_mut(&mut self) -> Option<&mut Connection> {
        self.db_conn.as_mut()
    }

    /// Computes and caches breadcrumb paths for all collections in `self.collections`.
    pub fn update_breadcrumbs(&mut self) {
        self.collection_breadcrumbs.clear();
        let id_map: HashMap<Uuid, (&str, Option<Uuid>)> = self
            .collections
            .iter()
            .filter_map(|c| c.id.map(|id| (id, (c.name.as_str(), c.parent_id))))
            .collect();

        for col in &self.collections {
            let breadcrumb = match &col.key {
                CollectionKey::All => "All Papers".to_string(),
                CollectionKey::RecentlyAdded => "Recently Added".to_string(),
                CollectionKey::Unfiled => "Unfiled".to_string(),
                CollectionKey::Untagged => "Untagged".to_string(),
                CollectionKey::Real(id) => {
                    let mut segments = Vec::new();
                    let mut curr = Some(*id);
                    let mut visited = std::collections::HashSet::new();

                    while let Some(cid) = curr {
                        if !visited.insert(cid) {
                            break; // cycle protection
                        }
                        if let Some((name, parent)) = id_map.get(&cid) {
                            segments.push(*name);
                            curr = *parent;
                        } else {
                            break;
                        }
                    }
                    segments.reverse();
                    segments.join(" / ")
                }
            };
            self.collection_breadcrumbs
                .insert(col.key.clone(), breadcrumb);
        }
    }

    /// Returns the cached breadcrumb for the currently selected collection.
    pub fn current_collection_breadcrumb(&self) -> &str {
        if let Some(col) = self.current_collection() {
            if let Some(bc) = self.collection_breadcrumbs.get(&col.key) {
                return bc.as_str();
            }
            return col.name.as_str();
        }
        "All Papers"
    }
}
