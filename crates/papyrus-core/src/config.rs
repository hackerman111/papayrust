use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Configuration errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config TOML: {0}")]
    Parse(#[from] toml::de::Error),
}

/// Root application configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_library_path")]
    pub library_path: PathBuf,

    #[serde(default = "default_database_path")]
    pub database_path: PathBuf,

    #[serde(default)]
    pub pdf: PdfConfig,

    #[serde(default)]
    pub toc: TocConfig,

    #[serde(default)]
    pub export: ExportConfig,
}

/// PDF viewer and launch template configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfConfig {
    #[serde(default = "default_pdf_viewer")]
    pub viewer: String,

    #[serde(default = "default_page_open_template")]
    pub page_open_template: String,
}

/// Table of contents extraction and management configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TocConfig {
    #[serde(default = "default_prefer_annotated_copy")]
    pub prefer_annotated_copy: bool,

    #[serde(default = "default_annotated_dir")]
    pub annotated_dir: PathBuf,

    #[serde(default = "default_auto_extract_on_import")]
    pub auto_extract_on_import: bool,
}

/// Export archive and backup configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportConfig {
    #[serde(default = "default_export_directory")]
    pub directory: PathBuf,

    #[serde(default = "default_export_compression")]
    pub compression: String,
}

fn default_library_path() -> PathBuf {
    PathBuf::from("/home/user/papers")
}

fn default_database_path() -> PathBuf {
    PathBuf::from("/home/user/papers/.library/library.db")
}

fn default_pdf_viewer() -> String {
    "zathura".to_string()
}

fn default_page_open_template() -> String {
    "zathura --page={page} {path}".to_string()
}

fn default_prefer_annotated_copy() -> bool {
    true
}

fn default_annotated_dir() -> PathBuf {
    PathBuf::from(".library/annotated")
}

fn default_auto_extract_on_import() -> bool {
    true
}

fn default_export_directory() -> PathBuf {
    PathBuf::from("/home/user/backups")
}

fn default_export_compression() -> String {
    "deflate".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            library_path: default_library_path(),
            database_path: default_database_path(),
            pdf: PdfConfig::default(),
            toc: TocConfig::default(),
            export: ExportConfig::default(),
        }
    }
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            viewer: default_pdf_viewer(),
            page_open_template: default_page_open_template(),
        }
    }
}

impl Default for TocConfig {
    fn default() -> Self {
        Self {
            prefer_annotated_copy: default_prefer_annotated_copy(),
            annotated_dir: default_annotated_dir(),
            auto_extract_on_import: default_auto_extract_on_import(),
        }
    }
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            directory: default_export_directory(),
            compression: default_export_compression(),
        }
    }
}

impl Config {
    /// Deserializes a configuration from a TOML string with defaults applied for missing keys.
    pub fn from_toml_str(s: &str) -> Result<Self, ConfigError> {
        let config: Config = toml::from_str(s)?;
        Ok(config)
    }

    /// Loads a configuration from the given file path.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
            path: path.to_path_buf(),
            source: err,
        })?;
        Self::from_toml_str(&content)
    }

    /// Loads configuration from the given path if provided, or returns `Config::default()` if `None`.
    pub fn load_or_default(path: Option<&Path>) -> Result<Self, ConfigError> {
        match path {
            Some(p) => Self::load(p),
            None => Ok(Self::default()),
        }
    }

    /// Returns a default configuration dynamically resolved for the active user's environment.
    /// If `$HOME` is set, uses `$HOME/papers`, `$HOME/papers/.library/library.db`, and `$HOME/backups`.
    /// Otherwise falls back to `Config::default()`.
    pub fn default_for_user() -> Self {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let library_path = home.join("papers");
            let database_path = library_path.join(".library").join("library.db");
            let export_dir = home.join("backups");
            Self {
                library_path,
                database_path,
                export: ExportConfig {
                    directory: export_dir,
                    ..ExportConfig::default()
                },
                ..Self::default()
            }
        } else {
            Self::default()
        }
    }

    /// Searches for a configuration file in candidate directories:
    /// 1. Current working directory: `papyrus.toml`
    /// 2. Hidden in current directory: `.papyrus/config.toml`
    /// 3. `$XDG_CONFIG_HOME/papyrus/config.toml` (or `~/.config/papyrus/config.toml`)
    /// 4. `~/.papyrus.toml`
    /// 5. `~/.papyrus/config.toml`
    pub fn discover() -> Option<PathBuf> {
        Self::discover_from_dir(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// Internal search helper rooted at `base_dir` and respecting `$HOME`/`$XDG_CONFIG_HOME`.
    pub fn discover_from_dir(base_dir: &Path) -> Option<PathBuf> {
        let candidates = [
            base_dir.join("papyrus.toml"),
            base_dir.join(".papyrus").join("config.toml"),
        ];
        for candidate in &candidates {
            if candidate.is_file() {
                return Some(candidate.clone());
            }
        }

        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
            let candidate = xdg.join("papyrus").join("config.toml");
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let user_candidates = [
                home.join(".config").join("papyrus").join("config.toml"),
                home.join(".papyrus.toml"),
                home.join(".papyrus").join("config.toml"),
            ];
            for candidate in &user_candidates {
                if candidate.is_file() {
                    return Some(candidate.clone());
                }
            }
        }

        None
    }

    /// Loads configuration from `path` if explicitly provided.
    /// If `path` is `None`, attempts `discover()`.
    /// If no configuration is discovered, falls back to `default_for_user()`.
    pub fn load_discovered_or_default(path: Option<&Path>) -> Result<Self, ConfigError> {
        match path {
            Some(p) => Self::load(p),
            None => {
                if let Some(discovered) = Self::discover() {
                    Self::load(&discovered)
                } else {
                    Ok(Self::default_for_user())
                }
            }
        }
    }
}
