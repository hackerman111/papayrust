use papyrus_core::config::{Config, ConfigError, ExportConfig};
use papyrus_core::db::{apply_migrations, current_schema_version, open_database, open_in_memory};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

/// RT-01: Valid config parsing from TOML.
#[test]
fn test_rt_01_valid_config_parsing() {
    let toml_content = r#"
library_path = "/custom/papers"
database_path = "/custom/papers/custom.db"

[pdf]
viewer = "mupdf"
page_open_template = "mupdf {path} {page}"

[toc]
prefer_annotated_copy = false
annotated_dir = "custom/annotated"
auto_extract_on_import = false

[export]
directory = "/custom/backups"
compression = "none"
"#;

    let config = Config::from_toml_str(toml_content).expect("failed to parse valid TOML");
    assert_eq!(config.library_path, PathBuf::from("/custom/papers"));
    assert_eq!(
        config.database_path,
        PathBuf::from("/custom/papers/custom.db")
    );
    assert_eq!(config.pdf.viewer, "mupdf");
    assert_eq!(config.pdf.page_open_template, "mupdf {path} {page}");
    assert!(!config.toc.prefer_annotated_copy);
    assert_eq!(config.toc.annotated_dir, PathBuf::from("custom/annotated"));
    assert!(!config.toc.auto_extract_on_import);
    assert_eq!(config.export.directory, PathBuf::from("/custom/backups"));
    assert_eq!(config.export.compression, "none");

    // Also verify loading from a file on disk
    let dir = tempdir().expect("tempdir");
    let file_path = dir.path().join("config.toml");
    fs::write(&file_path, toml_content).expect("write config");

    let loaded = Config::load(&file_path).expect("failed to load config from file");
    assert_eq!(loaded, config);

    let loaded_or_default =
        Config::load_or_default(Some(&file_path)).expect("failed to load_or_default with path");
    assert_eq!(loaded_or_default, config);
}

/// RT-02: Missing config behavior.
#[test]
fn test_rt_02_missing_config_behavior() {
    // 1. None returns default config
    let default_cfg =
        Config::load_or_default(None).expect("load_or_default(None) should return default");
    assert_eq!(default_cfg, Config::default());

    // 2. Explicit non-existent file returns an Io error
    let missing_path = Path::new("/path/that/definitely/does/not/exist_rt02.toml");
    let err = Config::load(missing_path);
    assert!(err.is_err(), "Loading a non-existent file must fail");
    match err.unwrap_err() {
        ConfigError::Io { path, source } => {
            assert_eq!(path, missing_path);
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        _ => panic!("Expected ConfigError::Io"),
    }

    // 3. load_or_default with explicit non-existent file also returns an Io error
    let err_explicit = Config::load_or_default(Some(missing_path));
    assert!(
        err_explicit.is_err(),
        "load_or_default with missing explicit path must fail"
    );
}

/// RT-03: Defaults of [export] (compression == "deflate", directory, etc.)
#[test]
fn test_rt_03_export_defaults() {
    let default_export = ExportConfig::default();
    assert_eq!(default_export.compression, "deflate");
    assert_eq!(
        default_export.directory,
        PathBuf::from("/home/user/backups")
    );

    // Check defaults when omitting [export] in TOML
    let partial_toml = r#"
library_path = "/home/user/papers"
"#;
    let config = Config::from_toml_str(partial_toml).expect("parse partial toml");
    assert_eq!(config.export.compression, "deflate");
    assert_eq!(config.export.directory, PathBuf::from("/home/user/backups"));
    assert_eq!(config.pdf.viewer, "zathura");
    assert_eq!(
        config.pdf.page_open_template,
        "zathura --page={page} {path}"
    );
    assert!(config.toc.prefer_annotated_copy);
    assert_eq!(
        config.toc.annotated_dir,
        PathBuf::from(".library/annotated")
    );
    assert!(config.toc.auto_extract_on_import);
}

/// RT-03a: Smoke-test creates SQLite DB via rusqlite, checks foreign_keys == 1 and journal_mode.
#[test]
fn test_rt_03a_sqlite_smoke_test() {
    // 0. Verify unmigrated DB returns version 0
    let unmigrated_conn = Connection::open_in_memory().expect("open raw in-memory");
    assert_eq!(
        current_schema_version(&unmigrated_conn).expect("unmigrated schema version"),
        0,
        "Unmigrated database schema version must be 0"
    );

    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("test_library.db");

    // 1. File database verification
    let file_conn = open_database(&db_path).expect("open file database");

    let fk: i64 = file_conn
        .query_row("PRAGMA foreign_keys;", [], |r| r.get(0))
        .expect("query foreign_keys");
    assert_eq!(fk, 1, "PRAGMA foreign_keys must be enabled (1)");

    let sync: i64 = file_conn
        .query_row("PRAGMA synchronous;", [], |r| r.get(0))
        .expect("query synchronous");
    assert_eq!(sync, 1, "PRAGMA synchronous must be NORMAL (1)");

    let journal_mode: String = file_conn
        .query_row("PRAGMA journal_mode;", [], |r| r.get(0))
        .expect("query journal_mode");
    assert_eq!(
        journal_mode.to_lowercase(),
        "wal",
        "File DB journal_mode must be WAL"
    );

    let busy_timeout: i64 = file_conn
        .query_row("PRAGMA busy_timeout;", [], |r| r.get(0))
        .expect("query busy_timeout");
    assert_eq!(busy_timeout, 5000, "busy_timeout must be 5000ms");

    // Verify initial schema tables exist
    let papers_count: i64 = file_conn
        .query_row("SELECT COUNT(*) FROM papers;", [], |r| r.get(0))
        .expect("papers table query");
    assert_eq!(papers_count, 0);

    let collections_count: i64 = file_conn
        .query_row("SELECT COUNT(*) FROM collections;", [], |r| r.get(0))
        .expect("collections table query");
    assert_eq!(collections_count, 0);

    let paper_collections_count: i64 = file_conn
        .query_row("SELECT COUNT(*) FROM paper_collections;", [], |r| r.get(0))
        .expect("paper_collections table query");
    assert_eq!(paper_collections_count, 0);

    let toc_entries_count: i64 = file_conn
        .query_row("SELECT COUNT(*) FROM toc_entries;", [], |r| r.get(0))
        .expect("toc_entries table query");
    assert_eq!(toc_entries_count, 0);

    let migration_version: u32 = file_conn
        .query_row("SELECT MAX(version) FROM schema_migrations;", [], |r| {
            r.get(0)
        })
        .expect("schema_migrations table query");
    assert_eq!(migration_version, 1);
    assert_eq!(
        current_schema_version(&file_conn).expect("file database schema version"),
        1,
        "Migrated file DB schema version must be 1"
    );

    // 2. In-memory database verification
    let mem_conn = open_in_memory().expect("open in-memory database");

    let mem_fk: i64 = mem_conn
        .query_row("PRAGMA foreign_keys;", [], |r| r.get(0))
        .expect("query mem foreign_keys");
    assert_eq!(
        mem_fk, 1,
        "In-memory PRAGMA foreign_keys must be enabled (1)"
    );

    let mem_journal_mode: String = mem_conn
        .query_row("PRAGMA journal_mode;", [], |r| r.get(0))
        .expect("query mem journal_mode");
    assert_eq!(
        mem_journal_mode.to_lowercase(),
        "memory",
        "In-memory DB journal_mode must be memory"
    );
    assert_eq!(
        current_schema_version(&mem_conn).expect("mem database schema version"),
        1,
        "Migrated mem DB schema version must be 1"
    );

    // 3. Migration idempotency test
    let mut conn_to_remigrate = open_database(&db_path).expect("reopen database");
    apply_migrations(&mut conn_to_remigrate)
        .expect("re-applying migrations should succeed idempotently");
    assert_eq!(
        current_schema_version(&conn_to_remigrate).expect("remigrated schema version"),
        1
    );
}
