use papyrus_core::config::Config;

pub fn run_status(config: &Config) -> anyhow::Result<()> {
    println!("Configuration:");
    println!("  Library:  {}", config.library_path.display());
    println!("  Database: {}", config.database_path.display());
    println!("  TUI version: {}", papyrus_tui::version());
    Ok(())
}
