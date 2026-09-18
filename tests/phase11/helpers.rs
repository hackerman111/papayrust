use std::path::{Path, PathBuf};

use lopdf::{dictionary, Document, Stream};
use papyrus_core::config::{Config, ExportConfig};
use papyrus_core::db::open_database;
use sha2::{Digest, Sha256};

pub fn setup_acceptance_env(dir: &Path) -> (Config, PathBuf) {
    let db_path = dir.join("library.db");
    let export_dir = dir.join("backups");
    let annotated_dir = dir.join(".library").join("annotated");
    std::fs::create_dir_all(&export_dir).expect("create backups dir");
    std::fs::create_dir_all(&annotated_dir).expect("create annotated dir");

    let _conn = open_database(&db_path).expect("init db schema");

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
        "library_path = \"{}\"\ndatabase_path = \"{}\"\n[export]\ndirectory = \"{}\"\n",
        config.library_path.display(),
        config.database_path.display(),
        config.export.directory.display(),
    );
    std::fs::write(&config_path, toml).expect("write acceptance config.toml");

    (config, config_path)
}

pub fn create_multi_page_pdf(path: &Path, page_count: usize, text: &str) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica"
    });
    let mut kid_ids = Vec::new();
    for i in 1..=page_count {
        let content_id = doc.add_object(Stream::new(
            dictionary!(),
            format!("BT /F1 12 Tf 100 700 Td ({text} - Page {i}) Tj ET").into_bytes(),
        ));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        });
        kid_ids.push(page_id.into());
    }
    doc.set_object(
        pages_id,
        dictionary! {
            "Type" => "Pages",
            "Kids" => kid_ids,
            "Count" => page_count as i64,
        },
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    doc.save(path).expect("save multi-page pdf");
}

pub fn compute_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn cli(cfg: &str, args: &[&str]) {
    let mut full = vec!["papyrus", "--config", cfg];
    full.extend_from_slice(args);
    papyrus_cli::run_with_args(&full).expect("cli execution succeeds");
}
