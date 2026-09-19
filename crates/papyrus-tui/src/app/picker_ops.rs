use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::app::batch::{batch_set_collections, batch_set_tags};
use crate::app::selection::CollectionKey;
use crate::app::{ActivePanel, App, GenericPicker, PickerItem};

/// Context and target metadata for an active generic picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerContext {
    QuickOpen,
    CollectionMembership { target_papers: Vec<Uuid> },
    TagManagement { target_papers: Vec<Uuid> },
}

/// Computes a deterministic UUID for a tag name using UUIDv5.
pub fn tag_to_uuid(tag: &str) -> Uuid {
    Uuid::new_v5(&Uuid::NAMESPACE_OID, tag.as_bytes())
}

impl App {
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
