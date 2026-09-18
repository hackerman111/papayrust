use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::{Component, Path};

use crate::db::models::Paper;
use crate::export::error::ExportError;

pub const CHUNK_SIZE: usize = 64 * 1024; // 64 KB

/// Validates that an entry path is safe for inclusion in a ZIP archive.
/// Rejects path traversal (`..`), leading slashes, and Windows drive prefixes.
pub fn validate_archive_entry_path(path: &str) -> Result<(), ExportError> {
    if path.is_empty() || path.contains('\0') {
        return Err(ExportError::InvalidPath(format!(
            "invalid empty or null-byte archive path: '{path}'"
        )));
    }

    let normalized = path.replace('\\', "/");
    if normalized.starts_with('/') {
        return Err(ExportError::PathTraversal(format!(
            "absolute archive path not allowed: '{path}'"
        )));
    }

    if normalized.len() >= 2
        && normalized.as_bytes()[1] == b':'
        && normalized.as_bytes()[0].is_ascii_alphabetic()
    {
        return Err(ExportError::PathTraversal(format!(
            "drive prefix detected: '{path}'"
        )));
    }

    for comp in Path::new(&normalized).components() {
        match comp {
            Component::ParentDir => {
                return Err(ExportError::PathTraversal(format!(
                    "path traversal '..' detected in archive path: '{path}'"
                )));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(ExportError::PathTraversal(format!(
                    "root or prefix detected in archive path: '{path}'"
                )));
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }

    Ok(())
}

/// Resolves filename collisions by appending `_1`, `_2`, etc. before the file extension.
pub fn resolve_collision(desired_path: &str, used_paths: &mut HashSet<String>) -> String {
    if !used_paths.contains(desired_path) {
        used_paths.insert(desired_path.to_string());
        return desired_path.to_string();
    }

    let p = Path::new(desired_path);
    let parent = p.parent().and_then(|d| d.to_str()).unwrap_or("");
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = p.extension().and_then(|e| e.to_str());

    let mut counter = 1usize;
    loop {
        let candidate = match (parent, ext) {
            ("", Some(e)) => format!("{stem}_{counter}.{e}"),
            ("", None) => format!("{stem}_{counter}"),
            (dir, Some(e)) => format!("{dir}/{stem}_{counter}.{e}"),
            (dir, None) => format!("{dir}/{stem}_{counter}"),
        };

        if !used_paths.contains(&candidate) {
            used_paths.insert(candidate.clone());
            return candidate;
        }
        counter += 1;
    }
}

/// Checks if an annotated PDF is fresh relative to the paper's last update.
pub fn is_annotated_fresh(paper: &Paper) -> bool {
    let Some(ref path_str) = paper.annotated_pdf_path else {
        return false;
    };
    if !Path::new(path_str).exists() {
        return false;
    }
    let Some(ref embedded_at) = paper.toc_embedded_at else {
        return false;
    };
    embedded_at.as_str() >= paper.updated_at.as_str()
}

/// Streams a file into the ZIP archive in 64 KB chunks with DEFLATE compression.
pub fn stream_file_to_zip<W: Write + Seek>(
    src_path: &Path,
    zip: &mut zip::ZipWriter<W>,
    entry_name: &str,
) -> Result<u64, ExportError> {
    validate_archive_entry_path(entry_name)?;

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file(entry_name, options)?;
    let mut file = File::open(src_path)?;
    let mut buffer = [0u8; CHUNK_SIZE];
    let mut total_bytes = 0u64;

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        zip.write_all(&buffer[..bytes_read])?;
        total_bytes += bytes_read as u64;
    }

    Ok(total_bytes)
}

/// Writes in-memory byte content into the ZIP archive with DEFLATE compression.
pub fn write_bytes_to_zip<W: Write + Seek>(
    bytes: &[u8],
    zip: &mut zip::ZipWriter<W>,
    entry_name: &str,
) -> Result<(), ExportError> {
    validate_archive_entry_path(entry_name)?;

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file(entry_name, options)?;
    zip.write_all(bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_traversal() {
        assert!(validate_archive_entry_path("../etc/passwd").is_err());
        assert!(validate_archive_entry_path("papers/../../secret").is_err());
        assert!(validate_archive_entry_path("/absolute/path").is_err());
        assert!(validate_archive_entry_path("papers/normal.pdf").is_ok());
        assert!(validate_archive_entry_path(".library/library.db").is_ok());
    }

    #[test]
    fn test_collision_resolution() {
        let mut used = HashSet::new();
        let path1 = resolve_collision("papers/doc.pdf", &mut used);
        assert_eq!(path1, "papers/doc.pdf");

        let path2 = resolve_collision("papers/doc.pdf", &mut used);
        assert_eq!(path2, "papers/doc_1.pdf");

        let path3 = resolve_collision("papers/doc.pdf", &mut used);
        assert_eq!(path3, "papers/doc_2.pdf");
    }
}
