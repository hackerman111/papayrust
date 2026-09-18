use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Represents a research paper stored in the database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paper {
    pub id: Uuid,
    pub file_path: String,
    pub content_hash: String,
    pub title: Option<String>,
    pub authors: Option<String>,
    pub year: Option<i64>,
    pub journal: Option<String>,
    pub doi: Option<String>,
    pub abstract_text: Option<String>,
    pub text_path: Option<String>,
    pub annotated_pdf_path: Option<String>,
    pub toc_embedded_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Represents a collection (folder/grouping) of papers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
}

/// Source origin of a table of contents entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TocSource {
    Auto,
    Manual,
    Imported,
}

impl TocSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
            Self::Imported => "imported",
        }
    }

    pub(crate) fn from_str_internal(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "manual" => Ok(Self::Manual),
            "imported" => Ok(Self::Imported),
            other => Err(format!("unknown toc source: '{other}'")),
        }
    }
}

impl fmt::Display for TocSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for TocSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_internal(s)
    }
}

/// Represents an entry in the table of contents of a paper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TocEntry {
    pub id: Uuid,
    pub paper_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub page_number: u32,
    pub order_index: i32,
    pub source: TocSource,
    pub created_at: String,
    pub updated_at: String,
}
