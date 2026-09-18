use rusqlite::Connection;

use crate::config::Config;
use crate::doctor::database::check_database;
use crate::doctor::files::check_files;
use crate::doctor::report::DoctorReport;

/// Runs all health checks (database and filesystem) and returns an aggregated report.
pub fn run_doctor(conn: &Connection, config: &Config, full: bool) -> DoctorReport {
    let mut report = check_database(conn, full);
    let files_report = check_files(conn, config);
    report.merge(files_report);
    report
}
