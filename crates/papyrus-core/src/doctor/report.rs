use std::fmt;

/// Overall verdict of the doctor health check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DoctorVerdict {
    Ok,
    Warnings,
    Failed,
}

impl fmt::Display for DoctorVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => write!(f, "OK"),
            Self::Warnings => write!(f, "WARNINGS"),
            Self::Failed => write!(f, "FAILED"),
        }
    }
}

/// Comprehensive report of health and integrity checks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DoctorReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl DoctorReport {
    /// Creates a new, empty `DoctorReport`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an error in the report.
    pub fn add_error(&mut self, err: impl Into<String>) {
        self.errors.push(err.into());
    }

    /// Records a warning in the report.
    pub fn add_warning(&mut self, warn: impl Into<String>) {
        self.warnings.push(warn.into());
    }

    /// Computes the overall verdict based on accumulated errors and warnings.
    pub fn verdict(&self) -> DoctorVerdict {
        if !self.errors.is_empty() {
            DoctorVerdict::Failed
        } else if !self.warnings.is_empty() {
            DoctorVerdict::Warnings
        } else {
            DoctorVerdict::Ok
        }
    }

    /// Returns `true` if there are no errors and no warnings.
    pub fn is_ok(&self) -> bool {
        self.verdict() == DoctorVerdict::Ok
    }

    /// Merges another report into this one.
    pub fn merge(&mut self, other: DoctorReport) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

impl fmt::Display for DoctorReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Verdict: {}", self.verdict())?;
        if !self.errors.is_empty() {
            writeln!(f, "\nErrors ({}):", self.errors.len())?;
            for err in &self.errors {
                writeln!(f, "  [ERROR] {err}")?;
            }
        }
        if !self.warnings.is_empty() {
            writeln!(f, "\nWarnings ({}):", self.warnings.len())?;
            for warn in &self.warnings {
                writeln!(f, "  [WARN] {warn}")?;
            }
        }
        if self.errors.is_empty() && self.warnings.is_empty() {
            writeln!(f, "All health checks passed.")?;
        }
        Ok(())
    }
}
