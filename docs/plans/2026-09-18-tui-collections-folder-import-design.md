# Design: TUI Collections Creation, Recursive Folder Import, and Terminal Artifact Elimination

**Date:** 2026-09-18  
**Status:** Approved

## 1. Problem Statement
1. **No Collection Creation in TUI**: Users cannot create folders/collections from within the TUI; collections could previously only be created via CLI or database seeds.
2. **No Bulk / Folder PDF Import**: Users could only add one PDF file at a time by entering its file path in the TUI. Adding an entire folder of PDFs required manual entry for each file.
3. **Terminal Visual Artifacts on Add**: During paper import, full-text extraction invokes `pdf-extract-0.7.12`, which issues uncoordinated `println!("unknown glyph name...")` calls to `stdout`. In raw TUI mode, this writes text arbitrarily across the screen, breaking ratatui's view and scrolling the buffer.

## 2. Architecture & Approach

### 2.1 Silencing Output during PDF Extraction & Terminal Clean Redraw
- In `crates/papyrus-core/src/pdf/text.rs`:
  - Implement an RAII / scoped output suppressor for Unix that temporarily redirects standard output and standard error file descriptors (`STDOUT_FILENO`, `STDERR_FILENO`) to `/dev/null` while running `pdf_extract::extract_text`.
  - Follow `.agent/UNSAFE.md` invariants and explicit safety comments for the `libc::dup` and `libc::dup2` file descriptor operations.
- In `crates/papyrus-tui`:
  - Add `needs_clear: bool` flag to `App`. When set, the main draw loop in `crates/papyrus-tui/src/event.rs` invokes `terminal.clear()` before redrawing, resetting any cursor drift or dirty terminal states.

### 2.2 Creating Collections in TUI
- **New Actions** in `papyrus_core::Action`:
  - `CreateCollectionModalOpen`
  - `CreateCollectionModalInput(char)`
  - `CreateCollectionModalBackspace`
  - `CreateCollectionModalConfirm`
  - `CreateCollectionModalCancel`
- **Keybindings**:
  - In `ActivePanel::Collections`: `a` triggers `CreateCollectionModalOpen`.
  - Inside Modal:
    - Typing appends to `collection_name_buffer`.
    - `Backspace` deletes characters.
    - `Enter` confirms (`CreateCollectionModalConfirm`).
    - `Esc` cancels (`CreateCollectionModalCancel`).
- **Dispatch**:
  - Validate collection name (`trim()`). If empty, show status `Collection name cannot be empty`.
  - Call `CollectionRepo::insert(&conn, &NewCollection { name: trimmed.to_string(), parent_id: None })`.
  - Handle duplicate name gracefully (status: `Collection with this name already exists`).
  - Reload application state via `self.reload_from_db()`.
  - Set `selected_collection` to point to the newly created collection and sync papers.
  - Status: `Created collection '<name>'`.

### 2.3 Recursive Folder PDF Import (Unified 'a' Modal)
- In `ActivePanel::Papers`:
  - `a` opens the import modal (titled `Add Paper or Folder to Library`).
  - Input field placeholder: `/path/to/paper.pdf or /path/to/folder`.
- In `confirm_add_paper`:
  - Expand path (`~` support).
  - Check `expanded_path.is_dir()`:
    - If a single file: continue existing single-file import pipeline with output suppression during indexing.
    - If a directory:
      - Recursively walk the directory tree for files ending in `.pdf` (case-insensitive).
      - If no PDFs found: report `No PDF files found in '<dir>'`.
      - For each file found:
        - Target collection is current collection (or `None` if "All Papers").
        - Call `import_paper(conn, &self.config, &file_path, target_collection_id)`.
        - On success: index paper in Tantivy full-text index with text extraction output silenced; increment `imported`.
        - On `ImportError::Duplicate`: increment `duplicates`.
        - On error: increment `errors`.
      - Call `self.reload_from_db()`.
      - Set `self.needs_clear = true`.
      - Report status summary: `Imported <N> papers from '<folder>' (<D> duplicates skipped)`.

## 3. Testing & Verification Plan
- Unit and integration tests:
  - Test collection creation action flow and DB persistence in `crates/papyrus-tui/src/app/tests.rs`.
  - Test recursive directory scanning and bulk import in `crates/papyrus-tui/src/app/tests.rs`.
  - Test stdout suppression during text extraction in `crates/papyrus-core`.
  - Run `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features`.
