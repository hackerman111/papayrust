pub mod archive;
pub mod backup;
pub mod error;
pub mod manifest;
pub mod options;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;

use rusqlite::Connection;
use uuid::Uuid;

use crate::config::Config;
use crate::db::collection_repo::CollectionRepo;
use crate::db::models::Paper;
use crate::db::paper_repo::PaperRepo;
use crate::db::toc_repo::TocRepo;
use crate::time::current_timestamp_utc;

pub use archive::{
    is_annotated_fresh, resolve_collision, stream_file_to_zip, validate_archive_entry_path,
    write_bytes_to_zip,
};
pub use backup::create_hot_backup;
pub use error::ExportError;
pub use manifest::{
    build_manifest_paper, compute_toc_hierarchy, Manifest, ManifestCollection, ManifestPaper,
};
use options::TempFileGuard;
pub use options::{ExportOptions, ExportResult};

/// Exports the entire library (all collections and papers) to a ZIP archive.
pub fn export_library(
    conn: &Connection,
    config: &Config,
    options: &ExportOptions,
) -> Result<ExportResult, ExportError> {
    let collections = CollectionRepo::list(conn)?;
    let manifest_cols = collections.iter().map(ManifestCollection::from).collect();
    let papers = PaperRepo::list(conn)?;

    let timestamp = current_timestamp_utc().replace(':', "-");
    let default_name = format!("papyrus_library_{timestamp}.zip");

    export_internal(conn, config, options, manifest_cols, papers, &default_name)
}

/// Exports a single collection and all of its associated papers to a ZIP archive.
pub fn export_collection(
    conn: &Connection,
    config: &Config,
    collection_id: Uuid,
    options: &ExportOptions,
) -> Result<ExportResult, ExportError> {
    let col = CollectionRepo::get_by_id(conn, collection_id)?
        .ok_or_else(|| ExportError::CollectionNotFound(collection_id.to_string()))?;

    let papers = CollectionRepo::get_papers(conn, collection_id)?;
    let manifest_cols = vec![ManifestCollection::from(&col)];

    let timestamp = current_timestamp_utc().replace(':', "-");
    let col_clean = col.name.replace(['/', '\\', ' '], "_");
    let default_name = format!("papyrus_collection_{col_clean}_{timestamp}.zip");

    export_internal(conn, config, options, manifest_cols, papers, &default_name)
}

fn export_internal(
    conn: &Connection,
    config: &Config,
    options: &ExportOptions,
    manifest_collections: Vec<ManifestCollection>,
    papers: Vec<Paper>,
    default_name: &str,
) -> Result<ExportResult, ExportError> {
    let target_path = if options.output_path.as_os_str().is_empty() {
        config.export.directory.join(default_name)
    } else {
        options.output_path.clone()
    };

    if target_path.exists() && !options.overwrite {
        return Err(ExportError::AlreadyExists(target_path));
    }

    let target_dir = target_path.parent().unwrap_or_else(|| Path::new("."));
    if !target_dir.as_os_str().is_empty() {
        std::fs::create_dir_all(target_dir)?;
    }

    let temp_zip_path = target_dir.join(format!(
        ".tmp_export_{}_{}.zip",
        std::process::id(),
        Uuid::now_v7()
    ));
    let _zip_guard = TempFileGuard(&temp_zip_path);

    let zip_file = File::create(&temp_zip_path)?;
    let mut zip = zip::ZipWriter::new(zip_file);

    let mut total_bytes = 0u64;
    let mut warnings = Vec::new();
    let mut used_archive_paths = HashSet::new();
    let mut archived_hashes: HashMap<String, String> = HashMap::new();
    let mut manifest_papers = Vec::with_capacity(papers.len());

    // 1. Hot backup SQLite DB if requested
    if options.include_database {
        let temp_db_path = target_dir.join(format!(
            ".tmp_backup_{}_{}.db",
            std::process::id(),
            Uuid::now_v7()
        ));
        let _db_guard = TempFileGuard(&temp_db_path);

        create_hot_backup(conn, &temp_db_path, 100)?;
        used_archive_paths.insert(".library/library.db".to_string());
        let db_bytes = stream_file_to_zip(&temp_db_path, &mut zip, ".library/library.db")?;
        total_bytes += db_bytes;
    }

    // 2. Export papers and annotated copies
    for paper in &papers {
        let col_records = CollectionRepo::get_collections_for_paper(conn, paper.id)?;
        let collection_ids = col_records.into_iter().map(|c| c.id).collect();
        let toc_entries = TocRepo::get_by_paper(conn, paper.id)?;

        let archive_file_path = if let Some(existing) = archived_hashes.get(&paper.content_hash) {
            existing.clone()
        } else {
            let orig_path = Path::new(&paper.file_path);
            if !orig_path.exists() {
                if options.skip_missing_files {
                    warnings.push(format!(
                        "File not found for paper {}: {}",
                        paper.id, paper.file_path
                    ));
                    String::new()
                } else {
                    return Err(ExportError::PaperFileNotFound {
                        id: paper.id,
                        path: paper.file_path.clone(),
                    });
                }
            } else {
                let raw_name = orig_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("paper.pdf");
                let desired = format!("papers/{raw_name}");
                let resolved = resolve_collision(&desired, &mut used_archive_paths);
                let bytes = stream_file_to_zip(orig_path, &mut zip, &resolved)?;
                total_bytes += bytes;
                archived_hashes.insert(paper.content_hash.clone(), resolved.clone());
                resolved
            }
        };

        let archive_annotated_path = if is_annotated_fresh(paper) {
            let ann_path_str = paper.annotated_pdf_path.as_deref().unwrap_or("");
            let ann_orig = Path::new(ann_path_str);
            let raw_ann_name = ann_orig
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("paper.annotated.pdf");
            let desired = format!("annotated/{raw_ann_name}");
            let resolved = resolve_collision(&desired, &mut used_archive_paths);
            let bytes = stream_file_to_zip(ann_orig, &mut zip, &resolved)?;
            total_bytes += bytes;
            Some(resolved)
        } else {
            None
        };

        manifest_papers.push(build_manifest_paper(
            paper,
            archive_file_path,
            archive_annotated_path,
            collection_ids,
            &toc_entries,
        ));
    }

    // 3. Write manifest.json
    let manifest = Manifest {
        version: 1,
        exported_at: current_timestamp_utc(),
        collections: manifest_collections,
        papers: manifest_papers,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    write_bytes_to_zip(&manifest_bytes, &mut zip, "manifest.json")?;
    total_bytes += manifest_bytes.len() as u64;

    zip.finish()?;

    // 4. Atomic rename from temp file to destination
    std::fs::rename(&temp_zip_path, &target_path)?;

    Ok(ExportResult {
        archive_path: target_path,
        paper_count: papers.len(),
        collection_count: manifest.collections.len(),
        total_bytes,
        warnings,
    })
}
