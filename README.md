# Papayrust

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Terminal-first library manager for books, scientific papers, and PDF documents. Think Zotero, but in the terminal, with Vim keybindings.

Built on a 3-crate workspace: `papyrus-core` (SQLite + Tantivy + PDF), `papyrus-tui` (Ratatui interface), `papyrus-cli` (automation commands).

Ex Omniscope, my first vibecode project

---

## Features

**Three-panel TUI**

- `Collections` — hierarchical folder tree with unlimited nesting, plus virtual collections (`All Papers`, `Recently Added`, `Unfiled`, `Untagged`).
- `Papers` — list of documents with authors, year, tags; sortable by date added, year, title, or author.
- `Details / TOC` — metadata card, abstract, interactive table of contents, and search result snippets.

**Vim navigation**

- `j`/`k`, `gg`/`G`, `Ctrl-d`/`Ctrl-u`, count prefixes (`5j`, `10k`).
- Panel switching via `h`/`l` or `Tab`/`BackTab`.

**Full-text search (Tantivy)**

- Inverted index over title, authors, abstract, tags, and full PDF text.
- Query syntax: `author:vaswani`, `year:2017`, `tag:transformer`.
- Snippet highlighting in the details panel.
- Fuzzy picker popups via `nucleo`.

**Table of contents editing**

- View TOC in the sidebar or full-screen (`t`).
- Add (`a`/`A`), rename (`e`), delete (`d`), reorder (`K`/`J`), change indent level (`H`/`L`).
- Write the modified TOC back into the PDF structure as `.annotated.pdf` (`Shift+E`).
- Open the PDF viewer at the selected TOC entry's page.

**Library organization**

- Many-to-many: one document can belong to multiple collections without duplicating the file.
- Collection picker (`c`) and tag editor (`Shift+T`) as modal overlays.

**Import and export**

- Add a single PDF or recursively scan a directory.
- SHA-256 deduplication.
- Metadata import from JSON.
- Tab-completion for paths, including `~/`.
- Export a collection or the entire library to a portable ZIP with a hot SQLite copy and `manifest.json`.

**Built-in diagnostics (`doctor`)**

- SQLite integrity check (`PRAGMA integrity_check`).
- Verifies availability of original and annotated PDFs.
- Checks Tantivy index consistency.

---

## Requirements

- Rust 1.80+ (stable)
- An external PDF viewer: `zathura`, `evince`, `okular`, or `xdg-open` (Linux) / `open` (macOS)

---

## Build

```bash
git clone https://github.com/hackerman111/papayrust.git
cd papayrust
cargo build --release
```

Binary: `./target/release/papyrus`

---

## Usage

```bash
# Launch the TUI
./target/release/papyrus

# Or via cargo
cargo run -p papayrust --
```

On first launch, Papayrust creates `.library/` with `library.db` and Tantivy search indices.

### CLI commands

```bash
papyrus status
papyrus add /path/to/paper.pdf
papyrus add /path/to/paper.pdf --collection "Machine Learning"
papyrus toc export <PAPER_UUID> --to toc.json
papyrus toc embed <PAPER_UUID>
papyrus export -o my_library_backup.zip
papyrus doctor --full
```

---

## Keybindings

### Global

| Key               | Action                                              |
| ----------------- | --------------------------------------------------- |
| `Tab` / `BackTab` | Cycle panels (`Collections` → `Papers` → `Details`) |
| `h` / `l`         | Move to adjacent panel                              |
| `/`               | Open full-text search                               |
| `?`               | Show help                                           |
| `q` / `Ctrl+C`    | Quit                                                |

### Collections panel

| Key       | Action                             |
| --------- | ---------------------------------- |
| `j` / `k` | Navigate                           |
| `Enter`   | Select collection                  |
| `a`       | New root collection                |
| `A`       | New sub-collection inside selected |
| `r`       | Rename                             |
| `E`       | Export to ZIP                      |
| `d`       | Delete (with confirmation)         |

### Papers panel

| Key           | Action                                                   |
| ------------- | -------------------------------------------------------- |
| `j` / `k`     | Navigate                                                 |
| `Enter` / `o` | Open PDF in external viewer                              |
| `c`           | Manage collections for this paper                        |
| `S`           | Cycle sort order (`Added` → `Year` → `Title` → `Author`) |
| `t`           | Full-screen TOC view                                     |
| `T`           | Edit tags                                                |
| `a`           | Add PDF or directory                                     |
| `e`           | Edit metadata (title, authors, year, DOI, abstract)      |
| `m`           | Import metadata from JSON                                |
| `d`           | Remove from current collection                           |
| `D`           | Permanently delete from library                          |

### Details / TOC panel

| Key       | Action                                       |
| --------- | -------------------------------------------- |
| `j` / `k` | Navigate TOC entries                         |
| `Enter`   | Open PDF at selected page                    |
| `t`       | Full-screen TOC                              |
| `a` / `A` | Add sibling / child entry                    |
| `e`       | Edit title or page number                    |
| `H` / `L` | Outdent / indent                             |
| `K` / `J` | Move entry up / down                         |
| `E`       | Embed TOC into PDF (writes `.annotated.pdf`) |
| `d`       | Delete entry                                 |

---

## Configuration (`papyrus.toml`)

Searched in the current directory or `~/.config/papyrus/config.toml`:

```toml
library_path = ".library"
database_path = ".library/library.db"

[viewer]
preferred_apps = ["zathura", "evince", "okular", "xdg-open"]
prefer_annotated = true

[search]
index_directory = ".library/search_index"
max_results = 50

[export]
directory = "exports"
include_database = true

[toc]
auto_extract_on_import = true
```

---

## Project layout

```
papayrust/
├── crates/
│   ├── papyrus-core/   # SQLite, Tantivy, PDF/TOC (lopdf, pdf-extract), import/export, doctor
│   ├── papyrus-tui/    # Ratatui + Crossterm, Vim navigation, pickers, modals, TOC tree
│   └── papyrus-cli/    # `papyrus` binary (CLI commands + TUI entrypoint)
├── tests/              # Integration tests
└── Cargo.toml          # Workspace manifest
```

---

## Roadmap

- [x] Phase 1 — Core: SQLite + Tantivy + three-panel TUI + TOC editing + ZIP export + doctor
- [ ] Phase 2 — UX: Quick Open (`Ctrl-p`), Markdown notes with backlinks
- [ ] Phase 3 — Science: DOI/arXiv/ISBN lookup, metadata enrichment (CrossRef, Semantic Scholar, OpenAlex), reference extraction, BibTeX/RIS/CSL export
- [ ] Phase 4 — Integrations: local AI assistant, Zotero/Calibre sync, server-side library sync

---

## Checks

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

---

## License

MIT
