# Design Document: Phase A — Stable Selection & Position Memory

Date: 2026-09-18
Status: Approved

## 1. Goal & Context

According to `Task.md` (sections 2, 34, 35 Phase A, and 36), the goal is to decouple the user selection state from raw list indices in Papyrus TUI.
List indices (`selected_collection`, `selected_paper`, `selected_toc`) are prone to invalidation upon sorting, tree collapse/expand, database reload, deletion, and filtering.

Phase A establishes an authoritative identity-based state model using UUIDs and strongly-typed keys, while maintaining list indices as a pure derived UI cache for Ratatui rendering.

## 2. Architecture & Data Structures

### 2.1. Module: `crates/papyrus-tui/src/app/selection.rs`

Define the authoritative selection types:

```rust
use std::collections::HashMap;
use uuid::Uuid;

/// Identity key for collections (real DB collections and virtual collections).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CollectionKey {
    All,
    Real(Uuid),
    RecentlyAdded,
    Unfiled,
    Untagged,
}

impl CollectionKey {
    /// Returns the UUID if this is a real DB collection.
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            Self::Real(id) => Some(*id),
            _ => None,
        }
    }
}

/// Authoritative selection state (Single Source of Truth).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionState {
    pub collection: CollectionKey,
    pub paper_id: Option<Uuid>,
    pub toc_id: Option<Uuid>,
}

impl Default for SelectionState {
    fn default() -> Self {
        Self {
            collection: CollectionKey::All,
            paper_id: None,
            toc_id: None,
        }
    }
}
```

### 2.2. Updating `CollectionItem` (`crates/papyrus-tui/src/app/panel.rs`)

`CollectionItem` is enhanced with `pub key: CollectionKey` instead of relying solely on `Option<Uuid>`:

```rust
pub struct CollectionItem {
    pub key: CollectionKey,
    pub name: String,
    pub paper_count: usize,
    pub depth: usize,
    pub parent_id: Option<Uuid>,
}

impl CollectionItem {
    pub fn id(&self) -> Option<Uuid> {
        self.key.as_uuid()
    }
}
```

Constructors `CollectionItem::new` and `with_hierarchy` are updated (with backward-compatible helpers accepting `Option<Uuid>` or `CollectionKey`).

### 2.3. Fields in `App` (`crates/papyrus-tui/src/app/mod.rs`)

Add to `App`:
- `pub selection: SelectionState`
- `pub last_paper_by_collection: HashMap<CollectionKey, Uuid>`
- `pub last_toc_by_paper: HashMap<Uuid, Uuid>`
- Existing `pub selected_collection: usize`, `pub selected_paper: usize`, `pub selected_toc: usize` remain as derived UI rendering caches.

## 3. Position Memory & Selection Synchronization

### 3.1. Transition Memory
- When leaving a collection: if a paper is selected, record its UUID in `last_paper_by_collection.insert(collection_key, paper_id)`.
- When leaving a paper: if a TOC entry is selected, record its UUID in `last_toc_by_paper.insert(paper_id, toc_id)`.

### 3.2. Synchronization Cycle
- `update_selection_from_indices(&mut self)`:
  - Updates `selection.collection`, `selection.paper_id`, `selection.toc_id` from the current UI indices.
  - Updates `last_paper_by_collection` and `last_toc_by_paper`.
- `restore_selection_by_uuid(&mut self)`:
  - Locates `selection.collection` in `collections`. Falls back to index clamp or `All` if missing.
  - Synchronizes papers for collection.
  - Locates paper by UUID prioritizing:
    1. Current `selection.paper_id` if present in collection.
    2. Remembered `last_paper_by_collection.get(&collection)`.
    3. Fallback clamp to `selected_paper.min(papers.len() - 1)`.
  - Synchronizes TOC preview.
  - Locates TOC by UUID prioritizing:
    1. Current `selection.toc_id` if present.
    2. Remembered `last_toc_by_paper.get(&paper_id)`.
    3. Fallback clamp to `selected_toc.min(toc.len() - 1)`.

### 3.3. Graceful Deletion & Fallback
- Deleting an element clamps index to `len - 1` without panicking.
- If list becomes empty, index becomes 0 and selected UUID becomes `None`.
- Memory maps are pruned of deleted UUIDs.

## 4. Database Reload & Event Dispatch Integration

- `reload_from_db` calls `restore_selection_by_uuid()` instead of indexing by previous raw `usize`.
- `dispatch.rs` key event handling updates position memory and selection state on navigation.
- Paper sorting maintains selected paper by UUID.

## 5. Verification Plan

- Unit tests in `crates/papyrus-tui/src/app/selection.rs`:
  - `CollectionKey` and `SelectionState` construction.
  - Reordering keeps selection on same UUID.
  - Position memory across collection switches.
  - Position memory across paper TOC switches.
  - Deletion fallback without out-of-bounds index.
- Integration tests with SQLite in `crates/papyrus-tui/src/app/tests.rs`:
  - Reloading DB preserves selection by UUID.
- Full workspace checks:
  - `cargo check --workspace --all-targets`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
