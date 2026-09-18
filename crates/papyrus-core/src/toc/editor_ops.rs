use rusqlite::Connection;
use uuid::Uuid;

use crate::db::models::{TocEntry, TocSource};
use crate::db::paper_repo::PaperRepo;
use crate::db::toc_repo::TocRepo;
use crate::time::current_timestamp_utc;
use crate::toc::model::TocError;

/// Adds a new TOC entry to the paper.
pub fn add_entry(
    conn: &mut Connection,
    paper_id: Uuid,
    parent_id: Option<Uuid>,
    title: String,
    page_number: u32,
    source: TocSource,
) -> Result<TocEntry, TocError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(TocError::Validation("Title cannot be empty".to_string()));
    }
    if page_number == 0 {
        return Err(TocError::Validation(
            "Page number must be at least 1".to_string(),
        ));
    }

    let tx = conn.transaction()?;

    // Verify paper exists
    let paper = PaperRepo::get_by_id(&tx, paper_id)?.ok_or(TocError::PaperNotFound(paper_id))?;

    // Determine next order_index among siblings
    let max_order: Option<i32> = match parent_id {
        Some(pid) => tx.query_row(
            "SELECT MAX(order_index) FROM toc_entries WHERE paper_id = ?1 AND parent_id = ?2",
            rusqlite::params![paper_id.to_string(), pid.to_string()],
            |row| row.get(0),
        )?,
        None => tx.query_row(
            "SELECT MAX(order_index) FROM toc_entries WHERE paper_id = ?1 AND parent_id IS NULL",
            rusqlite::params![paper_id.to_string()],
            |row| row.get(0),
        )?,
    };
    let order_index = max_order.map(|m| m + 1).unwrap_or(0);

    let now = current_timestamp_utc();
    let entry = TocEntry {
        id: Uuid::now_v7(),
        paper_id,
        parent_id,
        title: trimmed.to_string(),
        page_number,
        order_index,
        source,
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    TocRepo::insert(&tx, &entry)?;

    // Invalidate annotated copy by updating paper's updated_at
    let mut updated_paper = paper;
    updated_paper.updated_at = now;
    PaperRepo::update(&tx, &updated_paper)?;

    tx.commit()?;

    Ok(entry)
}

/// Edits the title and/or page of an existing TOC entry, preserving its `source` and hierarchy.
pub fn edit_entry(
    conn: &mut Connection,
    entry_id: Uuid,
    new_title: Option<String>,
    new_page: Option<u32>,
) -> Result<TocEntry, TocError> {
    if let Some(ref t) = new_title {
        let trimmed = t.trim();
        if trimmed.is_empty() {
            return Err(TocError::Validation("Title cannot be empty".to_string()));
        }
    }

    if let Some(p) = new_page {
        if p == 0 {
            return Err(TocError::Validation(
                "Page number must be at least 1".to_string(),
            ));
        }
    }

    let tx = conn.transaction()?;

    let mut entry = TocRepo::get_by_id(&tx, entry_id)?.ok_or(TocError::NotFound(entry_id))?;

    if let Some(t) = new_title {
        entry.title = t.trim().to_string();
    }

    if let Some(p) = new_page {
        entry.page_number = p;
    }

    let now = current_timestamp_utc();
    entry.updated_at = now.clone();

    // Source is explicitly preserved!
    TocRepo::update(&tx, &entry)?;

    // Update paper updated_at to mark annotated copy outdated
    if let Some(mut paper) = PaperRepo::get_by_id(&tx, entry.paper_id)? {
        paper.updated_at = now;
        PaperRepo::update(&tx, &paper)?;
    }

    tx.commit()?;

    Ok(entry)
}

/// Deletes a TOC entry and cascades deletion to all children via SQLite CASCADE.
pub fn delete_entry(conn: &mut Connection, entry_id: Uuid) -> Result<(), TocError> {
    let tx = conn.transaction()?;

    let entry = TocRepo::get_by_id(&tx, entry_id)?.ok_or(TocError::NotFound(entry_id))?;

    TocRepo::delete(&tx, entry_id)?;

    // Invalidate annotated copy
    let now = current_timestamp_utc();
    if let Some(mut paper) = PaperRepo::get_by_id(&tx, entry.paper_id)? {
        paper.updated_at = now;
        PaperRepo::update(&tx, &paper)?;
    }

    tx.commit()?;

    Ok(())
}

/// Indents an entry, making it the child of its immediate preceding sibling.
pub fn indent(conn: &mut Connection, entry_id: Uuid) -> Result<TocEntry, TocError> {
    let tx = conn.transaction()?;

    let mut entry = TocRepo::get_by_id(&tx, entry_id)?.ok_or(TocError::NotFound(entry_id))?;

    // Find preceding sibling (highest order_index < entry.order_index with same paper_id and parent_id)
    let prev_sibling: Option<(String, i32)> = match entry.parent_id {
        Some(pid) => {
            let mut stmt = tx.prepare(
                "SELECT id, order_index FROM toc_entries \
                 WHERE paper_id = ?1 AND parent_id = ?2 AND order_index < ?3 \
                 ORDER BY order_index DESC LIMIT 1",
            )?;
            stmt.query_row(
                rusqlite::params![
                    entry.paper_id.to_string(),
                    pid.to_string(),
                    entry.order_index
                ],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok()
        }
        None => {
            let mut stmt = tx.prepare(
                "SELECT id, order_index FROM toc_entries \
                 WHERE paper_id = ?1 AND parent_id IS NULL AND order_index < ?2 \
                 ORDER BY order_index DESC LIMIT 1",
            )?;
            stmt.query_row(
                rusqlite::params![entry.paper_id.to_string(), entry.order_index],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok()
        }
    };

    let Some((new_parent_id_str, _)) = prev_sibling else {
        return Err(TocError::Validation(
            "Cannot indent: no preceding sibling".to_string(),
        ));
    };

    let new_parent_id =
        Uuid::parse_str(&new_parent_id_str).map_err(|e| TocError::Validation(e.to_string()))?;

    // Determine order_index in new parent
    let max_order: Option<i32> = tx.query_row(
        "SELECT MAX(order_index) FROM toc_entries WHERE paper_id = ?1 AND parent_id = ?2",
        rusqlite::params![entry.paper_id.to_string(), new_parent_id.to_string()],
        |row| row.get(0),
    )?;
    let new_order = max_order.map(|m| m + 1).unwrap_or(0);

    let now = current_timestamp_utc();
    entry.parent_id = Some(new_parent_id);
    entry.order_index = new_order;
    entry.updated_at = now.clone();

    TocRepo::update(&tx, &entry)?;

    if let Some(mut paper) = PaperRepo::get_by_id(&tx, entry.paper_id)? {
        paper.updated_at = now;
        PaperRepo::update(&tx, &paper)?;
    }

    tx.commit()?;

    Ok(entry)
}

/// Outdents an entry, moving it up to its grandparent's level while keeping its own children.
pub fn outdent(conn: &mut Connection, entry_id: Uuid) -> Result<TocEntry, TocError> {
    let tx = conn.transaction()?;

    let mut entry = TocRepo::get_by_id(&tx, entry_id)?.ok_or(TocError::NotFound(entry_id))?;

    let Some(current_parent_id) = entry.parent_id else {
        return Err(TocError::Validation(
            "Cannot outdent: entry is already at the root level".to_string(),
        ));
    };

    let parent =
        TocRepo::get_by_id(&tx, current_parent_id)?.ok_or(TocError::NotFound(current_parent_id))?;

    let grandparent_id = parent.parent_id;

    // Shift subsequent siblings of parent to make room
    match grandparent_id {
        Some(gpid) => {
            tx.execute(
                "UPDATE toc_entries SET order_index = order_index + 1 \
                 WHERE paper_id = ?1 AND parent_id = ?2 AND order_index > ?3",
                rusqlite::params![
                    entry.paper_id.to_string(),
                    gpid.to_string(),
                    parent.order_index
                ],
            )?;
        }
        None => {
            tx.execute(
                "UPDATE toc_entries SET order_index = order_index + 1 \
                 WHERE paper_id = ?1 AND parent_id IS NULL AND order_index > ?2",
                rusqlite::params![entry.paper_id.to_string(), parent.order_index],
            )?;
        }
    }

    let now = current_timestamp_utc();
    entry.parent_id = grandparent_id;
    entry.order_index = parent.order_index + 1;
    entry.updated_at = now.clone();

    TocRepo::update(&tx, &entry)?;

    if let Some(mut paper) = PaperRepo::get_by_id(&tx, entry.paper_id)? {
        paper.updated_at = now;
        PaperRepo::update(&tx, &paper)?;
    }

    tx.commit()?;

    Ok(entry)
}

/// Swaps order_index with previous sibling.
pub fn move_up(conn: &mut Connection, entry_id: Uuid) -> Result<TocEntry, TocError> {
    let tx = conn.transaction()?;

    let mut entry = TocRepo::get_by_id(&tx, entry_id)?.ok_or(TocError::NotFound(entry_id))?;

    let prev: Option<(String, i32)> = match entry.parent_id {
        Some(pid) => {
            let mut stmt = tx.prepare(
                "SELECT id, order_index FROM toc_entries \
                 WHERE paper_id = ?1 AND parent_id = ?2 AND order_index < ?3 \
                 ORDER BY order_index DESC LIMIT 1",
            )?;
            stmt.query_row(
                rusqlite::params![
                    entry.paper_id.to_string(),
                    pid.to_string(),
                    entry.order_index
                ],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok()
        }
        None => {
            let mut stmt = tx.prepare(
                "SELECT id, order_index FROM toc_entries \
                 WHERE paper_id = ?1 AND parent_id IS NULL AND order_index < ?2 \
                 ORDER BY order_index DESC LIMIT 1",
            )?;
            stmt.query_row(
                rusqlite::params![entry.paper_id.to_string(), entry.order_index],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok()
        }
    };

    let Some((prev_id_str, prev_order)) = prev else {
        tx.commit()?;
        return Ok(entry); // Already at top of siblings
    };

    let prev_id = Uuid::parse_str(&prev_id_str).map_err(|e| TocError::Validation(e.to_string()))?;
    let mut prev_entry = TocRepo::get_by_id(&tx, prev_id)?.ok_or(TocError::NotFound(prev_id))?;

    let now = current_timestamp_utc();
    let current_order = entry.order_index;

    entry.order_index = prev_order;
    entry.updated_at = now.clone();

    prev_entry.order_index = current_order;
    prev_entry.updated_at = now.clone();

    TocRepo::update(&tx, &entry)?;
    TocRepo::update(&tx, &prev_entry)?;

    if let Some(mut paper) = PaperRepo::get_by_id(&tx, entry.paper_id)? {
        paper.updated_at = now;
        PaperRepo::update(&tx, &paper)?;
    }

    tx.commit()?;

    Ok(entry)
}

/// Swaps order_index with next sibling.
pub fn move_down(conn: &mut Connection, entry_id: Uuid) -> Result<TocEntry, TocError> {
    let tx = conn.transaction()?;

    let mut entry = TocRepo::get_by_id(&tx, entry_id)?.ok_or(TocError::NotFound(entry_id))?;

    let next: Option<(String, i32)> = match entry.parent_id {
        Some(pid) => {
            let mut stmt = tx.prepare(
                "SELECT id, order_index FROM toc_entries \
                 WHERE paper_id = ?1 AND parent_id = ?2 AND order_index > ?3 \
                 ORDER BY order_index ASC LIMIT 1",
            )?;
            stmt.query_row(
                rusqlite::params![
                    entry.paper_id.to_string(),
                    pid.to_string(),
                    entry.order_index
                ],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok()
        }
        None => {
            let mut stmt = tx.prepare(
                "SELECT id, order_index FROM toc_entries \
                 WHERE paper_id = ?1 AND parent_id IS NULL AND order_index > ?2 \
                 ORDER BY order_index ASC LIMIT 1",
            )?;
            stmt.query_row(
                rusqlite::params![entry.paper_id.to_string(), entry.order_index],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok()
        }
    };

    let Some((next_id_str, next_order)) = next else {
        tx.commit()?;
        return Ok(entry); // Already at bottom of siblings
    };

    let next_id = Uuid::parse_str(&next_id_str).map_err(|e| TocError::Validation(e.to_string()))?;
    let mut next_entry = TocRepo::get_by_id(&tx, next_id)?.ok_or(TocError::NotFound(next_id))?;

    let now = current_timestamp_utc();
    let current_order = entry.order_index;

    entry.order_index = next_order;
    entry.updated_at = now.clone();

    next_entry.order_index = current_order;
    next_entry.updated_at = now.clone();

    TocRepo::update(&tx, &entry)?;
    TocRepo::update(&tx, &next_entry)?;

    if let Some(mut paper) = PaperRepo::get_by_id(&tx, entry.paper_id)? {
        paper.updated_at = now;
        PaperRepo::update(&tx, &paper)?;
    }

    tx.commit()?;

    Ok(entry)
}
