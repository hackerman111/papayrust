use std::path::{Path, PathBuf};

use lopdf::{dictionary, Document, Object, Stream};
use papyrus_core::config::{Config, ExportConfig};
use papyrus_core::db::open_database;
use sha2::{Digest, Sha256};

pub fn setup_cli_test_env(dir: &Path) -> (Config, PathBuf) {
    let db_path = dir.join("library.db");
    let export_dir = dir.join("backups");
    std::fs::create_dir_all(&export_dir).expect("create backups dir");
    let _conn = open_database(&db_path).expect("init db schema");

    let config = Config {
        library_path: dir.to_path_buf(),
        database_path: db_path,
        export: ExportConfig {
            directory: export_dir,
            ..Default::default()
        },
        ..Default::default()
    };
    let config_path = dir.join("config.toml");
    let toml = format!(
        "library_path = \"{}\"\ndatabase_path = \"{}\"\n[export]\ndirectory = \"{}\"\n",
        config.library_path.display(),
        config.database_path.display(),
        config.export.directory.display(),
    );
    std::fs::write(&config_path, toml).expect("write test config.toml");
    (config, config_path)
}

pub fn create_test_pdf(path: &Path, text: &str) {
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
    doc.save(path).expect("save test pdf");
}

pub fn create_pdf_with_outline(path: &Path, title: &str) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
    });
    doc.set_object(
        pages_id,
        dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 },
    );
    let outlines_id = doc.new_object_id();
    let item_id = doc.add_object(dictionary! {
        "Title" => Object::string_literal(title), "Parent" => outlines_id,
        "Dest" => vec![page_id.into(), Object::Name(b"Fit".to_vec())],
    });
    doc.set_object(
        outlines_id,
        dictionary! { "Type" => "Outlines", "First" => item_id, "Last" => item_id, "Count" => 1 },
    );
    let catalog_id = doc.add_object(
        dictionary! { "Type" => "Catalog", "Pages" => pages_id, "Outlines" => outlines_id },
    );
    doc.trailer.set("Root", catalog_id);
    doc.save(path).expect("save pdf with outline");
}

pub fn compute_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
