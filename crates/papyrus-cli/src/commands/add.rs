use std::path::PathBuf;

use clap::Args;
use papyrus_core::config::Config;
use papyrus_core::db::{open_database, CollectionRepo};
use papyrus_core::importer::{import_paper, ImportError};
use papyrus_core::search::SearchIndex;
use uuid::Uuid;

#[derive(Args, Debug, Clone)]
pub struct AddArgs {
    /// Path to PDF file to import
    pub file: PathBuf,

    /// Collection to assign the paper to (by name or UUID)
    #[arg(short, long, value_name = "COLLECTION")]
    pub collection: Option<String>,

    /// Do not automatically extract table of contents from PDF outlines
    #[arg(long)]
    pub no_extract: bool,
}

pub fn run_add(args: AddArgs, config: &Config) -> anyhow::Result<()> {
    let mut conn = open_database(&config.database_path)?;

    let collection_id = match args.collection {
        Some(ref col_ref) => {
            let col = if let Ok(id) = Uuid::parse_str(col_ref) {
                match CollectionRepo::get_by_id(&conn, id)? {
                    Some(c) => Some(c),
                    None => CollectionRepo::get_by_name(&conn, col_ref)?,
                }
            } else {
                CollectionRepo::get_by_name(&conn, col_ref)?
            };
            let col = col.ok_or_else(|| anyhow::anyhow!("Collection not found: '{col_ref}'"))?;
            Some(col.id)
        }
        None => None,
    };

    let mut effective_config = config.clone();
    if args.no_extract {
        effective_config.toc.auto_extract_on_import = false;
    }

    let paper = match import_paper(&mut conn, &effective_config, &args.file, collection_id) {
        Ok(p) => p,
        Err(ImportError::Duplicate {
            existing_id,
            content_hash,
            ..
        }) => {
            anyhow::bail!(
                "Duplicate paper: already exists with ID {existing_id} (hash: {content_hash})"
            );
        }
        Err(err) => return Err(err.into()),
    };

    let search_dir = effective_config
        .library_path
        .join(".papyrus")
        .join("search");
    if let Ok(search_index) = SearchIndex::open_or_create(&search_dir) {
        let body_text = papyrus_core::pdf::extract_text(&args.file).ok();
        let _ = search_index.index_paper(&paper, body_text.as_deref());
    }

    println!(
        "Successfully added paper '{}' (ID: {})",
        paper.title.as_deref().unwrap_or("[Untitled]"),
        paper.id
    );
    Ok(())
}
