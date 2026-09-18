use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

use crate::db::models::{TocEntry, TocSource};
use crate::pdf::metadata::{decode_pdf_string, resolve_pdf_string};
use crate::time::current_timestamp_utc;

const MAX_DEPTH: usize = 16;

/// Errors that can occur during outline extraction.
#[derive(Debug, Error)]
pub enum OutlineError {
    #[error("failed to read outline: {0}")]
    Pdf(String),
}

/// Extracts table of contents entries from the `/Outlines` hierarchy in a `lopdf::Document`.
///
/// Returns an empty vector if the document does not contain an `/Outlines` dictionary
/// or if the outline tree has no items.
pub fn extract_outlines(doc: &Document, paper_id: Uuid) -> Result<Vec<TocEntry>, OutlineError> {
    let catalog = match doc.catalog() {
        Ok(c) => c,
        Err(_) => return Ok(Vec::new()),
    };

    let outlines_dict = match catalog.get(b"Outlines") {
        Ok(Object::Dictionary(dict)) => dict,
        Ok(Object::Reference(id)) => match doc.get_dictionary(*id) {
            Ok(dict) => dict,
            Err(_) => return Ok(Vec::new()),
        },
        _ => return Ok(Vec::new()),
    };

    let first_obj = match outlines_dict.get(b"First") {
        Ok(obj) => obj,
        Err(_) => return Ok(Vec::new()),
    };

    let pages = doc.get_pages();
    let page_count = pages.len() as u32;

    let mut page_id_to_num: HashMap<ObjectId, u32> = HashMap::with_capacity(pages.len());
    for (page_num, page_id) in pages {
        page_id_to_num.insert(page_id, page_num);
    }

    let mut named_dests = HashMap::new();
    collect_named_destinations(doc, catalog, &mut named_dests);

    let now = current_timestamp_utc();
    let mut entries = Vec::new();
    let mut visited = HashSet::new();

    let ctx = OutlineContext {
        doc,
        paper_id,
        page_id_to_num: &page_id_to_num,
        named_dests: &named_dests,
        page_count,
        now: &now,
    };

    traverse_outline_items(&ctx, first_obj, None, &mut visited, &mut entries);

    Ok(entries)
}

struct OutlineContext<'a> {
    doc: &'a Document,
    paper_id: Uuid,
    page_id_to_num: &'a HashMap<ObjectId, u32>,
    named_dests: &'a HashMap<Vec<u8>, Object>,
    page_count: u32,
    now: &'a str,
}

fn traverse_outline_items(
    ctx: &OutlineContext<'_>,
    first_obj: &Object,
    parent_id: Option<Uuid>,
    visited: &mut HashSet<ObjectId>,
    entries: &mut Vec<TocEntry>,
) {
    let mut curr = first_obj;
    let mut order_index = 0i32;

    loop {
        let item_dict = match curr {
            Object::Reference(id) => {
                if !visited.insert(*id) {
                    // Prevent infinite loops on malformed / cyclic outline structures
                    break;
                }
                match ctx.doc.get_dictionary(*id) {
                    Ok(d) => d,
                    Err(_) => break,
                }
            }
            Object::Dictionary(d) => d,
            _ => break,
        };

        // 1. Title
        let title = item_dict
            .get(b"Title")
            .ok()
            .and_then(|t| resolve_pdf_string(ctx.doc, t))
            .map(decode_pdf_string)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Untitled".to_string());

        // 2. Page number
        let page_number = resolve_page_number(
            ctx.doc,
            item_dict,
            ctx.page_id_to_num,
            ctx.named_dests,
            ctx.page_count,
        );

        // 3. Entry
        let entry_id = Uuid::now_v7();
        entries.push(TocEntry {
            id: entry_id,
            paper_id: ctx.paper_id,
            parent_id,
            title,
            page_number,
            order_index,
            source: TocSource::Auto,
            created_at: ctx.now.to_string(),
            updated_at: ctx.now.to_string(),
        });
        order_index += 1;

        // 4. Children (First child)
        if let Ok(child_first) = item_dict.get(b"First") {
            traverse_outline_items(ctx, child_first, Some(entry_id), visited, entries);
        }

        // 5. Next sibling
        match item_dict.get(b"Next") {
            Ok(next_obj) => curr = next_obj,
            Err(_) => break,
        }
    }
}

fn resolve_page_number(
    doc: &Document,
    item_dict: &Dictionary,
    page_id_to_num: &HashMap<ObjectId, u32>,
    named_dests: &HashMap<Vec<u8>, Object>,
    page_count: u32,
) -> u32 {
    let dest_obj: Option<&Object> = if let Ok(dest) = item_dict.get(b"Dest") {
        Some(dest)
    } else if let Ok(action) = item_dict.get(b"A") {
        let action_dict: Option<&Dictionary> = match action {
            Object::Dictionary(d) => Some(d),
            Object::Reference(id) => doc.get_dictionary(*id).ok(),
            _ => None,
        };
        action_dict.and_then(|ad| ad.get(b"D").ok())
    } else {
        None
    };

    let page_num = match dest_obj {
        Some(dest) => resolve_destination(doc, dest, page_id_to_num, named_dests, 0),
        None => None,
    };

    let p = page_num.unwrap_or(1);
    if page_count > 0 && p > page_count {
        p.clamp(1, page_count)
    } else if p < 1 {
        1
    } else {
        p
    }
}

fn resolve_destination(
    doc: &Document,
    dest: &Object,
    page_id_to_num: &HashMap<ObjectId, u32>,
    named_dests: &HashMap<Vec<u8>, Object>,
    depth: usize,
) -> Option<u32> {
    if depth > MAX_DEPTH {
        return None;
    }
    match dest {
        Object::Array(ref arr) => {
            if arr.is_empty() {
                return None;
            }
            match &arr[0] {
                Object::Reference(id) => page_id_to_num.get(id).copied(),
                Object::Integer(idx) => {
                    // 0-based page index
                    if *idx >= 0 {
                        Some((*idx as u32) + 1)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        Object::Reference(id) => {
            if let Some(page_num) = page_id_to_num.get(id) {
                return Some(*page_num);
            }
            if let Ok(obj) = doc.get_object(*id) {
                return resolve_destination(doc, obj, page_id_to_num, named_dests, depth + 1);
            }
            None
        }
        Object::String(ref bytes, _) | Object::Name(ref bytes) => {
            if let Some(dest_val) = named_dests.get(bytes) {
                return resolve_destination(doc, dest_val, page_id_to_num, named_dests, depth + 1);
            }
            None
        }
        Object::Dictionary(ref dict) => {
            if let Ok(d) = dict.get(b"D") {
                return resolve_destination(doc, d, page_id_to_num, named_dests, depth + 1);
            }
            None
        }
        _ => None,
    }
}

fn collect_named_destinations(
    doc: &Document,
    catalog: &Dictionary,
    named_dests: &mut HashMap<Vec<u8>, Object>,
) {
    // 1. Direct /Dests in catalog
    if let Ok(dests_obj) = catalog.get(b"Dests") {
        let dests_dict: Option<&Dictionary> = match dests_obj {
            Object::Dictionary(d) => Some(d),
            Object::Reference(id) => doc.get_dictionary(*id).ok(),
            _ => None,
        };
        if let Some(dict) = dests_dict {
            for (k, v) in dict.iter() {
                named_dests.insert(k.clone(), v.clone());
            }
        }
    }

    // 2. /Names -> /Dests name tree in catalog
    if let Ok(names_obj) = catalog.get(b"Names") {
        let names_dict: Option<&Dictionary> = match names_obj {
            Object::Dictionary(d) => Some(d),
            Object::Reference(id) => doc.get_dictionary(*id).ok(),
            _ => None,
        };
        if let Some(dict) = names_dict {
            if let Ok(dests_obj) = dict.get(b"Dests") {
                let mut visited = HashSet::new();
                let dests_dict: Option<&Dictionary> = match dests_obj {
                    Object::Dictionary(d) => Some(d),
                    Object::Reference(id) => {
                        visited.insert(*id);
                        doc.get_dictionary(*id).ok()
                    }
                    _ => None,
                };
                if let Some(dests_dict) = dests_dict {
                    traverse_name_tree(doc, dests_dict, named_dests, &mut visited, 0);
                }
            }
        }
    }
}

fn traverse_name_tree(
    doc: &Document,
    dict: &Dictionary,
    named_dests: &mut HashMap<Vec<u8>, Object>,
    visited: &mut HashSet<ObjectId>,
    depth: usize,
) {
    if depth > MAX_DEPTH {
        return;
    }

    if let Ok(kids) = dict.get(b"Kids") {
        if let Ok(arr) = kids.as_array() {
            for kid in arr {
                if let Ok(kid_id) = kid.as_reference() {
                    if visited.insert(kid_id) {
                        if let Ok(kid_dict) = doc.get_dictionary(kid_id) {
                            traverse_name_tree(doc, kid_dict, named_dests, visited, depth + 1);
                        }
                    }
                }
            }
        }
    }

    if let Ok(names) = dict.get(b"Names") {
        if let Ok(arr) = names.as_array() {
            let mut iter = arr.iter();
            while let (Some(key_obj), Some(val_obj)) = (iter.next(), iter.next()) {
                if let Some(key_bytes) = resolve_pdf_string(doc, key_obj) {
                    named_dests.insert(key_bytes.to_vec(), val_obj.clone());
                }
            }
        }
    }
}
