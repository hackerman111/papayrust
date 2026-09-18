# TUI Usability Overhaul, Fullscreen TOC, Tags, Search, and Deletion Implementation Plan

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Provide a dedicated fullscreen scrollable Table of Contents viewer with jump-to-page, paper tagging with SQLite persistence and search, collection/paper deletion dialogs, title & tag search with collection cycling, clean status bar, help modal (`?`), and initial focus on Papers.

**Architecture:**
1. Database: `tags` and `paper_tags` tables with `TagRepo` in `papyrus-core`.
2. Core Actions: Fullscreen TOC, Tag editing, Deletion confirmation, Search collection cycling, and Help modal.
3. TUI App & Dispatch: `tags_by_paper` caching, deletion logic with DB and Tantivy sync, search filtering by title & tags across selected collections, and viewport scrolling for TOC.
4. UI & Rendering: Dedicated fullscreen TOC renderer with viewport offset calculation, help modal, tag modal, delete confirmation modal, and context-sensitive status bar.

**Tech Stack:** Rust 2021, Rusqlite, Ratatui, Crossterm, Tantivy, Uuid.

---

### Task 1: Tag Repository & Database Schema

**Files:**
- Create: `crates/papyrus-core/src/db/tag_repo.rs`
- Modify: `crates/papyrus-core/src/db/mod.rs`
- Modify: `crates/papyrus-core/src/db/migration.rs`
- Modify: `crates/papyrus-core/src/db/connection.rs`
- Test: `crates/papyrus-core/src/db/tag_repo.rs`

**Step 1: Write the failing test**
In `crates/papyrus-core/src/db/tag_repo.rs`:
Add unit tests for `TagRepo::set_tags_for_paper`, `TagRepo::get_tags_for_paper`, `TagRepo::get_all_tags`, and deleting a paper cascading to `paper_tags`.

**Step 2: Run test to verify**
Run `cargo test -p papyrus-core --lib db::tag_repo`

**Step 3: Implement TagRepo & ensure tables exist**
Implement schema creation for `tags` and `paper_tags` (without altering migration version 1 count to preserve `RT-04`). Implement `TagRepo` with CRUD operations.

**Step 4: Run test to verify it passes**
Run `cargo test -p papyrus-core --lib db::tag_repo`

---

### Task 2: Core Action Additions & Initial Focus

**Files:**
- Modify: `crates/papyrus-core/src/action.rs`
- Modify: `crates/papyrus-tui/src/app/mod.rs`
- Test: `crates/papyrus-tui/src/app/tests.rs`

**Step 1: Write test for initial panel focus**
In `crates/papyrus-tui/src/app/tests.rs`: assert `app.active_panel == ActivePanel::Papers`.

**Step 2: Add actions and update App initial panel**
In `crates/papyrus-core/src/action.rs`, add:
- `OpenFullscreenToc`, `CloseFullscreenToc`
- `EditTagsModalOpen`, `EditTagsModalInput(char)`, `EditTagsModalBackspace`, `EditTagsModalConfirm`, `EditTagsModalCancel`
- `DeleteConfirmOpen`, `DeleteConfirmExecute`, `DeleteConfirmCancel`
- `SearchCycleCollection`
- `HelpModalToggle`
In `crates/papyrus-tui/src/app/mod.rs`:
Set default `active_panel: ActivePanel::Papers`. Add state fields for fullscreen TOC, tags editing, deletion modal, help modal, and tags cache.

**Step 3: Run test to verify it passes**
Run `cargo test -p papyrus-tui --lib app::tests`

---

### Task 3: Fullscreen Table of Contents View & Scrolling Fix

**Files:**
- Modify: `crates/papyrus-tui/src/event.rs`
- Modify: `crates/papyrus-tui/src/app/dispatch.rs`
- Modify: `crates/papyrus-tui/src/ui/panels.rs`
- Modify: `crates/papyrus-tui/src/ui/mod.rs`
- Test: `crates/papyrus-tui/src/app/tests.rs`

**Step 1: Write test for TOC fullscreen toggle and navigation**
Test entering fullscreen TOC via `t`, navigating with `j`/`k`, opening at page via `Enter`, and exiting via `Esc`/`t`/`q`.

**Step 2: Implement Fullscreen TOC view and scroll calculations**
- In `crates/papyrus-tui/src/ui/panels.rs`: add `render_fullscreen_toc(app, frame, area)` with dynamic viewport offset based on `selected_toc` and `area.height`. Also fix scrolling in `render_details`.
- In `crates/papyrus-tui/src/event.rs`: map `t` in Papers/Details to `OpenFullscreenToc`; when in fullscreen TOC, route `j`/`k`, `Enter`, `Esc`/`q`/`t`.
- In `crates/papyrus-tui/src/app/dispatch.rs`: handle `OpenFullscreenToc`, `CloseFullscreenToc`, and page opening.

**Step 3: Run test to verify it passes**
Run `cargo test -p papyrus-tui --lib app::tests`

---

### Task 4: Paper Tags Management & Search by Title and Tags with Collection Cycling

**Files:**
- Modify: `crates/papyrus-tui/src/app/search.rs`
- Modify: `crates/papyrus-tui/src/app/dispatch.rs`
- Modify: `crates/papyrus-tui/src/event.rs`
- Modify: `crates/papyrus-tui/src/ui/modals.rs`
- Modify: `crates/papyrus-tui/src/ui/panels.rs`
- Modify: `crates/papyrus-tui/src/ui/search_bar.rs`
- Test: `crates/papyrus-tui/src/app/tests.rs`

**Step 1: Write test for tags editing and tag-based search**
Test editing tags via `T` modal, saving to SQLite, and searching by title and tags with collection cycling.

**Step 2: Implement Tag Editing & Search Filtering**
- In `crates/papyrus-tui/src/ui/modals.rs`: add `render_edit_tags_modal`.
- In `crates/papyrus-tui/src/app/search.rs`: filter papers matching title or tags (including `#tag` or `tag:name` syntax) within the target collection. Support `Tab` to cycle target collection.
- In `crates/papyrus-tui/src/ui/search_bar.rs`: show the active search collection badge.

**Step 3: Run test to verify it passes**
Run `cargo test -p papyrus-tui --lib app::tests`

---

### Task 5: Deletion of Collections and Papers in TUI

**Files:**
- Modify: `crates/papyrus-tui/src/app/dispatch.rs`
- Modify: `crates/papyrus-tui/src/event.rs`
- Modify: `crates/papyrus-tui/src/ui/modals.rs`
- Test: `crates/papyrus-tui/src/app/tests.rs`

**Step 1: Write test for deleting a collection and paper**
Verify that pressing `d` opens delete confirmation modal, confirming deletes collection / paper and updates DB and Tantivy index.

**Step 2: Implement Deletion Flow**
- In `crates/papyrus-tui/src/event.rs`: map `d` in Collections and Papers to `DeleteConfirmOpen`.
- In `crates/papyrus-tui/src/app/dispatch.rs`: handle deletion with SQLite `CollectionRepo::delete` and `PaperRepo::delete`.
- In `crates/papyrus-tui/src/ui/modals.rs`: add `render_delete_confirm_modal`.

**Step 3: Run test to verify it passes**
Run `cargo test -p papyrus-tui --lib app::tests`

---

### Task 6: Contextual Status Bar & Help Modal (`?`)

**Files:**
- Modify: `crates/papyrus-tui/src/ui/status_bar.rs`
- Modify: `crates/papyrus-tui/src/ui/modals.rs`
- Modify: `crates/papyrus-tui/src/event.rs`
- Modify: `crates/papyrus-tui/src/ui/mod.rs`
- Test: `crates/papyrus-tui/src/app/tests.rs`

**Step 1: Write test for help modal toggle**
Verify pressing `?` toggles help modal.

**Step 2: Implement Help Modal & Contextual Status Bar**
- In `crates/papyrus-tui/src/ui/status_bar.rs`: clean, focused hotkey summary per panel.
- In `crates/papyrus-tui/src/ui/modals.rs`: `render_help_modal` with organized sections.
- In `crates/papyrus-tui/src/event.rs`: map `?` to `HelpModalToggle`.

---

### Task 7: Full Workspace Verification & Binary Build

**Step 1: Run workspace checks**
- `cargo check --workspace --all-targets`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`

**Step 2: Build release binary**
- `cargo build --release -p papyrus-cli && cp target/release/papyrus ./papyrus`
- Run `./papyrus doctor`
