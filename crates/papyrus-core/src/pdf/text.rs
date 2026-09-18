use std::path::Path;
use thiserror::Error;

/// Errors that can occur during PDF text extraction.
#[derive(Debug, Error)]
pub enum TextExtractionError {
    #[error("failed to extract text from PDF: {0}")]
    Extract(String),

    #[error("I/O error during text extraction: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(unix)]
struct SuppressOutputGuard {
    saved_stdout: libc::c_int,
    saved_stderr: libc::c_int,
}

#[cfg(unix)]
impl SuppressOutputGuard {
    fn new() -> Self {
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::os::unix::io::AsRawFd;

        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();

        // SAFETY:
        // We duplicate stdout and stderr descriptors to restore them when dropped.
        // We redirect STDOUT_FILENO and STDERR_FILENO to /dev/null to silence third-party library writes.
        // The saved file descriptors are owned by this guard and closed in drop().
        unsafe {
            let stdout_fd = libc::STDOUT_FILENO;
            let stderr_fd = libc::STDERR_FILENO;
            let saved_stdout = libc::dup(stdout_fd);
            let saved_stderr = libc::dup(stderr_fd);

            if let Ok(null_file) = OpenOptions::new().write(true).open("/dev/null") {
                let null_fd = null_file.as_raw_fd();
                if saved_stdout >= 0 {
                    libc::dup2(null_fd, stdout_fd);
                }
                if saved_stderr >= 0 {
                    libc::dup2(null_fd, stderr_fd);
                }
            }

            Self {
                saved_stdout,
                saved_stderr,
            }
        }
    }
}

#[cfg(unix)]
impl Drop for SuppressOutputGuard {
    fn drop(&mut self) {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();

        // SAFETY:
        // We restore the original stdout and stderr descriptors from saved backups, then close the backups.
        unsafe {
            let stdout_fd = libc::STDOUT_FILENO;
            let stderr_fd = libc::STDERR_FILENO;

            if self.saved_stdout >= 0 {
                libc::dup2(self.saved_stdout, stdout_fd);
                libc::close(self.saved_stdout);
                self.saved_stdout = -1;
            }
            if self.saved_stderr >= 0 {
                libc::dup2(self.saved_stderr, stderr_fd);
                libc::close(self.saved_stderr);
                self.saved_stderr = -1;
            }
        }
    }
}

fn suppress_output<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    #[cfg(unix)]
    {
        let _guard = SuppressOutputGuard::new();
        f()
    }
    #[cfg(not(unix))]
    {
        f()
    }
}

/// Extracts full text from the PDF file at `path` using `pdf-extract`.
pub fn extract_text(path: &Path) -> Result<String, TextExtractionError> {
    suppress_output(|| {
        pdf_extract::extract_text(path).map_err(|e| TextExtractionError::Extract(e.to_string()))
    })
}

/// Extracts full text from in-memory PDF `bytes` using `pdf-extract`.
pub fn extract_text_from_mem(bytes: &[u8]) -> Result<String, TextExtractionError> {
    suppress_output(|| {
        pdf_extract::extract_text_from_mem(bytes)
            .map_err(|e| TextExtractionError::Extract(e.to_string()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suppress_output_does_not_panic() {
        let result = suppress_output(|| {
            println!("This should not leak to test stdout if suppressed");
            42
        });
        assert_eq!(result, 42);
    }
}
