pub mod add;
pub mod doctor;
pub mod export;
pub mod status;
pub mod toc;

use std::io::IsTerminal;

use papyrus_core::config::Config;

use crate::Commands;

pub fn dispatch(command: Option<Commands>, config: Config) -> anyhow::Result<()> {
    match command {
        Some(Commands::Status) => status::run_status(&config),
        Some(Commands::Tui) if std::io::stdout().is_terminal() => {
            Ok(papyrus_tui::run_tui(&config)?)
        }
        Some(Commands::Tui) => {
            anyhow::bail!("Cannot launch TUI in a non-interactive terminal (stdout is not a tty).")
        }
        Some(Commands::Add(args)) => add::run_add(args, &config),
        Some(Commands::Toc { cmd }) => toc::run_toc(cmd, &config),
        Some(Commands::Export(args)) => export::run_export(args, &config),
        Some(Commands::Doctor { full }) => doctor::run_doctor_cmd(full, &config),
        None if std::io::stdout().is_terminal() => Ok(papyrus_tui::run_tui(&config)?),
        None => {
            println!(
                "Papyrus initialized with library at: {}",
                config.library_path.display()
            );
            Ok(())
        }
    }
}
