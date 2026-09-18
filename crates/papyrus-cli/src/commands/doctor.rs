use papyrus_core::config::Config;
use papyrus_core::doctor::{run_doctor, DoctorReport, DoctorVerdict};

/// Runs the doctor diagnostic checks and prints results to stdout.
pub fn run_doctor_cmd(full: bool, config: &Config) -> anyhow::Result<()> {
    let conn = match rusqlite::Connection::open(&config.database_path) {
        Ok(c) => c,
        Err(err) => {
            let mut report = DoctorReport::new();
            report.add_error(format!(
                "Failed to open database at '{}': {err}",
                config.database_path.display()
            ));
            print!("{report}");
            anyhow::bail!("Doctor check failed with 1 error(s)");
        }
    };
    let report = run_doctor(&conn, config, full);
    print!("{report}");

    if report.verdict() == DoctorVerdict::Failed {
        anyhow::bail!("Doctor check failed with {} error(s)", report.errors.len());
    }

    Ok(())
}
