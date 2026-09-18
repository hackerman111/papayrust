use std::path::{Path, PathBuf};

/// Configuration options for library or collection export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportOptions {
    pub output_path: PathBuf,
    pub overwrite: bool,
    pub include_database: bool,
    pub skip_missing_files: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            output_path: PathBuf::new(),
            overwrite: false,
            include_database: true,
            skip_missing_files: false,
        }
    }
}

/// Statistics and metadata returned from a successful export operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportResult {
    pub archive_path: PathBuf,
    pub paper_count: usize,
    pub collection_count: usize,
    pub total_bytes: u64,
    pub warnings: Vec<String>,
}

/// RAII guard that removes a temporary file on drop if it still exists.
pub(crate) struct TempFileGuard<'a>(pub &'a Path);

impl<'a> Drop for TempFileGuard<'a> {
    fn drop(&mut self) {
        if self.0.exists() {
            let _ = std::fs::remove_file(self.0);
        }
    }
}
