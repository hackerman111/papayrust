pub mod autocomplete;
pub mod batch;
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

pub use batch::{batch_delete_papers, batch_set_collections, batch_set_tags};
pub use navigation::Motion;
pub use panel::{ActivePanel, CollectionItem};
pub use picker::{GenericPicker, PickerItem};
pub use selection::{CollectionKey, SelectionState};
pub use sorting::{PaperSortField, SortDirection};
pub use toc::{TocEditState, TocImportSourceType, TocImportState};

/// Context and target metadata for an active generic picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerContext {
    QuickOpen,
    CollectionMembership { target_papers: Vec<uuid::Uuid> },
    TagManagement { target_papers: Vec<uuid::Uuid> },
}

/// Computes a deterministic UUID for a tag name using UUIDv5.
pub fn tag_to_uuid(tag: &str) -> Uuid {
    Uuid::new_v5(&Uuid::NAMESPACE_OID, tag.as_bytes())
}

/// Layout mode for the main panel area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutMode {
    #[default]
    MultiPanel,
    SinglePanel,
}

use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
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
    /// Flag indicating whether visual selection mode is active.
    pub visual_mode: bool,
    /// 0-based index of the anchor paper when entering visual mode.
    pub visual_anchor: Option<usize>,
    /// Set of selected paper UUIDs in visual mode.
    pub visual_selected_uuids: HashSet<Uuid>,
    /// Active generic picker modal state, if open.
    pub active_picker: Option<GenericPicker>,
    /// Contextual metadata for the active picker modal.
    pub picker_context: Option<PickerContext>,
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
            visual_mode: false,
            visual_anchor: None,
            visual_selected_uuids: HashSet::new(),
            active_picker: None,
            picker_context: None,
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
            visual_mode: false,
            visual_anchor: None,
            visual_selected_uuids: HashSet::new(),
            active_picker: None,
            picker_context: None,
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

    /// Opens the Quick Open modal (`Ctrl-p`), populating papers and collections.
    pub fn open_quick_open(&mut self) {
        let mut items = Vec::new();

        // 1. Collect all papers
        let mut seen_papers = HashSet::new();
        if let Some(all_papers) = self.papers_by_collection.get(&None) {
            for paper in all_papers {
                if seen_papers.insert(paper.id) {
                    let title = paper
                        .title
                        .clone()
                        .unwrap_or_else(|| paper.file_path.clone());
                    let subtitle = paper.authors.clone();
                    let mut item = PickerItem::new(paper.id, title).with_category("Papers");
                    if let Some(sub) = subtitle {
                        item = item.with_subtitle(sub);
                    }
                    items.push(item);
                }
            }
        }
        for paper in self
            .papers_by_collection
            .values()
            .flatten()
            .chain(&self.papers)
        {
            if seen_papers.insert(paper.id) {
                let title = paper
                    .title
                    .clone()
                    .unwrap_or_else(|| paper.file_path.clone());
                let subtitle = paper.authors.clone();
                let mut item = PickerItem::new(paper.id, title).with_category("Papers");
                if let Some(sub) = subtitle {
                    item = item.with_subtitle(sub);
                }
                items.push(item);
            }
        }

        // 2. Collect all collections
        for col in &self.collections {
            let title = self
                .collection_breadcrumbs
                .get(&col.key)
                .cloned()
                .unwrap_or_else(|| col.name.clone());
            let subtitle = format!("{} papers", col.paper_count);
            let id = col.id.unwrap_or_else(|| {
                Uuid::new_v5(
                    &Uuid::NAMESPACE_OID,
                    format!("col:{:?}", col.key).as_bytes(),
                )
            });
            items.push(
                PickerItem::new(id, title)
                    .with_subtitle(subtitle)
                    .with_category("Collections"),
            );
        }

        self.active_picker = Some(GenericPicker::new("Quick Open (Ctrl-p)", items, false));
        self.picker_context = Some(PickerContext::QuickOpen);
        self.needs_clear = true;
    }

    /// Computes which collections contain the target papers (all if multiple, or single).
    pub fn compute_collections_for_papers(&self, target_papers: &[Uuid]) -> HashSet<Uuid> {
        let mut result = HashSet::new();
        if target_papers.is_empty() {
            return result;
        }

        for col in &self.collections {
            if let CollectionKey::Real(col_id) = col.key {
                let has_paper = |pid: &Uuid| -> bool {
                    if let Some(papers) = self.papers_by_collection.get(&Some(col_id)) {
                        papers.iter().any(|p| p.id == *pid)
                    } else if let Some(ref conn) = self.db_conn {
                        papyrus_core::db::CollectionRepo::get_papers(conn, col_id)
                            .map(|papers| papers.iter().any(|p| p.id == *pid))
                            .unwrap_or(false)
                    } else {
                        false
                    }
                };

                let is_member = if target_papers.len() == 1 {
                    has_paper(&target_papers[0])
                } else {
                    target_papers.iter().all(has_paper)
                };

                if is_member {
                    result.insert(col_id);
                }
            }
        }
        result
    }

    /// Opens the collection membership picker modal (`c`) for target papers.
    pub fn open_collection_membership(&mut self) {
        let target_papers = self.visual_selected_papers();
        if target_papers.is_empty() {
            self.set_status("No paper selected");
            return;
        }

        let mut items = Vec::new();
        for col in &self.collections {
            if let CollectionKey::Real(col_id) = col.key {
                let title = self
                    .collection_breadcrumbs
                    .get(&col.key)
                    .cloned()
                    .unwrap_or_else(|| col.name.clone());
                let subtitle = format!("{} papers", col.paper_count);
                items.push(
                    PickerItem::new(col_id, title)
                        .with_subtitle(subtitle)
                        .with_category("Collections"),
                );
            }
        }

        let checked = self.compute_collections_for_papers(&target_papers);
        self.active_picker =
            Some(GenericPicker::new("Add/Remove Collections", items, true).with_checked(checked));
        self.picker_context = Some(PickerContext::CollectionMembership { target_papers });
        self.needs_clear = true;
    }

    /// Computes which tag UUIDs belong to the target papers.
    pub fn compute_tags_for_papers(&self, target_papers: &[Uuid]) -> HashSet<Uuid> {
        let mut checked_uuids = HashSet::new();
        if target_papers.is_empty() {
            return checked_uuids;
        }

        let get_tags = |pid: &Uuid| -> Vec<String> {
            if let Some(tags) = self.tags_by_paper.get(pid) {
                tags.clone()
            } else if let Some(ref conn) = self.db_conn {
                papyrus_core::db::TagRepo::get_tags_for_paper(conn, *pid).unwrap_or_default()
            } else {
                Vec::new()
            }
        };

        if target_papers.len() == 1 {
            for tag in get_tags(&target_papers[0]) {
                checked_uuids.insert(tag_to_uuid(&tag));
            }
        } else {
            let first_tags = get_tags(&target_papers[0]);
            for tag in first_tags {
                let in_all = target_papers[1..]
                    .iter()
                    .all(|pid| get_tags(pid).iter().any(|t| t == &tag));
                if in_all {
                    checked_uuids.insert(tag_to_uuid(&tag));
                }
            }
        }
        checked_uuids
    }

    /// Opens the tag management picker modal (`t`) for target papers.
    pub fn open_tag_picker(&mut self) {
        let target_papers = self.visual_selected_papers();
        if target_papers.is_empty() {
            self.set_status("No paper selected");
            return;
        }

        let mut all_tags_set = HashSet::new();
        if let Some(ref conn) = self.db_conn {
            if let Ok(tags) = papyrus_core::db::TagRepo::get_all_tags(conn) {
                for t in tags {
                    all_tags_set.insert(t);
                }
            }
        }
        for tags in self.tags_by_paper.values() {
            for t in tags {
                all_tags_set.insert(t.clone());
            }
        }
        let mut distinct_tags: Vec<String> = all_tags_set.into_iter().collect();
        distinct_tags.sort();

        let items: Vec<PickerItem> = distinct_tags
            .iter()
            .map(|tag| PickerItem::new(tag_to_uuid(tag), tag).with_category("Tags"))
            .collect();

        let checked = self.compute_tags_for_papers(&target_papers);
        self.active_picker =
            Some(GenericPicker::new("Manage Tags", items, true).with_checked(checked));
        self.picker_context = Some(PickerContext::TagManagement { target_papers });
        self.needs_clear = true;
    }

    /// Confirms the active picker modal selection and performs contextual updates.
    pub fn confirm_active_picker(&mut self) {
        let Some(picker) = self.active_picker.take() else {
            self.picker_context = None;
            return;
        };
        let Some(context) = self.picker_context.take() else {
            return;
        };

        match context {
            PickerContext::QuickOpen => {
                if let Some(selected) = picker.selected_item().cloned() {
                    match selected.category.as_deref() {
                        Some("Papers") => {
                            let target_pid = selected.id;
                            let col_idx = self
                                .collections
                                .iter()
                                .position(|c| c.key == CollectionKey::All)
                                .or_else(|| {
                                    self.collections.iter().position(|c| {
                                        self.papers_by_collection
                                            .get(&c.id)
                                            .map(|list| list.iter().any(|p| p.id == target_pid))
                                            .unwrap_or(false)
                                    })
                                })
                                .unwrap_or(self.selected_collection);

                            self.selected_collection = col_idx;
                            self.sync_current_selection();
                            self.selection.paper_id = Some(target_pid);
                            self.restore_selection_by_uuid();
                            self.active_panel = ActivePanel::Papers;
                        }
                        Some("Collections") => {
                            let matching_idx = self.collections.iter().position(|c| {
                                let cid = c.id.unwrap_or_else(|| {
                                    Uuid::new_v5(
                                        &Uuid::NAMESPACE_OID,
                                        format!("col:{:?}", c.key).as_bytes(),
                                    )
                                });
                                cid == selected.id
                            });
                            if let Some(idx) = matching_idx {
                                self.selected_collection = idx;
                                self.sync_current_selection();
                                self.active_panel = ActivePanel::Papers;
                            }
                        }
                        _ => {}
                    }
                }
            }
            PickerContext::CollectionMembership { target_papers } => {
                let original_checked = self.compute_collections_for_papers(&target_papers);
                let current_checked = picker.checked_items();
                let added: Vec<Uuid> = current_checked
                    .difference(&original_checked)
                    .copied()
                    .collect();
                let removed: Vec<Uuid> = original_checked
                    .difference(current_checked)
                    .copied()
                    .collect();

                if let Some(ref mut conn) = self.db_conn {
                    let _ = batch_set_collections(conn, &target_papers, &added, &removed);
                }
                if self.db_conn.is_some() {
                    let _ = self.reload_from_db();
                } else {
                    for &pid in &target_papers {
                        for &cid in &added {
                            let paper = self.papers.iter().find(|p| p.id == pid).cloned();
                            if let Some(p) = paper {
                                let list = self.papers_by_collection.entry(Some(cid)).or_default();
                                if !list.iter().any(|x| x.id == pid) {
                                    list.push(p);
                                }
                            }
                        }
                        for &cid in &removed {
                            if let Some(list) = self.papers_by_collection.get_mut(&Some(cid)) {
                                list.retain(|p| p.id != pid);
                            }
                        }
                    }
                    self.sync_current_selection();
                }

                if self.visual_mode {
                    self.exit_visual_mode();
                }
            }
            PickerContext::TagManagement { target_papers } => {
                let orig_checked_uuids = self.compute_tags_for_papers(&target_papers);
                let current_checked_uuids = picker.checked_items();

                let id_to_tag: HashMap<Uuid, String> = picker
                    .items
                    .iter()
                    .map(|it| (it.id, it.title.clone()))
                    .collect();

                let orig_tag_names: HashSet<String> = orig_checked_uuids
                    .iter()
                    .filter_map(|uid| id_to_tag.get(uid).cloned())
                    .collect();

                let current_tag_names: HashSet<String> = current_checked_uuids
                    .iter()
                    .filter_map(|uid| id_to_tag.get(uid).cloned())
                    .collect();

                let mut added: Vec<String> = current_tag_names
                    .difference(&orig_tag_names)
                    .cloned()
                    .collect();
                let removed: Vec<String> = orig_tag_names
                    .difference(&current_tag_names)
                    .cloned()
                    .collect();

                let query_tag = picker.query.trim().to_string();
                if !query_tag.is_empty()
                    && !picker
                        .items
                        .iter()
                        .any(|it| it.title.eq_ignore_ascii_case(&query_tag))
                    && !added.iter().any(|t| t.eq_ignore_ascii_case(&query_tag))
                {
                    added.push(query_tag);
                }

                if let Some(ref mut conn) = self.db_conn {
                    let _ = batch_set_tags(conn, &target_papers, &added, &removed);
                }
                if self.db_conn.is_some() {
                    let _ = self.reload_from_db();
                } else {
                    for &pid in &target_papers {
                        let tags = self.tags_by_paper.entry(pid).or_default();
                        for a in &added {
                            if !tags.contains(a) {
                                tags.push(a.clone());
                            }
                        }
                        tags.retain(|t| !removed.contains(t));
                    }
                }

                if self.visual_mode {
                    self.exit_visual_mode();
                }
            }
        }

        self.active_picker = None;
        self.picker_context = None;
        self.needs_clear = true;
    }

    /// Cancels the active picker modal, clearing picker state.
    pub fn cancel_active_picker(&mut self) {
        self.active_picker = None;
        self.picker_context = None;
        self.needs_clear = true;
    }
}
