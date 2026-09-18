use std::path::PathBuf;

use clap::Args;
use papyrus_core::config::Config;
use papyrus_core::db::{open_database, CollectionRepo};
use papyrus_core::export::{export_collection, export_library, ExportOptions};
use uuid::Uuid;

#[derive(Args, Debug, Clone)]
pub struct ExportArgs {
    /// Destination ZIP file path (optional)
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Collection to export (by name or UUID)
    #[arg(short, long, value_name = "COLLECTION")]
    pub collection: Option<String>,

    /// Overwrite existing destination file if it exists
    #[arg(short, long)]
    pub force: bool,

    /// Skip missing files and emit warnings instead of aborting
    #[arg(long, visible_alias = "skip-missing-files")]
    pub skip_missing: bool,
}

pub fn run_export(args: ExportArgs, config: &Config) -> anyhow::Result<()> {
    let conn = open_database(&config.database_path)?;
    let options = ExportOptions {
        output_path: args.output.unwrap_or_default(),
        overwrite: args.force,
        include_database: true,
        skip_missing_files: args.skip_missing,
    };

    let res = if let Some(ref col_ref) = args.collection {
        let col = if let Ok(id) = Uuid::parse_str(col_ref) {
            match CollectionRepo::get_by_id(&conn, id)? {
                Some(c) => Some(c),
                None => CollectionRepo::get_by_name(&conn, col_ref)?,
            }
        } else {
            CollectionRepo::get_by_name(&conn, col_ref)?
        }
        .ok_or_else(|| anyhow::anyhow!("Collection not found: '{col_ref}'"))?;

        let res = export_collection(&conn, config, col.id, &options)?;
        println!(
            "Successfully exported collection '{}' ({} papers) to {}",
            col.name,
            res.paper_count,
            res.archive_path.display()
        );
        res
    } else {
        let res = export_library(&conn, config, &options)?;
        println!(
            "Successfully exported library ({} papers, {} collections) to {}",
            res.paper_count,
            res.collection_count,
            res.archive_path.display()
        );
        res
    };

    for warning in &res.warnings {
        eprintln!("Warning: {warning}");
    }

    Ok(())
}
