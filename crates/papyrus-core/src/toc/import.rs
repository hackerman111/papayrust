use std::collections::HashMap;
use std::path::Path;

use lopdf::Document;
use rusqlite::Connection;
use uuid::Uuid;

use crate::action::TocImportSource;
use crate::db::models::{TocEntry, TocSource};
use crate::db::paper_repo::PaperRepo;
use crate::pdf::outline::extract_outlines;
use crate::time::current_timestamp_utc;
use crate::toc::model::{PendingTocEntry, TocError, TocImportError};

/// Parses a tab-indented (or 4-space indented) text table of contents.
/// Format: `[tabs]Title\tPage`
pub fn parse_text(content: &str) -> Result<Vec<PendingTocEntry>, TocImportError> {
    let mut entries = Vec::new();
    let mut prev_level: Option<usize> = None;
    let mut indent_type: Option<bool> = None; // true = tabs, false = spaces

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let line_trimmed = line.trim_end();
        if line_trimmed.trim().is_empty() {
            continue; // Skip empty/whitespace lines
        }

        // Detect indentation
        let mut tabs = 0;
        let mut spaces = 0;

        for c in line.chars() {
            if c == '\t' {
                tabs += 1;
            } else if c == ' ' {
                spaces += 1;
            } else {
                break;
            }
        }

        if tabs > 0 && spaces > 0 {
            return Err(TocImportError::Line {
                line_no,
                reason: "Mixed tabs and spaces in indentation is prohibited".to_string(),
            });
        }

        let level = if tabs > 0 {
            if let Some(false) = indent_type {
                return Err(TocImportError::Line {
                    line_no,
                    reason: "Mixed tabs and spaces in file indentation".to_string(),
                });
            }
            indent_type = Some(true);
            tabs
        } else if spaces > 0 {
            if spaces % 4 != 0 {
                return Err(TocImportError::Line {
                    line_no,
                    reason: format!(
                        "Spaces indentation must be multiples of 4, found {}",
                        spaces
                    ),
                });
            }
            if let Some(true) = indent_type {
                return Err(TocImportError::Line {
                    line_no,
                    reason: "Mixed tabs and spaces in file indentation".to_string(),
                });
            }
            indent_type = Some(false);
            spaces / 4
        } else {
            0
        };

        // Check indentation level leap
        if let Some(prev) = prev_level {
            if level > prev + 1 {
                return Err(TocImportError::Line {
                    line_no,
                    reason: format!(
                        "Indentation level jump too large (from level {} to {})",
                        prev, level
                    ),
                });
            }
        } else if level > 0 {
            return Err(TocImportError::Line {
                line_no,
                reason: format!("First TOC entry must be at level 0, found level {}", level),
            });
        }
        prev_level = Some(level);

        // Extract title and page number from remainder of line
        let remainder = line_trimmed.trim_start();
        // The page number is separated by the last tab or space
        let (title_part, page_part) = if let Some(last_tab_idx) = remainder.rfind('\t') {
            (&remainder[..last_tab_idx], &remainder[last_tab_idx + 1..])
        } else if let Some(last_space_idx) = remainder.rfind(' ') {
            (
                &remainder[..last_space_idx],
                &remainder[last_space_idx + 1..],
            )
        } else {
            return Err(TocImportError::Line {
                line_no,
                reason: "Missing page number".to_string(),
            });
        };

        let title = title_part.trim();
        if title.is_empty() {
            return Err(TocImportError::Line {
                line_no,
                reason: "Title cannot be empty".to_string(),
            });
        }

        let page: u32 = page_part.trim().parse().map_err(|_| TocImportError::Line {
            line_no,
            reason: format!("Invalid page number: '{}'", page_part.trim()),
        })?;

        if page == 0 {
            return Err(TocImportError::Line {
                line_no,
                reason: "Page number must be at least 1".to_string(),
            });
        }

        entries.push(PendingTocEntry {
            title: title.to_string(),
            page,
            level,
        });
    }

    if entries.is_empty() {
        return Err(TocImportError::EmptyOutline);
    }

    Ok(entries)
}

/// Parses a JSON array of `PendingTocEntry` objects.
pub fn parse_json(content: &str) -> Result<Vec<PendingTocEntry>, TocImportError> {
    let entries: Vec<PendingTocEntry> =
        serde_json::from_str(content).map_err(|e| TocImportError::Json(e.to_string()))?;

    if entries.is_empty() {
        return Err(TocImportError::EmptyOutline);
    }

    let mut prev_level: Option<usize> = None;
    for (idx, entry) in entries.iter().enumerate() {
        let line_no = idx + 1;
        if entry.title.trim().is_empty() {
            return Err(TocImportError::Line {
                line_no,
                reason: "Title cannot be empty".to_string(),
            });
        }
        if entry.page == 0 {
            return Err(TocImportError::Line {
                line_no,
                reason: "Page number must be at least 1".to_string(),
            });
        }

        if let Some(prev) = prev_level {
            if entry.level > prev + 1 {
                return Err(TocImportError::Line {
                    line_no,
                    reason: format!(
                        "Indentation level jump too large (from level {} to {})",
                        prev, entry.level
                    ),
                });
            }
        } else if entry.level > 0 {
            return Err(TocImportError::Line {
                line_no,
                reason: format!(
                    "First TOC entry must be at level 0, found level {}",
                    entry.level
                ),
            });
        }
        prev_level = Some(entry.level);
    }

    Ok(entries)
}

/// Extracts outline from a PDF file using lopdf and converts it into `Vec<PendingTocEntry>`.
pub fn from_pdf_outline(pdf_path: &Path) -> Result<Vec<PendingTocEntry>, TocImportError> {
    let doc = Document::load(pdf_path)
        .map_err(|e| TocImportError::Pdf(format!("{}: {}", pdf_path.display(), e)))?;
    let dummy_id = Uuid::now_v7();
    let entries =
        extract_outlines(&doc, dummy_id).map_err(|e| TocImportError::Pdf(e.to_string()))?;

    if entries.is_empty() {
        return Err(TocImportError::EmptyOutline);
    }

    // Build parent-to-level mapping
    let mut id_to_level = HashMap::new();
    let mut pending = Vec::with_capacity(entries.len());

    for entry in entries {
        let level = match entry.parent_id {
            Some(pid) => id_to_level.get(&pid).copied().unwrap_or(0) + 1,
            None => 0,
        };
        id_to_level.insert(entry.id, level);

        pending.push(PendingTocEntry {
            title: entry.title,
            page: entry.page_number,
            level,
        });
    }

    Ok(pending)
}

/// Exports existing `TocEntry` items for a paper into symmetric JSON representation.
pub fn export_to_json(entries: &[TocEntry]) -> Result<String, TocImportError> {
    if entries.is_empty() {
        return Ok("[]".to_string());
    }

    let id_to_parent: HashMap<Uuid, Option<Uuid>> =
        entries.iter().map(|e| (e.id, e.parent_id)).collect();

    let mut id_to_level: HashMap<Uuid, usize> = HashMap::new();
    for entry in entries {
        let mut level = 0;
        let mut curr = entry.parent_id;
        while let Some(pid) = curr {
            level += 1;
            curr = id_to_parent.get(&pid).copied().flatten();
            if level > 128 {
                break;
            }
        }
        id_to_level.insert(entry.id, level);
    }

    let mut pending = Vec::with_capacity(entries.len());
    for entry in entries {
        let level = id_to_level.get(&entry.id).copied().unwrap_or(0);
        pending.push(PendingTocEntry {
            title: entry.title.clone(),
            page: entry.page_number,
            level,
        });
    }

    serde_json::to_string_pretty(&pending).map_err(|e| TocImportError::Json(e.to_string()))
}

/// Imports a table of contents into a paper from `TocImportSource`.
/// All new entries receive `TocSource::Imported`.
/// Validates that all page numbers are `<= page_count` from the target PDF.
pub fn import_toc(
    conn: &mut Connection,
    paper_id: Uuid,
    pdf_path: &Path,
    source: &TocImportSource,
    merge: bool,
) -> Result<Vec<TocEntry>, TocError> {
    // 1. Parse source into PendingTocEntry items
    let pending_entries = match source {
        TocImportSource::TextFile(path) => {
            let content = std::fs::read_to_string(path)
                .map_err(|e| TocImportError::Io(format!("{}: {}", path.display(), e)))?;
            parse_text(&content)?
        }
        TocImportSource::JsonFile(path) => {
            let content = std::fs::read_to_string(path)
                .map_err(|e| TocImportError::Io(format!("{}: {}", path.display(), e)))?;
            parse_json(&content)?
        }
        TocImportSource::PdfOutline => from_pdf_outline(pdf_path)?,
    };

    // 2. Obtain total page count from lopdf
    let doc = Document::load(pdf_path)
        .map_err(|e| TocError::Pdf(format!("{}: {}", pdf_path.display(), e)))?;
    let page_count = doc.get_pages().len() as u32;

    // 3. Validate pages
    for entry in &pending_entries {
        if entry.page > page_count {
            return Err(TocError::Import(TocImportError::PageOutOfBounds {
                page: entry.page,
                page_count,
            }));
        }
    }

    // 4. Perform atomic database transaction
    let now = current_timestamp_utc();
    let mut paper =
        PaperRepo::get_by_id(conn, paper_id)?.ok_or(TocError::PaperNotFound(paper_id))?;

    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

    if !merge {
        // Replace mode: delete all existing entries for this paper
        tx.execute(
            "DELETE FROM toc_entries WHERE paper_id = ?1",
            rusqlite::params![paper_id.to_string()],
        )?;
    }

    // In merge mode, find the starting order_index for root entries
    let mut base_root_order: i32 = if merge {
        let max_order: Option<i32> = tx.query_row(
            "SELECT MAX(order_index) FROM toc_entries WHERE paper_id = ?1 AND parent_id IS NULL",
            rusqlite::params![paper_id.to_string()],
            |row| row.get(0),
        )?;
        max_order.map(|m| m + 1).unwrap_or(0)
    } else {
        0
    };

    let mut level_parents: Vec<(usize, Uuid)> = Vec::new();
    let mut level_orders: HashMap<Option<Uuid>, i32> = HashMap::new();
    let mut inserted_entries = Vec::with_capacity(pending_entries.len());

    for pending in pending_entries {
        // Adjust parent stack according to current level
        while let Some(&(stack_level, _)) = level_parents.last() {
            if stack_level >= pending.level {
                level_parents.pop();
            } else {
                break;
            }
        }

        let parent_id = level_parents.last().map(|&(_, id)| id);
        let order_index = if parent_id.is_none() && merge {
            let o = base_root_order;
            base_root_order += 1;
            o
        } else {
            let entry_counter = level_orders.entry(parent_id).or_insert(0);
            let o = *entry_counter;
            *entry_counter += 1;
            o
        };

        let new_id = Uuid::now_v7();
        let entry = TocEntry {
            id: new_id,
            paper_id,
            parent_id,
            title: pending.title,
            page_number: pending.page,
            order_index,
            source: TocSource::Imported,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        tx.execute(
            "INSERT INTO toc_entries (id, paper_id, parent_id, title, page_number, order_index, source, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                entry.id.to_string(),
                entry.paper_id.to_string(),
                entry.parent_id.map(|u| u.to_string()),
                entry.title,
                entry.page_number,
                entry.order_index,
                entry.source.as_str(),
                entry.created_at,
                entry.updated_at,
            ],
        )?;

        level_parents.push((pending.level, new_id));
        inserted_entries.push(entry);
    }

    // Invalidate annotated copy by updating paper's updated_at
    paper.updated_at = now;
    tx.execute(
        "UPDATE papers SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![paper.updated_at, paper.id.to_string()],
    )?;

    tx.commit()?;

    Ok(inserted_entries)
}
