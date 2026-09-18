# Phase A: Stable Selection & Position Memory Implementation Plan

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Decouple Papyrus TUI selection state from fragile list indices by introducing `CollectionKey`, authoritative `SelectionState`, position memory maps (`last_paper_by_collection`, `last_toc_by_paper`), and automatic index recomputation upon reload, reordering, and deletion.

**Architecture:** A dedicated module `crates/papyrus-tui/src/app/selection.rs` houses `CollectionKey` and `SelectionState`. `CollectionItem` binds to `CollectionKey`. `App` maintains authoritative selection state and position memory maps, keeping `selected_collection`, `selected_paper`, and `selected_toc` purely as derived UI cache indices for Ratatui rendering.

**Tech Stack:** Rust (edition 2021), Ratatui, Crossterm, Rusqlite, Uuid.

---

## Task Packet (as per .agent/MULTI_AGENT.md)

```text
TASK
User goal: Implement Phase A (Stable Selection) from Task.md.
Acceptance criteria:
- CollectionKey enum introduced: All, Real(Uuid), RecentlyAdded, Unfiled, Untagged.
- SelectionState is single source of truth: collection, paper_id, toc_id.
- selected_* usize fields are preserved as synchronized UI cache for rendering.
- last_paper_by_collection remembers paper position per collection.
- last_toc_by_paper remembers TOC position per paper.
- Reorder/sorting preserves selected paper by UUID.
- Reload from DB preserves selection by UUID.
- Deleting selected item falls back gracefully to nearest item or empty state without panic or out-of-bounds index.
- Full test suite passes without regressions.
Non-goals:
- Phase B navigation features (S sort key, gg/G, Ctrl-w) are out of scope for this plan.
- Markdown notes, fuzzy picker, search enhancements are deferred to later phases.
Repository constraints:
- Must follow AGENTS.md and .agent/*.md: no unnecessary clones, strict ownership, clear module boundaries.

SCOPE
Owning module/crate: crates/papyrus-tui
Relevant callers/tests: crates/papyrus-tui/src/app/tests.rs, tests/phase3_tui.rs
Files likely affected:
- crates/papyrus-tui/src/app/selection.rs (NEW)
- crates/papyrus-tui/src/app/panel.rs
- crates/papyrus-tui/src/app/mod.rs
- crates/papyrus-tui/src/app/db_sync.rs
- crates/papyrus-tui/src/app/dispatch.rs
- crates/papyrus-tui/src/app/tests.rs

INVARIANTS
Behavioral invariants:
- Any cursor movement updates both UI index and SelectionState + position memory.
- Any reload or reorder recalculates UI index based on SelectionState UUIDs.
- Deletion never panics; clamps index to remaining bounds.
Data/ownership invariants:
- CollectionKey is lightweight, Copy/Clone, Hash, Eq.
- No redundant allocations in hot rendering path.

PERFORMANCE CONTRACT
Performance-sensitive: yes (selection sync runs on every key press and frame)
Hot paths: sync_current_selection, restore_selection_by_uuid, update_selection_from_indices
Expected input scale: 100s to 1,000s of papers/collections
Baseline measurements: cargo test runs in ~0.04s
Optimization ideas/hypotheses:
- Use HashMap lookup for UUID-to-index or sequential scan for typical size (<1000 items). Sequential scan on contiguous Vec is cache-friendly and avoids overhead.
Costs that must not regress: Frame render latency and test suite runtime.
Measurement required before completion: cargo test --workspace

VERIFICATION
Focused checks: cargo test -p papyrus-tui
Workspace checks: cargo check --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

---

### Task 1: Create `app/selection.rs` with `CollectionKey` and `SelectionState`

**Files:**
- Create: `crates/papyrus-tui/src/app/selection.rs`
- Modify: `crates/papyrus-tui/src/app/mod.rs:1-15`

**Step 1: Write the failing test**
Create `crates/papyrus-tui/src/app/selection.rs` with unit tests for `CollectionKey`, `SelectionState`, and `as_uuid()`.

**Step 2: Run test to verify it fails**
Run: `cargo test -p papyrus-tui selection`
Expected: Compilation failure or not found.

**Step 3: Write minimal implementation**
Implement `CollectionKey`, `SelectionState`, and register `pub mod selection;` in `crates/papyrus-tui/src/app/mod.rs`.

**Step 4: Run test to verify it passes**
Run: `cargo test -p papyrus-tui selection`
Expected: PASS.

**Step 5: Commit**
Run:
```bash
git add crates/papyrus-tui/src/app/selection.rs crates/papyrus-tui/src/app/mod.rs
git commit -m "feat(tui): introduce CollectionKey and SelectionState in selection module"
```

---

### Task 2: Update `CollectionItem` with `CollectionKey`

**Files:**
- Modify: `crates/papyrus-tui/src/app/panel.rs`
- Modify: `crates/papyrus-tui/src/app/db_sync.rs`

**Step 1: Write the failing test**
In `crates/papyrus-tui/src/app/tests.rs`, add a test verifying `CollectionItem` construction with `CollectionKey`.

**Step 2: Run test to verify it fails**
Run: `cargo test -p papyrus-tui test_collection_item`
Expected: FAIL.

**Step 3: Write minimal implementation**
Update `CollectionItem`:
- `pub key: CollectionKey`
- helper `pub fn id(&self) -> Option<Uuid>`
- update constructors `new` and `with_hierarchy` to accept `CollectionKey` (or maintain backward compatibility).
- update `build_hierarchical_collection_items` in `db_sync.rs` to generate `CollectionKey::All` and `CollectionKey::Real(col.id)`.

**Step 4: Run test to verify it passes**
Run: `cargo test -p papyrus-tui`
Expected: PASS.

**Step 5: Commit**
Run:
```bash
git add crates/papyrus-tui/src/app/panel.rs crates/papyrus-tui/src/app/db_sync.rs crates/papyrus-tui/src/app/tests.rs
git commit -m "feat(tui): integrate CollectionKey into CollectionItem"
```

---

### Task 3: Add `SelectionState` and Position Memory to `App`

**Files:**
- Modify: `crates/papyrus-tui/src/app/mod.rs`
- Modify: `crates/papyrus-tui/src/app/selection.rs`

**Step 1: Write the failing test**
In `crates/papyrus-tui/src/app/selection.rs`, add unit tests for `update_selection_from_indices`, `restore_selection_by_uuid`, and position memory.

**Step 2: Run test to verify it fails**
Run: `cargo test -p papyrus-tui selection`
Expected: FAIL.

**Step 3: Write minimal implementation**
Add to `App`:
- `pub selection: SelectionState`
- `pub last_paper_by_collection: HashMap<CollectionKey, Uuid>`
- `pub last_toc_by_paper: HashMap<Uuid, Uuid>`
Implement methods on `App`:
- `pub fn update_selection_from_indices(&mut self)`
- `pub fn restore_selection_by_uuid(&mut self)`
- `pub fn record_position_for_current_collection(&mut self)`
- `pub fn record_position_for_current_paper(&mut self)`

**Step 4: Run test to verify it passes**
Run: `cargo test -p papyrus-tui selection`
Expected: PASS.

**Step 5: Commit**
Run:
```bash
git add crates/papyrus-tui/src/app/mod.rs crates/papyrus-tui/src/app/selection.rs
git commit -m "feat(tui): add selection state, position memory maps and sync methods to App"
```

---

### Task 4: Integrate Selection Restoration into `db_sync.rs`

**Files:**
- Modify: `crates/papyrus-tui/src/app/db_sync.rs`
- Modify: `crates/papyrus-tui/src/app/tests.rs`

**Step 1: Write the failing test**
Add `test_db_reload_preserves_selection_by_uuid` in `crates/papyrus-tui/src/app/tests.rs`.

**Step 2: Run test to verify it fails**
Run: `cargo test -p papyrus-tui test_db_reload_preserves_selection_by_uuid`
Expected: FAIL.

**Step 3: Write minimal implementation**
In `db_sync.rs`:
- Update `load_from_db_options` to initialize `SelectionState`.
- Update `reload_from_db` to call `restore_selection_by_uuid()`.
- Update `sync_current_selection` to consult `last_paper_by_collection`.
- Update `sync_paper_selection` to consult `last_toc_by_paper`.

**Step 4: Run test to verify it passes**
Run: `cargo test -p papyrus-tui test_db_reload_preserves_selection_by_uuid`
Expected: PASS.

**Step 5: Commit**
Run:
```bash
git add crates/papyrus-tui/src/app/db_sync.rs crates/papyrus-tui/src/app/tests.rs
git commit -m "feat(tui): preserve selection by UUID during database reload and sync"
```

---

### Task 5: Integrate Position Memory into `dispatch.rs`

**Files:**
- Modify: `crates/papyrus-tui/src/app/dispatch.rs`

**Step 1: Write the failing test**
In `crates/papyrus-tui/src/app/tests.rs`, add tests for position memory when navigating collections and papers via key events.

**Step 2: Run test to verify it fails**
Run: `cargo test -p papyrus-tui test_position_memory`
Expected: FAIL.

**Step 3: Write minimal implementation**
In `dispatch.rs`:
- When navigating Collections (`Down`/`Up`/etc.), record current paper before switching, and restore memory when switching.
- When navigating Papers, record TOC position before switching, and restore memory when switching.
- Call `update_selection_from_indices()` on list cursor moves.
- When deleting a paper or collection, clean up position memory and gracefully clamp indices.

**Step 4: Run test to verify it passes**
Run: `cargo test -p papyrus-tui test_position_memory`
Expected: PASS.

**Step 5: Commit**
Run:
```bash
git add crates/papyrus-tui/src/app/dispatch.rs crates/papyrus-tui/src/app/tests.rs
git commit -m "feat(tui): track position memory and update selection in event dispatcher"
```

---

### Task 6: Comprehensive Reorder, Delete, and Fallback Tests

**Files:**
- Modify: `crates/papyrus-tui/src/app/tests.rs`

**Step 1: Write test cases**
- `test_reorder_preserves_selection_by_uuid`
- `test_delete_fallback_selection`
- `test_position_memory_roundtrip`

**Step 2: Run tests to verify they pass**
Run: `cargo test -p papyrus-tui`
Expected: PASS.

**Step 3: Commit**
Run:
```bash
git add crates/papyrus-tui/src/app/tests.rs
git commit -m "test(tui): add comprehensive tests for UUID selection, reordering, and fallback"
```

---

### Task 7: Full Workspace Verification

**Step 1: Run all verification commands**
- `cargo check --workspace --all-targets`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

**Step 2: Commit any final cleanup**
Run:
```bash
git status
```
Expected: Clean working tree.
