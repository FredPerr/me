//! The system clock as an ISO 8601 string, with no third-party date crate.
//! Converts the Unix timestamp to a UTC civil date with Howard Hinnant's
//! `days_from_civil` inverse (`civil_from_days`), the same algorithm the C++
//! `<chrono>` proposal uses, so the format matches the timestamps the frontend
//! wrote before (`YYYY-MM-DDTHH:MM:SSZ`).

use std::time::{SystemTime, UNIX_EPOCH};

use crate::workspace::domain::ports::Clock;

const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 60 * SECONDS_PER_MINUTE;
const SECONDS_PER_DAY: u64 = 24 * SECONDS_PER_HOUR;

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_iso8601(&self) -> String {
        let seconds_since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or(0);
        format_iso8601(seconds_since_epoch)
    }
}

/// Formats seconds since the Unix epoch as a UTC ISO 8601 timestamp.
fn format_iso8601(seconds_since_epoch: u64) -> String {
    let days_since_epoch = (seconds_since_epoch / SECONDS_PER_DAY) as i64;
    let second_of_day = seconds_since_epoch % SECONDS_PER_DAY;
    let (year, month, day) = civil_from_days(days_since_epoch);
    let hour = second_of_day / SECONDS_PER_HOUR;
    let minute = (second_of_day % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;
    let second = second_of_day % SECONDS_PER_MINUTE;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// The civil (year, month, day) for a count of days since 1970-01-01, by
/// Hinnant's `civil_from_days`. `month` is 1..=12 and `day` is 1..=31.
fn civil_from_days(days_since_epoch: i64) -> (i64, u64, u64) {
    let shifted = days_since_epoch + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = (shifted - era * 146_097) as u64; // [0, 146096]
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365; // [0, 399]
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100); // [0, 365]
    let internal_month = (5 * day_of_year + 2) / 153; // [0, 11], March = 0
    let day = day_of_year - (153 * internal_month + 2) / 5 + 1; // [1, 31]
    let month = if internal_month < 10 {
        internal_month + 3
    } else {
        internal_month - 9
    }; // [1, 12]
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_the_unix_epoch() {
        assert_eq!(format_iso8601(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn formats_a_known_timestamp() {
        // 2024-01-15T10:30:45Z
        assert_eq!(format_iso8601(1_705_314_645), "2024-01-15T10:30:45Z");
    }

    #[test]
    fn formats_a_leap_day() {
        // 2020-02-29T00:00:00Z
        assert_eq!(format_iso8601(1_582_934_400), "2020-02-29T00:00:00Z");
    }

    #[test]
    fn the_system_clock_produces_a_recent_year() {
        let now = SystemClock.now_iso8601();
        assert!(now.ends_with('Z'), "{now}");
        let year: i64 = now[..4].parse().unwrap();
        assert!(year >= 2024, "{now}");
    }
}
