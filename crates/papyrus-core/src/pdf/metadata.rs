use lopdf::{Document, Object, StringFormat};
use std::path::Path;

/// Extracted metadata from a PDF document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfMetadata {
    pub title: String,
    pub authors: Option<String>,
    pub year: Option<i64>,
    pub doi: Option<String>,
    pub journal: Option<String>,
    pub page_count: u32,
}

/// Decodes raw PDF bytes into a Rust UTF-8 String, handling:
/// - UTF-16BE with BOM (`\xFE\xFF`)
/// - UTF-16LE with BOM (`\xFF\xFE`)
/// - UTF-8 with BOM (`\xEF\xBB\xBF`)
/// - Standard UTF-8 without BOM
/// - PDFDocEncoding (PDF 1.7 / ISO 32000-1) via `lopdf::decode_text_string`
pub fn decode_pdf_string(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    // UTF-16BE BOM
    if bytes.starts_with(b"\xFE\xFF") {
        let u16_units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&u16_units);
    }

    // UTF-16LE BOM
    if bytes.starts_with(b"\xFF\xFE") {
        let u16_units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&u16_units);
    }

    // UTF-8 BOM
    if bytes.starts_with(b"\xEF\xBB\xBF") {
        return String::from_utf8_lossy(&bytes[3..]).to_string();
    }

    // Check if input is valid UTF-8
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }

    // Fallback to PDFDocEncoding via lopdf
    let obj = Object::String(bytes.to_vec(), StringFormat::Literal);
    if let Ok(s) = lopdf::decode_text_string(&obj) {
        return s;
    }

    // Ultimate fallback: lossy UTF-8
    String::from_utf8_lossy(bytes).to_string()
}

/// Resolves a byte slice from an `Object::String` or `Object::Name`,
/// dereferencing indirect `Object::Reference`s if necessary.
pub fn resolve_pdf_string<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a [u8]> {
    let mut curr = obj;
    for _ in 0..8 {
        match curr {
            Object::String(bytes, _) | Object::Name(bytes) => return Some(bytes.as_slice()),
            Object::Reference(id) => match doc.get_object(*id) {
                Ok(resolved) => curr = resolved,
                Err(_) => return None,
            },
            _ => return None,
        }
    }
    None
}

/// Extracts a 4-digit calendar year (1000..=9999) from PDF date strings
/// such as `D:20240315120000Z`, `2023-05-01`, or `2022`.
pub fn parse_pdf_date_year(date_str: &str) -> Option<i64> {
    let trimmed = date_str.trim();
    let s = trimmed
        .strip_prefix("D:")
        .or_else(|| trimmed.strip_prefix("d:"))
        .unwrap_or(trimmed);

    for window in s.as_bytes().windows(4) {
        if window.iter().all(u8::is_ascii_digit) {
            let year = (window[0] - b'0') as i64 * 1000
                + (window[1] - b'0') as i64 * 100
                + (window[2] - b'0') as i64 * 10
                + (window[3] - b'0') as i64;
            if (1000..=9999).contains(&year) {
                return Some(year);
            }
        }
    }

    None
}

/// Extracts metadata from a parsed `lopdf::Document`.
///
/// If `Title` is absent or whitespace-only, falls back to the file stem of `file_path`.
/// If `CreationDate` / `ModDate` is missing or unparseable, `year` is `None`.
pub fn extract_metadata(doc: &Document, file_path: &Path) -> PdfMetadata {
    let page_count = doc.get_pages().len() as u32;

    let file_stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("Untitled")
        .to_string();

    let info_dict = doc.trailer.get(b"Info").ok().and_then(|obj| match obj {
        Object::Dictionary(dict) => Some(dict),
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        _ => None,
    });

    let mut title = None;
    let mut authors = None;
    let mut year = None;
    let mut doi = None;
    let mut journal = None;

    if let Some(info) = info_dict {
        // Title
        if let Ok(title_obj) = info.get(b"Title") {
            if let Some(bytes) = resolve_pdf_string(doc, title_obj) {
                let decoded = decode_pdf_string(bytes);
                let trimmed = decoded.trim().to_string();
                if !trimmed.is_empty() {
                    title = Some(trimmed);
                }
            }
        }

        // Author / Authors
        let author_obj = info.get(b"Author").or_else(|_| info.get(b"Authors"));
        if let Ok(obj) = author_obj {
            if let Some(bytes) = resolve_pdf_string(doc, obj) {
                let decoded = decode_pdf_string(bytes);
                let trimmed = decoded.trim().to_string();
                if !trimmed.is_empty() {
                    authors = Some(trimmed);
                }
            }
        }

        // CreationDate / ModDate -> year
        let date_obj = info.get(b"CreationDate").or_else(|_| info.get(b"ModDate"));
        if let Ok(obj) = date_obj {
            if let Some(bytes) = resolve_pdf_string(doc, obj) {
                let date_str = decode_pdf_string(bytes);
                year = parse_pdf_date_year(&date_str);
            }
        }

        // DOI
        let doi_obj = info
            .get(b"DOI")
            .or_else(|_| info.get(b"doi"))
            .or_else(|_| info.get(b"Doi"));
        if let Ok(obj) = doi_obj {
            if let Some(bytes) = resolve_pdf_string(doc, obj) {
                let decoded = decode_pdf_string(bytes);
                let trimmed = decoded.trim().to_string();
                if !trimmed.is_empty() {
                    doi = Some(trimmed);
                }
            }
        }

        // Journal
        let journal_obj = info.get(b"Journal").or_else(|_| info.get(b"journal"));
        if let Ok(obj) = journal_obj {
            if let Some(bytes) = resolve_pdf_string(doc, obj) {
                let decoded = decode_pdf_string(bytes);
                let trimmed = decoded.trim().to_string();
                if !trimmed.is_empty() {
                    journal = Some(trimmed);
                }
            }
        }
    }

    let title = title.unwrap_or(file_stem);

    PdfMetadata {
        title,
        authors,
        year,
        doi,
        journal,
        page_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;

    #[test]
    fn test_decode_pdf_string_ascii() {
        let decoded = decode_pdf_string(b"Attention Is All You Need");
        assert_eq!(decoded, "Attention Is All You Need");
    }

    #[test]
    fn test_decode_pdf_string_utf16be() {
        // "Test" encoded in UTF-16BE with BOM \xFE\xFF
        let bytes = vec![0xFE, 0xFF, 0x00, 0x54, 0x00, 0x65, 0x00, 0x73, 0x00, 0x74];
        let decoded = decode_pdf_string(&bytes);
        assert_eq!(decoded, "Test");
    }

    #[test]
    fn test_decode_pdf_string_utf8_bom() {
        // "Test" with UTF-8 BOM \xEF\xBB\xBF
        let bytes = vec![0xEF, 0xBB, 0xBF, b'T', b'e', b's', b't'];
        let decoded = decode_pdf_string(&bytes);
        assert_eq!(decoded, "Test");
    }

    #[test]
    fn test_decode_pdf_string_utf8_without_bom() {
        let bytes = "Оглавление статьи".as_bytes();
        let decoded = decode_pdf_string(bytes);
        assert_eq!(decoded, "Оглавление статьи");
    }

    #[test]
    fn test_parse_pdf_date_year() {
        assert_eq!(parse_pdf_date_year("D:20240115123000Z"), Some(2024));
        assert_eq!(parse_pdf_date_year("D:19991231"), Some(1999));
        assert_eq!(parse_pdf_date_year("2023-04-10"), Some(2023));
        assert_eq!(parse_pdf_date_year("2021"), Some(2021));
        assert_eq!(parse_pdf_date_year(""), None);
        assert_eq!(parse_pdf_date_year("invalid_date"), None);
    }

    #[test]
    fn test_extract_metadata_fallback_to_file_stem() {
        let doc = Document::new();
        let path = Path::new("/path/to/my_research_paper.pdf");
        let meta = extract_metadata(&doc, path);

        assert_eq!(meta.title, "my_research_paper");
        assert_eq!(meta.authors, None);
        assert_eq!(meta.year, None);
        assert_eq!(meta.doi, None);
        assert_eq!(meta.journal, None);
        assert_eq!(meta.page_count, 0);
    }

    #[test]
    fn test_extract_metadata_with_info_dict() {
        let mut doc = Document::new();
        let info = dictionary! {
            "Title" => Object::string_literal("Deep Residual Learning"),
            "Author" => Object::string_literal("Kaiming He, Xiangyu Zhang"),
            "CreationDate" => Object::string_literal("D:20151210000000Z"),
            "DOI" => Object::string_literal("10.1109/CVPR.2016.90"),
            "Journal" => Object::string_literal("CVPR"),
        };
        let info_id = doc.add_object(info);
        doc.trailer.set("Info", info_id);

        let path = Path::new("/papers/resnet.pdf");
        let meta = extract_metadata(&doc, path);

        assert_eq!(meta.title, "Deep Residual Learning");
        assert_eq!(meta.authors, Some("Kaiming He, Xiangyu Zhang".to_string()));
        assert_eq!(meta.year, Some(2015));
        assert_eq!(meta.doi, Some("10.1109/CVPR.2016.90".to_string()));
        assert_eq!(meta.journal, Some("CVPR".to_string()));
    }

    #[test]
    fn test_decode_pdf_string_pdfdocencoding() {
        // PDFDocEncoding: 0xA9 is copyright sign ©, 0xE9 is é
        let bytes = b"Copyright \xA9 2024 Poincar\xE9";
        let decoded = decode_pdf_string(bytes);
        assert_eq!(decoded, "Copyright © 2024 Poincaré");
    }

    #[test]
    fn test_resolve_pdf_string_direct_and_indirect() {
        let mut doc = Document::new();
        let str_id = doc.add_object(Object::string_literal("Indirect Title"));
        let ref_obj = Object::Reference(str_id);
        let direct_obj = Object::string_literal("Direct Title");

        assert_eq!(
            resolve_pdf_string(&doc, &direct_obj),
            Some(b"Direct Title".as_slice())
        );
        assert_eq!(
            resolve_pdf_string(&doc, &ref_obj),
            Some(b"Indirect Title".as_slice())
        );
        assert_eq!(resolve_pdf_string(&doc, &Object::Integer(42)), None);
    }
}
