use std::path::{Path, PathBuf};

use clap::Subcommand;
use papyrus_core::config::Config;
use papyrus_core::db::{open_database, PaperRepo, TocRepo};
use papyrus_core::toc::{embed_toc_in_pdf, export_to_json, import_toc};
use papyrus_core::TocImportSource;
use uuid::Uuid;

#[derive(Subcommand, Debug, Clone)]
pub enum TocCommands {
    /// Import table of contents for a paper
    Import {
        paper_id: Uuid,
        #[arg(long)]
        from_pdf: bool,
        #[arg(long, value_name = "PATH")]
        from_text: Option<PathBuf>,
        #[arg(long, value_name = "PATH")]
        from_json: Option<PathBuf>,
        #[arg(long)]
        merge: bool,
    },
    /// Export table of contents of a paper to JSON file
    Export {
        paper_id: Uuid,
        #[arg(long, value_name = "PATH")]
        to: PathBuf,
    },
    /// Materialize table of contents into an annotated PDF copy
    Embed { paper_id: Uuid },
}

pub fn run_toc(cmd: TocCommands, config: &Config) -> anyhow::Result<()> {
    let mut conn = open_database(&config.database_path)?;
    match cmd {
        TocCommands::Import {
            paper_id,
            from_pdf,
            from_text,
            from_json,
            merge,
        } => {
            let paper = PaperRepo::get_by_id(&conn, paper_id)?
                .ok_or_else(|| anyhow::anyhow!("Paper not found with ID {paper_id}"))?;

            let source = match (from_pdf, from_text, from_json) {
                (true, None, None) => TocImportSource::PdfOutline,
                (false, Some(path), None) => TocImportSource::TextFile(path),
                (false, None, Some(path)) => TocImportSource::JsonFile(path),
                _ => anyhow::bail!(
                    "Must specify exactly one of --from-pdf, --from-text, or --from-json"
                ),
            };

            let imported = import_toc(
                &mut conn,
                paper.id,
                Path::new(&paper.file_path),
                &source,
                merge,
            )?;
            let mode = if merge {
                "merged"
            } else {
                "imported (replaced)"
            };
            println!(
                "Successfully {mode} {} TOC entries for paper '{}'",
                imported.len(),
                paper.title.as_deref().unwrap_or("[Untitled]")
            );
        }
        TocCommands::Export { paper_id, to } => {
            let paper = PaperRepo::get_by_id(&conn, paper_id)?
                .ok_or_else(|| anyhow::anyhow!("Paper not found with ID {paper_id}"))?;
            let entries = TocRepo::get_by_paper(&conn, paper.id)?;
            std::fs::write(&to, export_to_json(&entries)?)?;
            println!(
                "Successfully exported {} TOC entries for paper '{}' to {}",
                entries.len(),
                paper.title.as_deref().unwrap_or("[Untitled]"),
                to.display()
            );
        }
        TocCommands::Embed { paper_id } => {
            let paper = PaperRepo::get_by_id(&conn, paper_id)?
                .ok_or_else(|| anyhow::anyhow!("Paper not found with ID {paper_id}"))?;
            let path = embed_toc_in_pdf(&mut conn, config, paper.id)?;
            println!(
                "Successfully embedded TOC into annotated copy: {}",
                path.display()
            );
        }
    }
    Ok(())
}
