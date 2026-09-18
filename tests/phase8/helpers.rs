use lopdf::{dictionary, Document, Stream};
use papyrus_core::config::Config;
use papyrus_core::db::open_database;
use papyrus_core::export::Manifest;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

/// Creates a synthetic valid PDF with specified text content.
pub fn create_test_pdf(path: &Path, text: &str) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let content_str = format!("BT /F1 12 Tf 100 700 Td ({text}) Tj ET");
    let content_id = doc.add_object(Stream::new(dictionary!(), content_str.into_bytes()));

    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        "Resources" => dictionary! {
            "Font" => dictionary! { "F1" => font_id },
        },
    });

    doc.set_object(
        pages_id,
        dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        },
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });

    doc.trailer.set("Root", catalog_id);
    doc.save(path).expect("failed to save synthetic PDF");
}

/// Sets up a test database with schema migrations in the given directory.
pub fn setup_test_library(dir: &Path) -> (Connection, Config) {
    let db_path = dir.join("library.db");
    let conn = open_database(&db_path).expect("failed to open test database");

    let export_dir = dir.join("backups");
    std::fs::create_dir_all(&export_dir).expect("create backups dir");

    let config = Config {
        library_path: dir.to_path_buf(),
        database_path: db_path,
        export: papyrus_core::config::ExportConfig {
            directory: export_dir,
            ..Default::default()
        },
        ..Default::default()
    };

    (conn, config)
}

/// Computes hex-encoded SHA-256 hash of bytes.
pub fn compute_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Reads an entry from a ZIP archive into a byte vector.
pub fn read_zip_entry(archive: &mut ZipArchive<File>, entry_name: &str) -> Vec<u8> {
    let mut file = archive
        .by_name(entry_name)
        .unwrap_or_else(|_| panic!("entry '{entry_name}' not found in archive"));
    let mut content = Vec::new();
    file.read_to_end(&mut content)
        .expect("failed to read entry content");
    content
}

/// Reads and parses `manifest.json` from a ZIP archive.
pub fn read_zip_manifest(archive: &mut ZipArchive<File>) -> Manifest {
    let bytes = read_zip_entry(archive, "manifest.json");
    serde_json::from_slice(&bytes).expect("failed to parse manifest.json")
}
