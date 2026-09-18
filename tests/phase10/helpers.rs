use std::path::{Path, PathBuf};

use lopdf::{dictionary, Document, Stream};
use papyrus_core::config::{Config, ExportConfig};
use papyrus_core::db::open_database;
use rusqlite::Connection;
use sha2::{Digest, Sha256};

pub fn setup_doctor_test_env(dir: &Path) -> (Config, Connection) {
    let db_path = dir.join("library.db");
    let export_dir = dir.join("backups");
    let annotated_dir = dir.join(".library").join("annotated");
    std::fs::create_dir_all(&export_dir).expect("create backups dir");
    std::fs::create_dir_all(&annotated_dir).expect("create annotated dir");

    let conn = open_database(&db_path).expect("init db schema");

    let mut config = Config {
        library_path: dir.to_path_buf(),
        database_path: db_path.clone(),
        export: ExportConfig {
            directory: export_dir,
            ..Default::default()
        },
        ..Default::default()
    };
    config.toc.annotated_dir = annotated_dir;

    let config_path = dir.join("config.toml");
    let toml = format!(
        "library_path = \"{}\"\ndatabase_path = \"{}\"\n",
        config.library_path.display(),
        config.database_path.display(),
    );
    std::fs::write(&config_path, toml).expect("write test config.toml");

    (config, conn)
}

pub fn create_valid_pdf_with_text(path: &Path, text: &str) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica"
    });
    let content_id = doc.add_object(Stream::new(
        dictionary!(),
        format!("BT /F1 12 Tf 100 700 Td ({text}) Tj ET").into_bytes(),
    ));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages_id, "Contents" => content_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
    });
    doc.set_object(
        pages_id,
        dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 },
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    doc.save(path).expect("save valid pdf");
}

pub fn create_corrupt_pdf(path: &Path) {
    std::fs::write(path, b"NOT_A_VALID_PDF_HEADER_99999\xff\xfe\x00\x01corrupt")
        .expect("write corrupt pdf");
}

pub fn create_corrupt_sqlite(path: &Path) {
    let conn = open_database(path).expect("create valid sqlite");
    conn.execute(
        "CREATE TABLE test_data (id INTEGER PRIMARY KEY, content TEXT)",
        [],
    )
    .expect("create test table");
    for i in 0..50 {
        conn.execute(
            "INSERT INTO test_data (content) VALUES (?)",
            [format!("row data {i}")],
        )
        .expect("insert data");
    }
    drop(conn);

    let mut bytes = std::fs::read(path).expect("read sqlite bytes");
    // Corrupt B-Tree pages by zeroing/overwriting pages after header
    if bytes.len() > 200 {
        for b in &mut bytes[100..200] {
            *b = 0xAA;
        }
    }
    std::fs::write(path, bytes).expect("write corrupted sqlite bytes");
}

pub fn create_corrupt_tantivy_index(dir: &Path) -> PathBuf {
    let search_dir = dir.join(".papyrus").join("search");
    std::fs::create_dir_all(&search_dir).expect("create search dir");
    std::fs::write(search_dir.join("meta.json"), b"{ invalid_json: 1234 }")
        .expect("write corrupt meta.json");
    search_dir
}

#[allow(dead_code)]
pub fn compute_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
