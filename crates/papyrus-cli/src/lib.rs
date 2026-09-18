pub mod commands;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use papyrus_core::config::Config;

pub use commands::add::AddArgs;
pub use commands::export::ExportArgs;
pub use commands::toc::TocCommands;

#[derive(Parser, Debug)]
#[command(name = "papyrus", about = "Terminal research paper manager", version)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Path to SQLite database file (overrides config)
    #[arg(short, long, value_name = "DB_FILE")]
    pub database: Option<PathBuf>,

    /// Path to library directory (overrides config)
    #[arg(short, long, value_name = "DIR")]
    pub library: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show current configuration and database status
    Status,
    /// Launch the interactive terminal user interface
    Tui,
    /// Add a new paper to the library
    Add(AddArgs),
    /// Manage table of contents for papers
    Toc {
        #[command(subcommand)]
        cmd: TocCommands,
    },
    /// Export the library or a specific collection to a ZIP archive
    Export(ExportArgs),
    /// Check database and files health and integrity
    Doctor {
        /// Perform full database integrity check (PRAGMA integrity_check)
        #[arg(long)]
        full: bool,
    },
}

/// Runs the CLI application parsing arguments from std::env::args_os.
pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    run_cli(cli)
}

/// Runs the CLI application using explicit argument slices (for testing or embedding).
pub fn run_with_args(args: &[&str]) -> anyhow::Result<()> {
    let cli = Cli::try_parse_from(args)?;
    run_cli(cli)
}

fn run_cli(cli: Cli) -> anyhow::Result<()> {
    let mut config = Config::load_discovered_or_default(cli.config.as_deref())?;

    let env_db = std::env::var_os("PAPYRUS_DATABASE").map(PathBuf::from);
    let env_lib = std::env::var_os("PAPYRUS_LIBRARY").map(PathBuf::from);

    let db_override = cli.database.or(env_db);
    let lib_override = cli.library.or(env_lib);

    if let Some(db_path) = db_override {
        if lib_override.is_none() {
            if let Some(parent) = db_path.parent() {
                if !parent.as_os_str().is_empty() {
                    if parent.file_name().is_some_and(|n| n == ".library") {
                        if let Some(grandparent) = parent.parent() {
                            config.library_path = if grandparent.as_os_str().is_empty() {
                                PathBuf::from(".")
                            } else {
                                grandparent.to_path_buf()
                            };
                        } else {
                            config.library_path = parent.to_path_buf();
                        }
                    } else {
                        config.library_path = parent.to_path_buf();
                    }
                } else {
                    config.library_path = PathBuf::from(".");
                }
            }
        }
        config.database_path = db_path;
    }

    if let Some(lib_path) = lib_override {
        config.library_path = lib_path;
    }

    commands::dispatch(cli.command, config)
}
