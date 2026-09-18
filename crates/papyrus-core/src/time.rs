use std::time::SystemTime;

/// Returns current UTC timestamp in ISO 8601 format: `YYYY-MM-DDTHH:MM:SSZ`.
///
/// Implemented via civil calendar arithmetic without external runtime dependencies.
pub fn current_timestamp_utc() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = now.as_secs();

    // Days since Jan 1 1970 and seconds within current day
    let days = (total_secs / 86400) as i64;
    let day_secs = (total_secs % 86400) as u32;
    let hours = day_secs / 3600;
    let mins = (day_secs % 3600) / 60;
    let secs = day_secs % 60;

    // Howard Hinnant's civil calendar algorithm (shift epoch to March 1 0000)
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{mins:02}:{secs:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_timestamp_utc_format() {
        let ts = current_timestamp_utc();
        assert_eq!(ts.len(), 20, "ISO 8601 UTC timestamp must be 20 chars");
        assert!(ts.ends_with('Z'), "Timestamp must end with Z");
        assert_eq!(&ts[10..11], "T", "Separator must be T");
    }
}
