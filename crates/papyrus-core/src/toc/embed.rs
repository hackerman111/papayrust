use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use lopdf::{Dictionary, Document, Object, ObjectId};
use rusqlite::Connection;
#[cfg(debug_assertions)]
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::Config;
use crate::db::models::{Paper, TocEntry};
use crate::db::paper_repo::PaperRepo;
use crate::db::toc_repo::TocRepo;
use crate::time::current_timestamp_utc;
use crate::toc::model::TocError;

/// Checks if a paper's annotated PDF copy is outdated relative to its TOC entries.
pub fn is_annotated_copy_outdated(paper: &Paper) -> bool {
    let Some(ref path_str) = paper.annotated_pdf_path else {
        return true;
    };
    if !Path::new(path_str).exists() {
        return true;
    }
    let Some(ref embedded_at) = paper.toc_embedded_at else {
        return true;
    };
    // If updated_at is newer than toc_embedded_at, the annotated copy is outdated
    paper.updated_at.as_str() > embedded_at.as_str()
}

/// Materializes the table of contents into an annotated PDF copy.
/// Strict invariant: The original PDF file at `paper.file_path` is never modified.
pub fn embed_toc_in_pdf(
    conn: &mut Connection,
    config: &Config,
    paper_id: Uuid,
) -> Result<PathBuf, TocError> {
    let paper = PaperRepo::get_by_id(conn, paper_id)?.ok_or(TocError::PaperNotFound(paper_id))?;

    let orig_path = Path::new(&paper.file_path);
    if !orig_path.exists() {
        return Err(TocError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Original PDF not found at {}", orig_path.display()),
        )));
    }

    // Capture original file hash to verify immutability invariant
    let orig_bytes = fs::read(orig_path)?;
    #[cfg(debug_assertions)]
    let orig_hash_before = format!("{:x}", Sha256::digest(&orig_bytes));

    let toc_entries = TocRepo::get_by_paper(conn, paper_id)?;

    // Load PDF document
    let mut doc = Document::load_mem(&orig_bytes)
        .map_err(|e| TocError::Pdf(format!("Failed to load PDF: {}", e)))?;

    // Build page lookup: 1-based page number -> Page ObjectId
    let page_map: HashMap<u32, ObjectId> = doc.get_pages().into_iter().collect();

    // Group entries by parent_id
    let mut children_map: HashMap<Option<Uuid>, Vec<&TocEntry>> = HashMap::new();
    for entry in &toc_entries {
        children_map.entry(entry.parent_id).or_default().push(entry);
    }
    for children in children_map.values_mut() {
        children.sort_by_key(|e| e.order_index);
    }

    // Allocate an ObjectId for each TocEntry
    let mut entry_obj_ids: HashMap<Uuid, ObjectId> = HashMap::new();
    for entry in &toc_entries {
        let obj_id = doc.new_object_id();
        entry_obj_ids.insert(entry.id, obj_id);
    }

    // Allocate outline root dictionary ObjectId
    let outlines_root_id = doc.new_object_id();

    // Build outline items
    for entry in &toc_entries {
        let obj_id = match entry_obj_ids.get(&entry.id) {
            Some(&id) => id,
            None => continue,
        };
        let parent_obj_id = match entry.parent_id {
            Some(pid) => entry_obj_ids.get(&pid).copied().unwrap_or(outlines_root_id),
            None => outlines_root_id,
        };

        let mut item_dict = Dictionary::new();
        item_dict.set("Title", Object::string_literal(entry.title.as_bytes()));
        item_dict.set("Parent", Object::Reference(parent_obj_id));

        // Set destination page
        if let Some(&page_obj_id) = page_map.get(&entry.page_number) {
            item_dict.set(
                "Dest",
                Object::Array(vec![
                    Object::Reference(page_obj_id),
                    Object::Name(b"Fit".to_vec()),
                ]),
            );
        } else if let Some(&first_page_obj_id) = page_map.get(&1) {
            item_dict.set(
                "Dest",
                Object::Array(vec![
                    Object::Reference(first_page_obj_id),
                    Object::Name(b"Fit".to_vec()),
                ]),
            );
        }

        // Prev and Next among siblings
        if let Some(siblings) = children_map.get(&entry.parent_id) {
            if let Some(idx) = siblings.iter().position(|e| e.id == entry.id) {
                if idx > 0 {
                    if let Some(prev) = siblings.get(idx - 1) {
                        if let Some(&prev_id) = entry_obj_ids.get(&prev.id) {
                            item_dict.set("Prev", Object::Reference(prev_id));
                        }
                    }
                }
                if idx + 1 < siblings.len() {
                    if let Some(next) = siblings.get(idx + 1) {
                        if let Some(&next_id) = entry_obj_ids.get(&next.id) {
                            item_dict.set("Next", Object::Reference(next_id));
                        }
                    }
                }
            }
        }

        // First and Last among children
        if let Some(children) = children_map.get(&Some(entry.id)) {
            if !children.is_empty() {
                if let (Some(first_child), Some(last_child)) = (children.first(), children.last()) {
                    if let (Some(&first_child_id), Some(&last_child_id)) = (
                        entry_obj_ids.get(&first_child.id),
                        entry_obj_ids.get(&last_child.id),
                    ) {
                        item_dict.set("First", Object::Reference(first_child_id));
                        item_dict.set("Last", Object::Reference(last_child_id));
                        item_dict.set("Count", Object::Integer(children.len() as i64));
                    }
                }
            }
        }

        doc.objects.insert(obj_id, Object::Dictionary(item_dict));
    }

    // Build Outlines root dictionary
    let mut root_dict = Dictionary::new();
    root_dict.set("Type", Object::Name(b"Outlines".to_vec()));
    if let Some(top_entries) = children_map.get(&None) {
        if !top_entries.is_empty() {
            if let (Some(first_entry), Some(last_entry)) = (top_entries.first(), top_entries.last())
            {
                if let (Some(&first_id), Some(&last_id)) = (
                    entry_obj_ids.get(&first_entry.id),
                    entry_obj_ids.get(&last_entry.id),
                ) {
                    root_dict.set("First", Object::Reference(first_id));
                    root_dict.set("Last", Object::Reference(last_id));
                    root_dict.set("Count", Object::Integer(top_entries.len() as i64));
                }
            }
        }
    }
    doc.objects
        .insert(outlines_root_id, Object::Dictionary(root_dict));

    // Link Outlines root into Catalog (trailer -> Root)
    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(|obj| obj.as_reference())
        .map_err(|e| TocError::Pdf(format!("Catalog root not found: {}", e)))?;

    let catalog_dict = doc
        .get_object_mut(catalog_id)
        .map_err(|e| TocError::Pdf(format!("Catalog object not found: {}", e)))?
        .as_dict_mut()
        .map_err(|e| TocError::Pdf(format!("Catalog object is not a dictionary: {}", e)))?;
    catalog_dict.set("Outlines", Object::Reference(outlines_root_id));

    // Determine target annotated directory
    let annotated_dir = if config.toc.annotated_dir.is_absolute() {
        config.toc.annotated_dir.clone()
    } else {
        config.library_path.join(&config.toc.annotated_dir)
    };
    fs::create_dir_all(&annotated_dir)?;

    let stem = orig_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("paper");
    let target_filename = format!("{}.annotated.pdf", stem);
    let target_path = annotated_dir.join(target_filename);

    // Atomic write via tempfile in same directory
    let temp_path = annotated_dir.join(format!("{}.tmp.{}", stem, Uuid::now_v7()));
    doc.save(&temp_path)
        .map_err(|e| TocError::Pdf(format!("Failed to save annotated PDF: {}", e)))?;

    fs::rename(&temp_path, &target_path)?;

    // VERIFY INVARIANT: Original file must be completely untouched!
    #[cfg(debug_assertions)]
    {
        let orig_bytes_after = fs::read(orig_path)?;
        let orig_hash_after = format!("{:x}", Sha256::digest(&orig_bytes_after));
        debug_assert_eq!(
            orig_hash_before, orig_hash_after,
            "CRITICAL INVARIANT VIOLATION: Original PDF file was modified during embed!"
        );
    }

    // Update paper record in SQLite
    let now = current_timestamp_utc();
    let mut updated_paper = paper;
    updated_paper.annotated_pdf_path = Some(target_path.to_string_lossy().to_string());
    updated_paper.toc_embedded_at = Some(now.clone());
    updated_paper.updated_at = now;
    PaperRepo::update(conn, &updated_paper)?;

    Ok(target_path)
}
