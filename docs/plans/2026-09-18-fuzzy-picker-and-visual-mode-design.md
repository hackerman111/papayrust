# Design: Generic Picker, Quick Open, Collection Membership & Visual Mode

Date: 2026-09-18
Status: Approved

## 1. Overview & Goals

This design addresses three core capabilities from `Task.md`:
1. **Generic Picker with Fuzzy Matching** (`Task.md` Sections 9.2, 10, 13):
   - A reusable modal component `GenericPicker` supporting fast substring/subsequence fuzzy search.
   - **Quick Open (`Ctrl-p`)**: global search jumping directly to papers or collections.
   - **Collection Membership (`c`)**: quick assignment / toggling of collections for single or multiple papers.
   - **Tag Management (`t`)**: tag picker for single or multiple papers.
2. **Visual Mode for Multi-Select Operations** (`Task.md` Section 29):
   - `V` toggles visual selection mode in the `Papers` panel.
   - Range selection via `j`/`k` and motions; toggle individual items via `Space`.
   - Visual highlighting and checkboxes `[x]` in the papers list.
3. **Dedicated Batch Operations Layer** (`Task.md` Section 29):
   - Atomically execute batch mutations in SQLite within a single transaction:
     - Batch collection assignment/removal.
     - Batch tag assignment/removal.
     - Batch paper deletion.
   - Avoid sequential UI action looping.

---

## 2. Component Architecture

### 2.1. `crates/papyrus-tui/src/app/picker.rs`
Reusable modal picker:
```rust
#[derive(Debug, Clone)]
pub struct PickerItem {
    pub id: Uuid,
    pub title: String,
    pub subtitle: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GenericPicker {
    pub title: String,
    pub items: Vec<PickerItem>,
    pub visible_indices: Vec<usize>,
    pub selected: usize,
    pub query: String,
    pub checked: HashSet<Uuid>,
    pub multi_select: bool,
}
```
Key behaviors:
- `query` updates re-evaluate matching scores across `title`, `subtitle`, `category`.
- Navigation: `j`/`k`, `Down`/`Up`, `Ctrl-d`/`Ctrl-u` navigate `visible_indices`.
- `Space`: toggles `checked.insert` / `checked.remove` when `multi_select` is true.
- `Enter`: confirms selection (single item or checked set).
- `Esc`: cancels picker.

### 2.2. Modal States on `App`
- `active_picker: Option<GenericPicker>`
- `picker_mode: Option<PickerMode>`:
  - `QuickOpen`
  - `CollectionMembership { target_papers: Vec<Uuid> }`
  - `TagManagement { target_papers: Vec<Uuid> }`

### 2.3. Visual Mode on `App`
- `visual_mode: bool`
- `visual_anchor: Option<usize>`
- `visual_selected_uuids: HashSet<Uuid>`
Interactions:
- `V`: toggles mode. Sets anchor at current index.
- Cursor movements: dynamically updates selection between anchor and cursor.
- `Space`: toggles item under cursor.
- `Esc`: clears visual selection and exits visual mode.

### 2.4. Batch Operations Layer (`crates/papyrus-tui/src/app/batch.rs`)
Pure transactional DB routines:
- `batch_set_collections(conn: &mut Connection, paper_ids: &[Uuid], add: &[Uuid], remove: &[Uuid]) -> Result<()>`
- `batch_set_tags(conn: &mut Connection, paper_ids: &[Uuid], add: &[String], remove: &[String]) -> Result<()>`
- `batch_delete_papers(conn: &mut Connection, paper_ids: &[Uuid], search_index: &Option<SearchIndex>) -> Result<()>`

---

## 3. Keybindings Summary
- `Ctrl-p`: open Quick Open picker.
- `c`: open Collection Membership picker for current paper (or visual selection).
- `t`: open Tag Management picker for current paper (or visual selection).
- `V`: toggle Visual Mode in `Papers`.
- Inside Visual Mode:
  - `j`/`k`: expand/shrink selection range.
  - `Space`: toggle individual paper.
  - `c`: batch collections.
  - `t`: batch tags.
  - `d` / `Delete`: batch delete (with confirmation dialog).
  - `Esc`: exit visual mode.
- Inside Picker:
  - typing: filter query.
  - `j`/`k`/arrows: move selection.
  - `Space`: toggle checkbox (multi-select).
  - `Enter`: confirm.
  - `Esc`: cancel.

---

## 4. Performance & Safety Invariants
- Fuzzy matching performs zero heap allocations on static strings during scanning.
- Batch operations use single SQLite transactions with rollback on error.
- Search index updates are batched.
- Position memory and selection UUID are strictly preserved after batch updates.
