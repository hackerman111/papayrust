use std::path::Path;

use rusqlite::Connection;

use crate::config::Config;
use crate::doctor::report::DoctorReport;

fn file_exists_on_disk(file_str: &str, library_path: &Path) -> bool {
    let path = Path::new(file_str);
    if path.is_absolute() {
        path.exists()
    } else {
        path.exists() || library_path.join(path).exists()
    }
}

/// Checks the presence of paper files, annotated copies, text extractions, and Tantivy index.
pub fn check_files(conn: &Connection, config: &Config) -> DoctorReport {
    let mut report = DoctorReport::new();

    // 1. Check papers table for missing files
    let query_sql = "SELECT id, file_path, annotated_pdf_path, text_path FROM papers;";
    if let Ok(mut stmt) = conn.prepare(query_sql) {
        if let Ok(rows) = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let file_path: String = row.get(1)?;
            let annotated_path: Option<String> = row.get(2)?;
            let text_path: Option<String> = row.get(3)?;
            Ok((id, file_path, annotated_path, text_path))
        }) {
            for row in rows.flatten() {
                let (id, file_path, annotated_path, text_path) = row;

                // Primary PDF must exist -> error if missing
                if !file_exists_on_disk(&file_path, &config.library_path) {
                    report.add_error(format!(
                        "Paper {id}: original PDF file not found at '{file_path}'"
                    ));
                }

                // Dead annotated_pdf_path -> error if set in DB but not on disk
                if let Some(ref ann) = annotated_path {
                    if !file_exists_on_disk(ann, &config.library_path) {
                        report.add_error(format!(
                            "Paper {id}: dead annotated_pdf_path points to missing file '{ann}'"
                        ));
                    }
                }

                // Missing text_path -> warning if set in DB but not on disk
                if let Some(ref txt) = text_path {
                    if !file_exists_on_disk(txt, &config.library_path) {
                        report.add_warning(format!(
                            "Paper {id}: text_path points to missing file '{txt}'"
                        ));
                    }
                }
            }
        }
    }

    // 2. Check Tantivy search index if directory exists
    let search_dir = config.library_path.join(".papyrus").join("search");
    if search_dir.exists() {
        match tantivy::Index::open_in_dir(&search_dir) {
            Ok(index) => {
                if let Err(err) = index.reader_builder().try_into() {
                    report.add_error(format!(
                        "Search index at '{}' is corrupt: failed to create reader ({err})",
                        search_dir.display()
                    ));
                }
            }
            Err(err) => {
                report.add_error(format!(
                    "Search index at '{}' is corrupt: failed to open ({err})",
                    search_dir.display()
                ));
            }
        }
    }

    report
}
