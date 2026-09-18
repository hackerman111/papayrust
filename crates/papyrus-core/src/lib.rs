//! Papyrus Core: domain logic, storage, search, and PDF processing.

pub mod action;
pub mod config;
pub mod db;
pub mod doctor;
pub mod export;
pub mod importer;
pub mod metadata_editor;
pub mod opener;
pub mod pdf;
pub mod search;
pub mod time;
pub mod toc;

pub use action::{Action, TocImportSource};
pub use doctor::{check_database, check_files, run_doctor, DoctorReport, DoctorVerdict};
pub use export::{
    export_collection, export_library, ExportError, ExportOptions, ExportResult, Manifest,
    ManifestCollection, ManifestPaper,
};
pub use importer::{import_paper, ImportError, Importer};
pub use metadata_editor::{update_metadata, MetadataError, UpdatePaperMetadata};
pub use opener::{
    build_open_command, open_paper, resolve_paper_path, CommandRunner, ExecutedCommand,
    MockCommandRunner, OpenerError, ProcessCommandRunner,
};
pub use search::{SearchError, SearchIndex, SearchResult};
pub use toc::{
    add_entry, delete_entry, edit_entry, embed_toc_in_pdf, export_to_json, from_pdf_outline,
    import_toc, indent, is_annotated_copy_outdated, move_down, move_up, outdent, parse_json,
    parse_text, PendingTocEntry, TocError, TocImportError,
};
