use rusqlite::Connection;

use crate::doctor::report::DoctorReport;

/// Checks database integrity, foreign keys, and TOC structure.
pub fn check_database(conn: &Connection, full: bool) -> DoctorReport {
    let mut report = DoctorReport::new();

    // 1. Run PRAGMA integrity check
    let pragma_sql = if full {
        "PRAGMA integrity_check;"
    } else {
        "PRAGMA quick_check;"
    };

    match conn.prepare(pragma_sql) {
        Ok(mut stmt) => match stmt.query_map([], |row| row.get::<_, String>(0)) {
            Ok(rows) => {
                for row_result in rows {
                    match row_result {
                        Ok(row) => {
                            if row.trim().to_lowercase() != "ok" {
                                report.add_error(format!("Integrity check failure: {row}"));
                            }
                        }
                        Err(err) => {
                            report.add_error(format!("Integrity check failure: {err}"));
                        }
                    }
                }
            }
            Err(err) => {
                report.add_error(format!("Failed to execute {pragma_sql}: {err}"));
            }
        },
        Err(err) => {
            report.add_error(format!("Failed to prepare {pragma_sql}: {err}"));
        }
    }

    // 2. Run PRAGMA foreign_key_check
    match conn.prepare("PRAGMA foreign_key_check;") {
        Ok(mut stmt) => {
            let fk_rows = stmt.query_map([], |row| {
                let table: String = row.get(0)?;
                let rowid: i64 = row.get(1)?;
                let parent: String = row.get(2)?;
                let fkid: i64 = row.get(3)?;
                Ok((table, rowid, parent, fkid))
            });
            if let Ok(rows) = fk_rows {
                for row in rows.flatten() {
                    report.add_error(format!(
                        "Foreign key violation in table '{}' (rowid: {}), parent '{}', fk id {}",
                        row.0, row.1, row.2, row.3
                    ));
                }
            }
        }
        Err(err) => {
            report.add_error(format!("Failed to run foreign_key_check: {err}"));
        }
    }

    // 3. Verify TOC hierarchy consistency
    let toc_sql = "
        SELECT c.id, c.paper_id, p.id, p.paper_id
        FROM toc_entries c
        JOIN toc_entries p ON c.parent_id = p.id
        WHERE c.paper_id != p.paper_id;
    ";
    if let Ok(mut stmt) = conn.prepare(toc_sql) {
        if let Ok(rows) = stmt.query_map([], |row| {
            let child_id: String = row.get(0)?;
            let child_paper: String = row.get(1)?;
            let parent_id: String = row.get(2)?;
            let parent_paper: String = row.get(3)?;
            Ok((child_id, child_paper, parent_id, parent_paper))
        }) {
            for row in rows.flatten() {
                report.add_error(format!(
                    "TOC hierarchy mismatch: child {} (paper {}) != parent {} (paper {})",
                    row.0, row.1, row.2, row.3
                ));
            }
        }
    }

    report
}
