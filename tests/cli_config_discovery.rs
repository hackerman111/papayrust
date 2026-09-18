use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

use papyrus_cli::run_with_args;
use papyrus_core::config::Config;
use papyrus_core::db::open_database;

#[test]
fn test_auto_discovery_in_dir() {
    let tmp = tempdir().unwrap();
    let config_path = tmp.path().join("papyrus.toml");
    fs::write(&config_path, "library_path = \"/custom/lib\"\n").unwrap();

    let discovered = Config::discover_from_dir(tmp.path());
    assert_eq!(discovered, Some(config_path));
}

#[test]
fn test_auto_discovery_hidden_dir() {
    let tmp = tempdir().unwrap();
    let hidden = tmp.path().join(".papyrus");
    fs::create_dir_all(&hidden).unwrap();
    let config_path = hidden.join("config.toml");
    fs::write(&config_path, "library_path = \"/hidden/lib\"\n").unwrap();

    let discovered = Config::discover_from_dir(tmp.path());
    assert_eq!(discovered, Some(config_path));
}

#[test]
fn test_default_for_user_uses_home() {
    let user_default = Config::default_for_user();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        assert_eq!(user_default.library_path, home.join("papers"));
        assert_eq!(
            user_default.database_path,
            home.join("papers").join(".library").join("library.db")
        );
        assert_eq!(user_default.export.directory, home.join("backups"));
    }
}

#[test]
fn test_cli_database_flag_overrides_db() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("custom_location").join("any.db");
    let db_str = db_path.to_str().unwrap();

    // Running status with -d creates or accepts the arbitrary database path
    let result = run_with_args(&["papyrus", "-d", db_str, "status"]);
    assert!(
        result.is_ok(),
        "status with -d flag should succeed: {:?}",
        result.err()
    );
}

#[test]
fn test_cli_database_and_library_flags() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("custom_db.sqlite");
    let lib_path = tmp.path().join("my_library_dir");
    let db_str = db_path.to_str().unwrap();
    let lib_str = lib_path.to_str().unwrap();

    let result = run_with_args(&["papyrus", "-d", db_str, "-l", lib_str, "status"]);
    assert!(
        result.is_ok(),
        "status with -d and -l should succeed: {:?}",
        result.err()
    );
}

#[test]
fn test_open_database_creates_nested_directories() {
    let tmp = tempdir().unwrap();
    let nested_db = tmp
        .path()
        .join("deep")
        .join("nested")
        .join("folder")
        .join("test.db");
    assert!(!nested_db.parent().unwrap().exists());

    let conn = open_database(&nested_db);
    assert!(
        conn.is_ok(),
        "open_database should succeed: {:?}",
        conn.err()
    );
    assert!(nested_db.exists(), "database file should exist on disk");
}
