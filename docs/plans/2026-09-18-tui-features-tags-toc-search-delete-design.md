# Design: TUI Usability Overhaul, Fullscreen TOC, Tags, Unified Search, and Deletion

**Date:** 2026-09-18  
**Status:** Approved

## 1. Scope and Goals
1. **Default Focus**: Launch TUI focused on `ActivePanel::Papers` instead of `ActivePanel::Collections`.
2. **Dedicated Fullscreen Table of Contents (TOC) View (`t`)**:
   - Dedicated full-screen viewer displaying paper header, structured TOC hierarchy, and page numbers.
   - Smooth viewport scrolling with dynamic offset calculation (fixing truncation / cursor overflow).
   - Direct jump to page via `Enter` (`opener::open_paper(paper, Some(page), ...)`).
   - Fix side-panel TOC scrolling as well.
3. **Tags System**:
   - SQLite tables: `tags(id, name)` and `paper_tags(paper_id, tag_id)`.
   - `TagRepo` in `papyrus-core`: get/set/list tags.
   - Hotkey `T` (Shift+t) in `Papers` panel opens modal dialog to edit comma-separated tags.
   - Tags displayed in paper list and details view.
4. **Search Overhaul**:
   - Filter by `title` and by `tags` (including `#tag` or `tag:name` syntax, or general substring matching).
   - Collection cycling in search mode via `Tab` key in search input.
5. **Collection and Paper Deletion (`d`)**:
   - `ActivePanel::Collections`: `d` prompts confirmation, calls `CollectionRepo::delete(conn, id)`, reloads state.
   - `ActivePanel::Papers`: `d` prompts confirmation, calls `PaperRepo::delete(conn, id)`, removes paper from Tantivy index, reloads state.
6. **Clean Status Bar & Help Modal (`?`)**:
   - Context-aware status bar showing only relevant keys for the active panel/mode.
   - `?` opens a Help overlay modal explaining navigation and shortcuts.

## 2. Architecture & Modules
- `papyrus-core`:
  - Database schema: ensure `tags` and `paper_tags` tables exist.
  - `TagRepo` module in `crates/papyrus-core/src/db/tag_repo.rs`.
  - Actions in `crates/papyrus-core/src/action.rs`:
    - `OpenFullscreenToc`, `CloseFullscreenToc`
    - `EditTagsModalOpen`, `EditTagsModalInput(char)`, `EditTagsModalBackspace`, `EditTagsModalConfirm`, `EditTagsModalCancel`
    - `DeleteModalOpen`, `DeleteModalConfirm`, `DeleteModalCancel`
    - `SearchCycleCollection`
    - `HelpModalToggle`
- `papyrus-tui`:
  - `App` state updates: `is_viewing_fullscreen_toc`, `toc_scroll_offset`, `is_editing_tags`, `tags_input_buffer`, `is_confirming_delete`, `delete_target_description`, `is_showing_help`, `search_collection_index`.
  - Event loop mapping in `event.rs`.
  - Action dispatching in `dispatch.rs`.
  - UI rendering in `panels.rs`, `modals.rs`, `search_bar.rs`, `status_bar.rs`.

## 3. Verification
- Unit & integration tests for tag repository, deletion actions, search filtering by tag/title, fullscreen TOC scrolling, and keybindings.
- All workspace checks passing: `cargo test --workspace`, `cargo clippy`, `cargo fmt`.
- Build release binary `./papyrus`.
