# Generic Picker, Quick Open, Collection Membership & Visual Mode Implementation Plan

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Implement a reusable `GenericPicker` with fuzzy filtering (`Ctrl-p` Quick Open, `c` Collection Membership, `t` Tag Management) and Vim-style Visual Mode (`V`) with an atomic transactional batch operations layer.

**Architecture:**
- `app/picker.rs`: Generic reusable picker component with fuzzy subsequence matching, scoring, and multi-select checkboxes.
- `app/batch.rs`: Pure SQLite transactional layer for batch collection, tag, and deletion operations.
- `app/mod.rs` & `app/dispatch.rs`: Visual mode state (`visual_mode`, `visual_anchor`, `visual_selected_uuids`), picker modal lifecycle, and batch dispatch.
- `event.rs`: Key mappings for `Ctrl-p`, `c`, `t`, `V`, and modal event capture.
- `ui/modals.rs`, `ui/panels.rs`, `ui/status_bar.rs`: Modal rendering for pickers, row selection highlights in Papers, and visual mode status bar indicators.

**Tech Stack:** Rust, Ratatui, Crossterm, Rusqlite, Tantivy.

---

### Task 1: Reusable GenericPicker & Fuzzy Subsequence Matching

**Files:**
- Modify: `crates/papyrus-core/src/action.rs`
- Create: `crates/papyrus-tui/src/app/picker.rs`
- Modify: `crates/papyrus-tui/src/app/mod.rs`

**Step 1: Write unit tests for GenericPicker & fuzzy matching**
Create `crates/papyrus-tui/src/app/picker.rs` with tests verifying:
- Filtering empty query returns all items.
- Exact and subsequence matches are ranked by score.
- Navigation (`j`/`k`, `Down`/`Up`, `Ctrl-d`/`Ctrl-u`) clamps within visible bounds.
- Multi-select toggling via `Space` updates `checked` set.

**Step 2: Run tests to verify failure**
Run: `cargo test -p papyrus-tui app::picker`
Expected: FAIL (module not yet registered)

**Step 3: Implement GenericPicker & fuzzy matching**
Implement:
- `PickerItem { id: Uuid, title: String, subtitle: Option<String>, category: Option<String> }`
- `GenericPicker`:
  - `new(title, items, multi_select)`
  - `set_query(query)`
  - `move_cursor(delta)`
  - `toggle_selected()`
  - `selected_item() -> Option<&PickerItem>`
  - `checked_items() -> &HashSet<Uuid>`
- Register `pub mod picker;` in `crates/papyrus-tui/src/app/mod.rs`.
- Add picker actions to `crates/papyrus-core/src/action.rs`.

**Step 4: Run tests to verify pass**
Run: `cargo test -p papyrus-tui app::picker`
Expected: PASS

**Step 5: Commit**
```bash
git add crates/papyrus-core/src/action.rs crates/papyrus-tui/src/app/picker.rs crates/papyrus-tui/src/app/mod.rs
git commit -m "feat(tui): implement reusable GenericPicker with fuzzy matching"
```

---

### Task 2: Transactional Batch Operations Layer

**Files:**
- Create: `crates/papyrus-tui/src/app/batch.rs`
- Modify: `crates/papyrus-tui/src/app/mod.rs`

**Step 1: Write unit tests for batch operations**
Create `crates/papyrus-tui/src/app/batch.rs` with tests using in-memory SQLite:
- `test_batch_set_collections`: adds and removes papers across multiple collections in one transaction.
- `test_batch_set_tags`: updates tags across multiple papers atomically.
- `test_batch_delete_papers`: deletes multiple papers and cascades to associations cleanly.

**Step 2: Run tests to verify failure**
Run: `cargo test -p papyrus-tui app::batch`
Expected: FAIL

**Step 3: Implement batch operations**
Implement:
- `pub fn batch_set_collections(conn: &mut rusqlite::Connection, paper_ids: &[uuid::Uuid], add_cols: &[uuid::Uuid], remove_cols: &[uuid::Uuid]) -> Result<(), rusqlite::Error>`
- `pub fn batch_set_tags(conn: &mut rusqlite::Connection, paper_ids: &[uuid::Uuid], add_tags: &[String], remove_tags: &[String]) -> Result<(), rusqlite::Error>`
- `pub fn batch_delete_papers(conn: &mut rusqlite::Connection, paper_ids: &[uuid::Uuid]) -> Result<(), rusqlite::Error>`
- Register `pub mod batch;` in `crates/papyrus-tui/src/app/mod.rs`.

**Step 4: Run tests to verify pass**
Run: `cargo test -p papyrus-tui app::batch`
Expected: PASS

**Step 5: Commit**
```bash
git add crates/papyrus-tui/src/app/batch.rs crates/papyrus-tui/src/app/mod.rs
git commit -m "feat(tui): implement transactional batch operations layer"
```

---

### Task 3: Visual Mode State & Dispatch Integration

**Files:**
- Modify: `crates/papyrus-tui/src/app/mod.rs`
- Modify: `crates/papyrus-tui/src/app/dispatch.rs`
- Modify: `crates/papyrus-tui/src/app/navigation.rs`
- Modify: `crates/papyrus-tui/src/event.rs`

**Step 1: Write unit tests for Visual Mode**
In `crates/papyrus-tui/src/app/tests.rs`:
- `test_visual_mode_toggle_and_range_expansion`: pressing `V` toggles mode, `j`/`k` expands selection between anchor and cursor.
- `test_visual_mode_space_toggle`: `Space` toggles individual items in selection set.
- `test_visual_mode_esc_cancel`: `Esc` exits visual mode and clears selection.

**Step 2: Run tests to verify failure**
Run: `cargo test -p papyrus-tui test_visual_mode`
Expected: FAIL

**Step 3: Implement Visual Mode**
- Add fields to `App`:
  - `pub visual_mode: bool`
  - `pub visual_anchor: Option<usize>`
  - `pub visual_selected_uuids: std::collections::HashSet<uuid::Uuid>`
- Update `apply_motion` to update `visual_selected_uuids` when `visual_mode` is true.
- Wire `V`, `Space`, `Esc` in `event.rs` and `dispatch.rs`.

**Step 4: Run tests to verify pass**
Run: `cargo test -p papyrus-tui test_visual_mode`
Expected: PASS

**Step 5: Commit**
```bash
git add crates/papyrus-tui/src/app/ crates/papyrus-tui/src/event.rs
git commit -m "feat(tui): add visual mode selection and motion integration"
```

---

### Task 4: Quick Open (`Ctrl-p`), Collection Membership (`c`), and Tag Management (`t`)

**Files:**
- Modify: `crates/papyrus-tui/src/app/mod.rs`
- Modify: `crates/papyrus-tui/src/app/dispatch.rs`
- Modify: `crates/papyrus-tui/src/event.rs`

**Step 1: Write unit tests for pickers**
In `crates/papyrus-tui/src/app/tests.rs`:
- `test_quick_open_jump_to_paper_and_collection`: `Ctrl-p` opens picker, selecting paper jumps to it.
- `test_collection_membership_picker_single_and_batch`: `c` opens picker, toggling collections updates associations.
- `test_tag_picker_single_and_batch`: `t` updates tags.

**Step 2: Run tests to verify failure**
Run: `cargo test -p papyrus-tui test_quick_open`
Expected: FAIL

**Step 3: Implement picker modal lifecycles**
- Add `active_picker: Option<GenericPicker>` and `picker_context: Option<PickerContext>` to `App`.
- Implement `open_quick_open(&mut self)`, `open_collection_membership(&mut self)`, `open_tag_picker(&mut self)`.
- Implement confirmation handlers routing through `batch.rs` and single-item repos.
- Map keys in `event.rs`.

**Step 4: Run tests to verify pass**
Run: `cargo test -p papyrus-tui test_quick_open`
Expected: PASS

**Step 5: Commit**
```bash
git add crates/papyrus-tui/src/app/ crates/papyrus-tui/src/event.rs
git commit -m "feat(tui): wire quick open, collection membership, and tag pickers"
```

---

### Task 5: UI Rendering for Pickers, Visual Selection, and Batch Confirmation

**Files:**
- Modify: `crates/papyrus-tui/src/ui/modals.rs`
- Modify: `crates/papyrus-tui/src/ui/panels.rs`
- Modify: `crates/papyrus-tui/src/ui/status_bar.rs`
- Modify: `crates/papyrus-tui/src/ui/mod.rs`

**Step 1: Implement UI rendering**
- In `ui/modals.rs`: `render_generic_picker` with search input, result counts, category badges, and checkbox marks `[x]`.
- In `ui/panels.rs`: render `[x]` marks and background highlight on rows contained in `app.visual_selected_uuids`.
- In `ui/status_bar.rs`: display `-- VISUAL (N selected) --` in bold yellow/cyan when `app.visual_mode == true`.
- In `ui/mod.rs`: conditionally draw `render_generic_picker` when `app.active_picker.is_some()`.

**Step 2: Run rendering integration tests**
Run: `cargo test -p papyrus-tui`
Expected: PASS

**Step 3: Commit**
```bash
git add crates/papyrus-tui/src/ui/
git commit -m "feat(tui): render generic picker modal, visual row highlights, and status badge"
```

---

### Task 6: Full Verification and Clippy Cleanliness

**Files:**
- Run workspace checks and formatting.

**Step 1: Run complete test suite**
Run: `cargo test --workspace`
Expected: PASS (all unit, integration, and doc tests)

**Step 2: Run clippy**
Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: PASS (0 warnings)

**Step 3: Run fmt**
Run: `cargo fmt --all -- --check`
Expected: PASS (clean formatting)

**Step 4: Commit any remaining test additions**
```bash
git commit --allow-empty -m "chore: verify full workspace tests and lints"
```
