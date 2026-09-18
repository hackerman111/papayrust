# TUI Collections Creation and Recursive Folder Import Implementation Plan

> **For Antigravity:** REQUIRED SUB-SKILL: Load executing-plans to implement this plan task-by-task.

**Goal:** Enable collection (folder) creation in the TUI, support recursive bulk PDF folder import in the unified add modal, and eliminate terminal visual artifacts from `pdf-extract` output.

**Architecture:** 
1. `papyrus-core::pdf::text` provides scoped OS output suppression (redirecting fd 1 and 2 to `/dev/null` on Unix) during PDF text extraction, preventing third-party `println!` pollution. `App` adds `needs_clear: bool` to ensure `terminal.clear()` resets the screen buffer after imports.
2. `papyrus-core::Action` adds collection modal actions. `papyrus-tui` connects `a` in `ActivePanel::Collections` to `CreateCollectionModalOpen`, inserts collections via `CollectionRepo::insert`, reloads state, and focuses the new collection.
3. `papyrus-tui::app::dispatch::confirm_add_paper` checks `expanded_path.is_dir()`, recursively scans for `.pdf` files, imports each into the active collection with full-text indexing, and displays import statistics.

**Tech Stack:** Rust 2021, Ratatui 0.29, Crossterm 0.28, Rusqlite 0.32, Tantivy 0.22, Lopdf 0.34, pdf-extract 0.7.12, libc.

---

### Task 1: Silence output during PDF extraction & add terminal clear trigger

**Files:**
- Modify: `crates/papyrus-core/Cargo.toml` (ensure `libc` dependency if needed on unix)
- Modify: `crates/papyrus-core/src/pdf/text.rs`
- Modify: `crates/papyrus-tui/src/app/mod.rs`
- Modify: `crates/papyrus-tui/src/event.rs`
- Test: `crates/papyrus-core/src/pdf/text.rs` (unit test)

**Step 1: Write the failing test**
In `crates/papyrus-core/src/pdf/text.rs`:
Add a test verifying output suppression: writing to stdout inside the suppression scope does not print to real stdout.

**Step 2: Run test to verify**
Run `cargo test -p papyrus-core --lib pdf::text`

**Step 3: Implement output suppression in `extract_text` and `extract_text_from_mem`**
In `crates/papyrus-core/src/pdf/text.rs`:
Implement `suppress_output<F, R>(f: F) -> R` using `libc::dup` and `libc::dup2` to `/dev/null` with `// SAFETY:` invariant comments.
Wrap `pdf_extract::extract_text` and `pdf_extract::extract_text_from_mem` inside `suppress_output`.
In `crates/papyrus-tui/src/app/mod.rs`:
Add `pub needs_clear: bool` field to `App`.
In `crates/papyrus-tui/src/event.rs`:
In `run_app`, before `terminal.draw`, check:
```rust
if app.needs_clear {
    terminal.clear()?;
    app.needs_clear = false;
}
```

**Step 4: Run test to verify it passes**
Run `cargo test -p papyrus-core --lib pdf::text`

---

### Task 2: Implement TUI Collection Creation

**Files:**
- Modify: `crates/papyrus-core/src/action.rs`
- Modify: `crates/papyrus-tui/src/app/mod.rs`
- Modify: `crates/papyrus-tui/src/event.rs`
- Modify: `crates/papyrus-tui/src/app/dispatch.rs`
- Modify: `crates/papyrus-tui/src/ui/modals.rs`
- Modify: `crates/papyrus-tui/src/ui/status_bar.rs`
- Modify: `crates/papyrus-tui/src/ui/mod.rs`
- Test: `crates/papyrus-tui/src/app/tests.rs`

**Step 1: Write the failing test**
In `crates/papyrus-tui/src/app/tests.rs`:
Test that pressing `a` on `ActivePanel::Collections` produces `Action::CreateCollectionModalOpen`, typing inputs characters, and `CreateCollectionModalConfirm` inserts the collection into the database and updates `app.collections`.

**Step 2: Run test to verify it fails**
Run `cargo test -p papyrus-tui --lib app::tests`

**Step 3: Implement Actions and Collection Creation Flow**
1. Add `CreateCollectionModalOpen`, `CreateCollectionModalInput(char)`, `CreateCollectionModalBackspace`, `CreateCollectionModalConfirm`, `CreateCollectionModalCancel` to `Action`.
2. Add `is_creating_collection: bool` and `collection_name_buffer: String` to `App`.
3. In `crates/papyrus-tui/src/event.rs`, handle modal keystrokes when `app.is_creating_collection`, and map `KeyCode::Char('a')` to `Action::CreateCollectionModalOpen` when `active_panel == ActivePanel::Collections`.
4. In `crates/papyrus-tui/src/app/dispatch.rs`, handle `CreateCollectionModal*` actions, calling `CollectionRepo::insert(&conn, &NewCollection { name, parent_id: None })`, reloading from DB, and selecting the new collection.
5. In `crates/papyrus-tui/src/ui/modals.rs`, render `render_create_collection_modal`.
6. In `crates/papyrus-tui/src/ui/status_bar.rs`, display `a: New collection` when `active_panel == ActivePanel::Collections`.

**Step 4: Run test to verify it passes**
Run `cargo test -p papyrus-tui --lib app::tests`

---

### Task 3: Implement Recursive Folder PDF Import

**Files:**
- Modify: `crates/papyrus-tui/src/app/dispatch.rs`
- Modify: `crates/papyrus-tui/src/ui/modals.rs`
- Modify: `crates/papyrus-tui/src/ui/status_bar.rs`
- Test: `crates/papyrus-tui/src/app/tests.rs`

**Step 1: Write the failing test**
In `crates/papyrus-tui/src/app/tests.rs`:
Test importing a folder containing multiple PDFs (including subdirectories) into a collection. Verify that all PDFs are imported, duplicates are skipped, and status message reports correct counts.

**Step 2: Run test to verify it fails**
Run `cargo test -p papyrus-tui --lib app::tests`

**Step 3: Implement Recursive Directory Import in `confirm_add_paper`**
1. Update `confirm_add_paper` in `crates/papyrus-tui/src/app/dispatch.rs`:
   - If `expanded_path.is_dir()`:
     - Recursively collect `.pdf` paths using standard library `std::fs::read_dir`.
     - For each file, import via `papyrus_core::importer::import_paper`.
     - Index text in Tantivy.
     - Count successes and duplicates.
     - Call `reload_from_db()` and set `app.needs_clear = true`.
     - Set informative status message.
2. Update UI labels in `render_add_paper_modal` in `crates/papyrus-tui/src/ui/modals.rs`:
   - Title: `Add Paper or Folder to Library`
   - Input title: `Path to PDF file or Directory`
   - Placeholder: `/path/to/paper.pdf or /path/to/folder`

**Step 4: Run test to verify it passes**
Run `cargo test -p papyrus-tui --lib app::tests`

---

### Task 4: System Verification & Release Build

**Step 1: Workspace Checks**
Run:
- `cargo check --workspace --all-targets`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`

**Step 2: Rebuild Release Binary**
Run `cargo build --release` and copy binary to `./papyrus`.
