//! PDF parsing, metadata extraction, outline extraction, and text extraction.

pub mod metadata;
pub mod outline;
pub mod text;

pub use metadata::{
    decode_pdf_string, extract_metadata, parse_pdf_date_year, resolve_pdf_string, PdfMetadata,
};
pub use outline::{extract_outlines, OutlineError};
pub use text::{extract_text, extract_text_from_mem, TextExtractionError};
