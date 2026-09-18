use std::path::Path;
use std::time::Duration;

use rusqlite::backup::Backup;
use rusqlite::Connection;

use crate::export::error::ExportError;

/// Creates a consistent hot backup of an SQLite database using SQLite's Online Backup API.
pub fn create_hot_backup(
    src_conn: &Connection,
    dst_path: &Path,
    pages_per_step: i32,
) -> Result<(), ExportError> {
    if let Some(parent) = dst_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut dst_conn = Connection::open(dst_path)
        .map_err(|e| ExportError::Backup(format!("failed to open destination DB: {e}")))?;

    // Scope the backup instance so dst_conn can be queried afterwards
    {
        let backup = Backup::new(src_conn, &mut dst_conn)
            .map_err(|e| ExportError::Backup(format!("failed to initialize SQLite backup: {e}")))?;

        let step_size = if pages_per_step <= 0 {
            100
        } else {
            pages_per_step
        };
        backup
            .run_to_completion(step_size, Duration::from_millis(1), None)
            .map_err(|e| ExportError::Backup(format!("backup failed to complete: {e}")))?;
    }

    // Verify destination DB integrity
    let integrity: String = dst_conn
        .query_row("PRAGMA integrity_check;", [], |row| row.get(0))
        .map_err(|e| ExportError::Backup(format!("integrity check query failed: {e}")))?;

    if integrity != "ok" {
        return Err(ExportError::Backup(format!(
            "destination DB integrity check failed: {integrity}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_hot_backup_basic() {
        let src = Connection::open_in_memory().unwrap();
        src.execute(
            "CREATE TABLE test_table (id INTEGER PRIMARY KEY, val TEXT);",
            [],
        )
        .unwrap();
        src.execute("INSERT INTO test_table (val) VALUES ('hello');", [])
            .unwrap();

        let dir = tempdir().unwrap();
        let dst_path = dir.path().join("backup.db");

        create_hot_backup(&src, &dst_path, 100).expect("hot backup should succeed");

        assert!(dst_path.exists());
        let dst = Connection::open(&dst_path).unwrap();
        let val: String = dst
            .query_row("SELECT val FROM test_table WHERE id = 1;", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(val, "hello");
    }
}
